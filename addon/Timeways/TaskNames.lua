-- Marks the names of players in a text for the story program, which gives each player an
-- ID before a model reads the text (GAMEPLAY.md 4.7, 4.8, 5.11). A name matches as a whole
-- word, in any case, with or without its realm: "élise", "ÉLISE", and "Élise-Stormrage"
-- are all Élise.

local _, ns = ...

local TaskNames = {}
ns.TaskNames = TaskNames

-- One character of UTF-8: an ASCII byte, or a lead byte and its continuation bytes.
local CHAR = "[%z\1-\127\194-\244][\128-\191]*"

-- Past ASCII, a code point is a letter only in one of these ranges. Every other code point
-- is a sign: a wrong split hides too much, never too little. The desktop holds the same
-- table (`aliases::is_word_char`), and the addon test
-- `the_addon_and_the_desktop_agree_on_every_letter` checks every code point of the BMP.
local LETTERS = {
	{ 0x00AA, 0x00AA }, -- ª
	{ 0x00B5, 0x00B5 }, -- µ
	{ 0x00BA, 0x00BA }, -- º
	{ 0x00C0, 0x00D6 }, -- Latin-1 letters, less × and ÷
	{ 0x00D8, 0x00F6 },
	{ 0x00F8, 0x00FF },
	{ 0x0100, 0x024F }, -- Latin Extended-A and B
	{ 0x0370, 0x0374 }, -- Greek
	{ 0x0376, 0x0377 },
	{ 0x037A, 0x037D },
	{ 0x037F, 0x037F },
	{ 0x0386, 0x0386 },
	{ 0x0388, 0x038A },
	{ 0x038C, 0x038C },
	{ 0x038E, 0x03A1 },
	{ 0x03A3, 0x03F5 },
	{ 0x03F7, 0x03FF },
	{ 0x0400, 0x0481 }, -- Cyrillic
	{ 0x048A, 0x04FF },
	{ 0x1100, 0x115E }, -- Hangul Jamo, less the fillers
	{ 0x1161, 0x11FF },
	{ 0x1E00, 0x1EFF }, -- Latin Extended Additional
	{ 0x3041, 0x3096 }, -- Hiragana
	{ 0x309D, 0x309F },
	{ 0x30A1, 0x30FA }, -- Katakana
	{ 0x30FC, 0x30FF },
	{ 0x3131, 0x3163 }, -- Hangul compatibility Jamo, less the filler
	{ 0x3165, 0x318E },
	{ 0x4E00, 0x9FFF }, -- CJK
	{ 0xAC00, 0xD7A3 }, -- Hangul syllables
}

-- The characters that show nothing. They go before a match: "Cor\226\128\139vin" is
-- Corvin. The desktop drops the same ones (`aliases::is_hidden`).
local HIDDEN = {
	{ 0x00AD, 0x00AD }, -- the soft hyphen
	{ 0x034F, 0x034F }, -- the combining grapheme joiner
	{ 0x180E, 0x180E }, -- the Mongolian vowel separator
	{ 0x200B, 0x200F }, -- zero-width spaces and joiners, and direction marks
	{ 0x2060, 0x2064 }, -- the word joiner and invisible operators
	{ 0xFE00, 0xFE0F }, -- variation selectors
	{ 0xFEFF, 0xFEFF }, -- the zero-width no-break space
}

-- The code point of one character of UTF-8.
local function CodePoint(char)
	if #char == 1 then
		return char:byte()
	end
	local code = char:byte(1) % 2 ^ (7 - #char)
	for n = 2, #char do
		code = code * 64 + char:byte(n) % 64
	end
	return code
end

local function InRanges(code, ranges)
	for _, range in ipairs(ranges) do
		if code >= range[1] and code <= range[2] then
			return true
		end
	end
	return false
end

-- One character is part of a word: an ASCII letter or digit, or a letter past ASCII.
function TaskNames.IsWordChar(char)
	local code = CodePoint(char)
	if code < 0x80 then
		return char:find("^%w$") ~= nil
	end
	return InRanges(code, LETTERS)
end

-- The text with each character that shows nothing taken out.
function TaskNames.Visible(text)
	return (
		text:gsub(CHAR, function(char)
			if #char > 1 and InRanges(CodePoint(char), HIDDEN) then
				return ""
			end
		end)
	)
end

local IsWordChar = TaskNames.IsWordChar

-- The first and the last byte of the next word at or after `at`, or nil. A word is a run of
-- ASCII letters and digits and of letters past ASCII, such as "é".
local function NextWord(text, at)
	local first, last
	local from = at
	while true do
		local start, stop = text:find(CHAR, from)
		-- A broken byte ends a word too.
		if not start or (first and start > from) then
			return first, last
		end
		if IsWordChar(text:sub(start, stop)) then
			first, last = first or start, stop
		elseif first then
			return first, last
		end
		from = stop + 1
	end
end

-- Latin-1 capitals are U+00C0 to U+00DE, less ×, and each small letter is 32 above.
local function FoldLatin1(trail)
	if trail == "\151" then
		return nil
	end
	return "\195" .. string.char(trail:byte() + 32)
end

-- In Latin Extended-A, U+0100 to U+017F, each capital is one below its small letter. Where
-- a pair starts on an odd code, the capital is the odd one.
local function IsCapitalExtended(code)
	local even = code % 2 == 0
	if (code >= 0x100 and code <= 0x137) or (code >= 0x14A and code <= 0x177) then
		return even
	end
	return ((code >= 0x139 and code <= 0x148) or (code >= 0x179 and code <= 0x17E)) and not even
end

local function FoldExtended(lead, trail)
	local code = (lead:byte() - 0xC0) * 64 + trail:byte() - 0x80
	if code == 0x178 then
		return "\195\191"
	end
	if not IsCapitalExtended(code) then
		return nil
	end
	code = code + 1
	return string.char(0xC0 + math.floor(code / 64), 0x80 + code % 64)
end

-- Lowercase for ASCII and the Latin letters of European realms. `string.lower` knows only
-- ASCII.
function TaskNames.Fold(text)
	text = text:lower():gsub("\195([\128-\158])", FoldLatin1)
	return (text:gsub("([\196\197])([\128-\191])", FoldExtended))
end

-- Each source writes a name its own way: the friends list writes "Leborc James" on WoW
-- Forever. The key of the game, "Leborc-James", joins them, and every player with one first
-- name stays: { [first] = { "Kobee-Will", "Kobee-Moss" } }.
local function Add(names, name)
	if type(name) ~= "string" or issecretvalue(name) then
		return
	end
	local full = ns.TaskPeople.Full(name)
	if not full then
		return
	end
	local first = TaskNames.Fold(full:match("^[^-]+"))
	names[first] = names[first] or {}
	for _, known in ipairs(names[first]) do
		if known == full then
			return
		end
	end
	table.insert(names[first], full)
end

local function AddAll(names, list)
	for _, name in ipairs(list) do
		Add(names, name)
	end
end

-- The players of your tasks, and the one that the form names.
local function AddTaskPlayers(names)
	for _, entry in ipairs(ns.PlayerTasks.Given()) do
		Add(names, entry.task.doer)
	end
	for _, entry in ipairs(ns.PlayerTasks.Received()) do
		Add(names, entry.task.giver)
	end
	local form = ns.TaskForm.Draft()
	Add(names, form.doer)
	for _, step in ipairs(form.steps) do
		if step.kind == "meet" then
			Add(names, step.target)
		end
	end
end

-- The players of one first name in a steady order: your group first, then by the key.
-- `pairs` and the order of the guild roster change between calls.
local function Ordered(lists, group)
	local function Before(a, b)
		if group[a] ~= group[b] then
			return group[a] == true
		end
		return a < b
	end
	for _, players in pairs(lists) do
		table.sort(players, Before)
	end
	return lists
end

-- Every player whom the addon knows, by the folded first name: { [name] = { "Name-Realm" } },
-- and the players of your group.
local function KnownNames()
	local names, group = {}, {}
	for _, name in ipairs(ns.TaskPeople.GroupNames()) do
		group[ns.TaskPeople.Full(name) or name] = true
	end
	AddAll(names, ns.TaskPeople.GroupNames())
	AddAll(names, ns.TaskPeople.GuildNames())
	AddAll(names, ns.TaskPeople.FriendNames())
	AddTaskPlayers(names)
	-- An author can leave your group long before you accept the story.
	AddAll(names, ns.PlayerStories.Names())
	AddAll(names, ns.SeenPlayers.Names())
	return Ordered(names, group), group
end

-- Your first name, folded, and your surname.
local function MyName()
	local first, surname = ns.PlayerNames.MyParts()
	if first then
		return TaskNames.Fold(first), surname
	end
end

-- The story program reads a brace as a mark, so a brace that a player typed becomes a
-- parenthesis.
local function WithoutBraces(text)
	return (text:gsub("{", "("):gsub("}", ")"))
end

-- The last byte of a run of plain and no-break spaces that starts at `at`, or nil.
local function SpacesEnd(text, at)
	local stop
	while true do
		if text:sub(at, at) == " " then
			stop, at = at, at + 1
		elseif text:sub(at, at + 1) == "\194\160" then
			stop, at = at + 1, at + 2
		else
			return stop
		end
	end
end

-- The word right after `last` and a separator: "-", or " " for any run of plain and
-- no-break spaces. Its first and last byte.
local function WordAfter(text, last, separator)
	local gapEnd
	if separator == "-" then
		gapEnd = text:sub(last + 1, last + 1) == "-" and last + 1 or nil
	else
		gapEnd = SpacesEnd(text, last + 1)
	end
	if not gapEnd then
		return nil
	end
	local first, wordEnd = NextWord(text, gapEnd + 1)
	if first == gapEnd + 1 then
		return first, wordEnd
	end
end

-- True when the word starts with a capital: its first letter changes when folded.
local function IsCapitalized(word)
	local first = word:match("^" .. CHAR)
	return first ~= nil and TaskNames.Fold(first) ~= first
end

-- What follows a first name that ends at `last`: the realm or the surname after a dash, as
-- in "Ada-Stormrage", and the word with a capital after a space, as in "Kobee Bean". A
-- surname in lower case is a word: "Kobee will help" keeps "will" for Kobee Will.
local function Following(text, last)
	local after = {}
	local dashFirst, dashLast = WordAfter(text, last, "-")
	if dashFirst then
		after.dash, after.dashLast = TaskNames.Fold(text:sub(dashFirst, dashLast)), dashLast
	end
	local spaceFirst, spaceLast = WordAfter(text, after.dashLast or last, " ")
	local word = spaceFirst and text:sub(spaceFirst, spaceLast)
	if word and IsCapitalized(word) then
		after.capital, after.capitalLast = TaskNames.Fold(word), spaceLast
	end
	return after
end

-- The folded surname of a key, or nil.
local function SurnameOf(full)
	local surname = select(2, ns.PlayerNames.Parts(full))
	return surname and TaskNames.Fold(surname)
end

-- The player whose whole name the text writes, as "Kobee Will" or "Kobee-Will": "me" or a
-- key of `candidates`. Nil when no whole name matches.
local function WholeName(candidates, isMe, mySurname, after)
	local function Matches(surname)
		return surname and (surname == after.dash or surname == after.capital)
	end
	if isMe and Matches(mySurname and TaskNames.Fold(mySurname)) then
		return "me"
	end
	for _, full in ipairs(candidates) do
		if Matches(SurnameOf(full)) then
			return full
		end
	end
end

-- The last byte of a name that starts with the word ending at `last`: a realm or a surname
-- after a dash always goes with it, and a surname with a capital after a space goes with it
-- when it is the surname of this player.
local function NameEnd(last, after, surname)
	local stop = after.dashLast or last
	if surname and after.capital == surname then
		stop = after.capitalLast
	end
	return stop
end

-- The players of each known surname, folded, in the order of `Ordered`.
local function KnownSurnames(names, group)
	local surnames = {}
	for _, players in pairs(names) do
		for _, full in ipairs(players) do
			local surname = SurnameOf(full)
			if surname then
				surnames[surname] = surnames[surname] or {}
				table.insert(surnames[surname], full)
			end
		end
	end
	return Ordered(surnames, group)
end

-- The mark of a surname alone with a capital, "the Will family": "{~Kobee-Will}", with the
-- surname as the game writes it, and its player. Your screen keeps the word, in the case of
-- the game. Nil for a word that is
-- no surname of a known player, for a surname in lower case, and for your own surname:
-- the desktop hides that one.
local function LoneSurnameMark(word, surnames, mySurname)
	local folded = TaskNames.Fold(word)
	if not IsCapitalized(word) or (mySurname and folded == TaskNames.Fold(mySurname)) then
		return nil
	end
	local full = (surnames[folded] or {})[1]
	if full then
		-- The game's surname, not the typed one, names the row: "WILL" shows as "Will".
		local first, surname = ns.PlayerNames.Parts(full)
		return "{~" .. first .. "-" .. surname .. "}", full
	end
end

-- The mark of a player: the key of the game when the key holds a surname, so the desktop
-- learns it, and the first name alone when it holds a realm.
local function MarkOf(full)
	local first, surname = ns.PlayerNames.Parts(full)
	return "{" .. (surname and (first .. "-" .. surname) or full:match("^[^-]+")) .. "}"
end

-- Each player that the addon knows becomes "{Name}", or "{Name-Surname}" on WoW Forever,
-- and your own name becomes `$N`. A whole name goes first: your whole name, then the whole
-- name of a known player, and then a first name alone, which is you when it is yours. A
-- surname alone with a capital becomes a surname mark, "{~Kobee-Will}".
-- Returns the text, and the players that it marks as "Name-Realm", once each.
local function Mark(text, ownMark)
	-- The bridge reads broken bytes as U+FFFD, so the marks must read them so too.
	text = WithoutBraces(TaskNames.Visible(ns.Utf8.Lossy(text)))
	local names, group = KnownNames()
	local surnames = KnownSurnames(names, group)
	local me, mySurname = MyName()
	local parts, players, seen, at = {}, {}, {}, 1
	local function Named(full)
		if not seen[full] then
			seen[full] = true
			players[#players + 1] = full
		end
	end
	while true do
		local first, last = NextWord(text, at)
		if not first then
			break
		end
		parts[#parts + 1] = text:sub(at, first - 1)
		local word = text:sub(first, last)
		local folded = TaskNames.Fold(word)
		local candidates = names[folded] or {}
		local after = Following(text, last)
		local who = WholeName(candidates, folded == me, mySurname, after) or (folded == me and "me") or candidates[1]
		local surnameMark, surnameOf = LoneSurnameMark(word, surnames, mySurname)
		if who == "me" then
			parts[#parts + 1] = ownMark
			last = NameEnd(last, after, mySurname and TaskNames.Fold(mySurname))
		elseif who then
			parts[#parts + 1] = MarkOf(who)
			last = NameEnd(last, after, SurnameOf(who))
			Named(who)
		elseif surnameMark then
			parts[#parts + 1] = surnameMark
			Named(surnameOf)
		else
			parts[#parts + 1] = word
		end
		at = last + 1
	end
	parts[#parts + 1] = text:sub(at)
	return table.concat(parts), players
end

function TaskNames.Marked(text)
	return Mark(text, "$N")
end

-- The text for another player: your own name is your mark too, because `$N` names the
-- reader on the other side.
function TaskNames.ForPeer(text)
	local me = ns.TaskPeople.Me()
	return Mark(text, me and MarkOf(me) or "$N")
end

-- The first name and the surname of the key of a mark, or nil when it is no key: only
-- word letters, and at most one dash.
local function KeyParts(key)
	local first, surname = key:match("^([^-]+)%-?(.*)$")
	local function IsWord(part)
		local from, to = NextWord(part, 1)
		return from == 1 and to == #part
	end
	if not first or not IsWord(first) or (surname ~= "" and not IsWord(surname)) then
		return nil
	end
	return first, surname ~= "" and surname or nil
end

-- Each mark of a text that another player sent: the start and the end of the mark, whether
-- it is a surname mark, its first name, and its surname.
local function PeerMarks(text)
	local marks, at = {}, 1
	while true do
		local open, close, tilde, key = text:find("{(~?)([^{}]*)}", at)
		if not open then
			return marks
		end
		local first, surname = KeyParts(key)
		if first and (tilde == "" or surname) then
			marks[#marks + 1] = { open = open, close = close, alone = tilde == "~", first = first, surname = surname }
		end
		at = close + 1
	end
end

-- The text as your screen shows it: a mark of a player as its first name, and a surname
-- mark as the surname.
function TaskNames.Shown(text)
	local parts, at = {}, 1
	for _, mark in ipairs(PeerMarks(text)) do
		parts[#parts + 1] = text:sub(at, mark.open - 1)
		parts[#parts + 1] = mark.alone and mark.surname or mark.first
		at = mark.close + 1
	end
	parts[#parts + 1] = text:sub(at)
	return table.concat(parts)
end

-- A text that another player sent, for the story program: the marks of the author stay,
-- a mark of you becomes `$N`, and the rest is marked as `Marked` does. Returns the text,
-- and the players that it marks, once each.
function TaskNames.MarkedKeeping(text)
	local me, mySurname = MyName()
	local parts, players, seen, at = {}, {}, {}, 1
	local function Named(full)
		if full and not seen[full] then
			seen[full] = true
			players[#players + 1] = full
		end
	end
	local function MarkRest(rest)
		local marked, named = TaskNames.Marked(rest)
		parts[#parts + 1] = marked
		for _, full in ipairs(named) do
			Named(full)
		end
	end
	for _, mark in ipairs(PeerMarks(text)) do
		MarkRest(text:sub(at, mark.open - 1))
		local isMe = TaskNames.Fold(mark.first) == me
			and (not mark.surname or (mySurname and TaskNames.Fold(mark.surname) == TaskNames.Fold(mySurname)))
		if isMe then
			parts[#parts + 1] = "$N"
		else
			parts[#parts + 1] = text:sub(mark.open, mark.close)
			Named(ns.TaskPeople.Full(mark.surname and (mark.first .. "-" .. mark.surname) or mark.first))
		end
		at = mark.close + 1
	end
	MarkRest(text:sub(at))
	return table.concat(parts), players
end

-- A line with the race and the class of each player whom the game shows now, for the card
-- that a model reads. A player out of sight gets no line.
function TaskNames.Described(players)
	local lines = {}
	for _, full in ipairs(players) do
		local unit = ns.TaskPeople.UnitOf(full)
		if unit then
			local race, class = ns.Units.Shown(select(2, UnitRace(unit))), ns.Units.Shown(select(2, UnitClass(unit)))
			lines[#lines + 1] = ns.Inputs.PlayerDescribed(time(), full:match("^[^-]+"), race, class)
		end
	end
	return lines
end

local function SendDescribed(players)
	for _, line in ipairs(TaskNames.Described(players)) do
		ns.Outbox.Add(line)
	end
end

-- The text for the story program. The lines that describe its players go first.
function TaskNames.Send(text)
	local marked, players = TaskNames.Marked(text)
	SendDescribed(players)
	return marked
end

-- A text of the hero can name at most this many players. The desktop refuses more.
TaskNames.MAX_NAMES = 32

-- The players that a text names, in the order of the text, as "First-Surname", or the
-- first name alone. A realm is no surname.
function TaskNames.Named(text)
	local _, players = TaskNames.Marked(text)
	local names = {}
	for n, full in ipairs(players) do
		local first, surname = ns.PlayerNames.Parts(full)
		names[n] = surname and (first .. "-" .. surname) or full:match("^[^-]+")
	end
	return names, players
end

-- The players of a text of the hero, for the story program, which keeps the text as you
-- typed it. The lines that describe them go first.
function TaskNames.SendNamed(text)
	local names, players = TaskNames.Named(text)
	SendDescribed(players)
	return names
end
