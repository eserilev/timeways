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

-- The players of one first name in a steady order: your group first, then the players
-- that you know, then the players that you only saw, each by the key. `pairs` and the order
-- of the guild roster change between calls. `ranks` holds 1 for your group and 3 for a
-- player that you only saw.
local function Ordered(lists, ranks)
	local function Before(a, b)
		local rankA, rankB = ranks[a] or 2, ranks[b] or 2
		if rankA ~= rankB then
			return rankA < rankB
		end
		return a < b
	end
	for _, players in pairs(lists) do
		table.sort(players, Before)
	end
	return lists
end

-- The keys of the players that a list of names holds.
local function Keys(list)
	local keys = {}
	for _, name in ipairs(list) do
		local full = type(name) == "string" and not issecretvalue(name) and ns.TaskPeople.Full(name)
		if full then
			keys[full] = true
		end
	end
	return keys
end

-- Every player whom the addon knows, by the folded first name: { [name] = { "Name-Realm" } },
-- and the rank of each player for `Ordered`.
local function KnownNames()
	local names, ranks = {}, {}
	for full in pairs(Keys(ns.TaskPeople.GroupNames())) do
		ranks[full] = 1
	end
	AddAll(names, ns.TaskPeople.GroupNames())
	AddAll(names, ns.TaskPeople.GuildNames())
	AddAll(names, ns.TaskPeople.FriendNames())
	AddTaskPlayers(names)
	-- An author can leave your group long before you accept the story.
	AddAll(names, ns.PlayerStories.Names())
	local known = {}
	for _, players in pairs(names) do
		for _, full in ipairs(players) do
			known[full] = true
		end
	end
	for full in pairs(Keys(ns.SeenPlayers.Names())) do
		if not known[full] then
			ranks[full] = 3
		end
	end
	AddAll(names, ns.SeenPlayers.Names())
	return Ordered(names, ranks), ranks
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

-- The players of each known surname, folded, in the order of `Ordered`. A player that you
-- only saw gives no surname: a passerby "Bob Light" never turns "The Light" into a mark.
local function KnownSurnames(names, ranks)
	local surnames = {}
	for _, players in pairs(names) do
		for _, full in ipairs(players) do
			local surname = ranks[full] ~= 3 and SurnameOf(full)
			if surname then
				surnames[surname] = surnames[surname] or {}
				table.insert(surnames[surname], full)
			end
		end
	end
	return Ordered(surnames, ranks)
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
-- The players that the addon knows, read once for one text: the game lists can be long.
local function Known()
	local names, ranks = KnownNames()
	local me, mySurname = MyName()
	return { names = names, surnames = KnownSurnames(names, ranks), me = me, mySurname = mySurname }
end

-- The text as the story program reads it: broken bytes as U+FFFD, as the bridge reads them,
-- no character that shows nothing, and each typed brace as a parenthesis.
function TaskNames.Typed(text)
	return WithoutBraces(TaskNames.Visible(ns.Utf8.Lossy(text)))
end

local function Mark(text, ownMark, known)
	text = TaskNames.Typed(text)
	local names, surnames, me, mySurname = known.names, known.surnames, known.me, known.mySurname
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
	return Mark(text, "$N", Known())
end

-- The text for another player: your own name is your mark too, because `$N` names the
-- reader on the other side.
function TaskNames.ForPeer(text)
	local me = ns.TaskPeople.Me()
	return Mark(text, me and MarkOf(me) or "$N", Known())
end

-- The most players that one story names, besides the reader: the author can mark any
-- word, so a story teaches the reader's desktop a bounded number of names. The desktop
-- refuses a story with more.
TaskNames.MAX_PEER_MARKS = 32

-- The longest first name that the desktop takes, in bytes.
local MAX_NAME_BYTES = 48

-- English words that a mark of a story never teaches, though the game allows them as
-- names. The desktop holds the same list (`aliases::FUNCTION_WORDS`).
local FUNCTION_WORDS = {}
for word in
	([[the and but then when he she we it they you his her my our their in on at of
	to for with from by as is was be not no yes this that there here so or if an me him]]):gmatch("%a+")
do
	FUNCTION_WORDS[word] = true
end

-- True for a text of word characters only, as the desktop reads a word.
local function IsWordOnly(part, letters)
	if part == "" then
		return false
	end
	for char in part:gmatch(CHAR) do
		if not IsWordChar(char) or (letters and char:find("^%d$")) then
			return false
		end
	end
	return true
end

-- The first name and the surname of the key of a mark, or nil when the desktop reads no
-- mark there: a first name of letters of at most 48 bytes, and a surname of word
-- characters after one dash.
local function KeyParts(key)
	local first, surname = key:match("^([^-]*)%-(.*)$")
	first = first or key
	if not IsWordOnly(first, true) or #first > MAX_NAME_BYTES then
		return nil
	end
	if surname and not IsWordOnly(surname, false) then
		return nil
	end
	return first, surname
end

-- True for a first name that a mark of a story can teach: 2 to 12 letters, as the game
-- allows a name, and no function word. Else the mark is its word.
local function IsLikelyName(first)
	local letters = select(2, first:gsub(CHAR, ""))
	return letters >= 2 and letters <= 12 and not FUNCTION_WORDS[TaskNames.Fold(first)]
end

-- The character that ends at byte `last`, or nil.
local function CharBefore(text, last)
	local start = last
	while start > 1 and text:byte(start) and text:byte(start) >= 128 and text:byte(start) < 192 do
		start = start - 1
	end
	return last >= 1 and text:sub(start, last) or nil
end

-- The character that starts at byte `at`, or nil.
local function CharAt(text, at)
	return text:match("^" .. CHAR, at)
end

-- Each mark of a text that another player sent, as the desktop reads it: the start and
-- the end of the mark, whether it is a surname mark, its first name, its surname, its key,
-- and whether it is a likely name. A mark that touches a word character was typed, and is
-- no mark.
local function PeerMarks(text)
	local marks, at = {}, 1
	while true do
		local open, close, tilde, key = text:find("{(~?)([^{}]*)}", at)
		if not open then
			return marks
		end
		local first, surname = KeyParts(key)
		local before, after = CharBefore(text, open - 1), CharAt(text, close + 1)
		local touches = (before and IsWordChar(before)) or (after and IsWordChar(after))
		if first and not touches and (tilde == "" or surname) then
			marks[#marks + 1] = {
				open = open,
				close = close,
				alone = tilde == "~",
				first = first,
				surname = surname,
				key = key,
				likely = IsLikelyName(first),
			}
		end
		at = close + 1
	end
end

-- The word that a mark shows: its first name, or the surname of a surname mark.
local function MarkWord(mark)
	return mark.alone and mark.surname or mark.first
end

-- True for a mark of the reader: its first name, alone or with its surname.
local function IsMine(mark, me, mySurname)
	local sameSurname = not mark.surname
		or (mySurname ~= nil and TaskNames.Fold(mark.surname) == TaskNames.Fold(mySurname))
	return TaskNames.Fold(mark.first) == me and sameSurname
end

-- A text of a peer as the reader's addon reads it: broken bytes as U+FFFD, no character
-- that shows nothing, and a typed `$N` as "N": on the reader's desktop `$N` names the
-- reader, so a peer can't type it.
local function PeerText(text)
	return (TaskNames.Visible(ns.Utf8.Lossy(text)):gsub("%$N", "N"))
end

-- The text with each mark that `turns` picks as its word. A mark that touches the word
-- of a mark before it is no mark any more, as the desktop reads the text.
local function AsWords(text, turns)
	local parts, at = {}, 1
	for _, mark in ipairs(PeerMarks(text)) do
		if turns(mark) then
			parts[#parts + 1] = text:sub(at, mark.open - 1)
			parts[#parts + 1] = MarkWord(mark)
			at = mark.close + 1
		end
	end
	parts[#parts + 1] = text:sub(at)
	return table.concat(parts)
end

local function Unlikely(mark)
	return not mark.likely
end

-- The text as your screen shows it: each mark as its word, every other brace as a
-- parenthesis, as the desktop reads the text.
local function ShownText(text)
	local parts, at = {}, 1
	for _, mark in ipairs(PeerMarks(text)) do
		parts[#parts + 1] = WithoutBraces(text:sub(at, mark.open - 1))
		parts[#parts + 1] = MarkWord(mark)
		at = mark.close + 1
	end
	parts[#parts + 1] = WithoutBraces(text:sub(at))
	return table.concat(parts)
end

function TaskNames.Shown(text)
	return ShownText(AsWords(PeerText(text), Unlikely))
end

-- The folded key of the mark of a player: "kobee-will", or "bram" for a player with a realm.
local function KeyOf(full)
	return TaskNames.Fold(MarkOf(full):sub(2, -2))
end

TaskNames.TOO_MANY_PLAYERS = "This story names too many players to keep."

-- What the reader keeps of a story of a peer, the title and the paragraphs together, for
-- the story program, which counts every mark of the story: a mark of an unlikely name as
-- its word, the players that the reader's addon marks counted first, then the marks of the
-- author that fit `MAX_PEER_MARKS`, the others as their words. A mark of the reader stays a
-- mark, and the desktop reads it as `$N`. Returns the texts, the players that they mark,
-- the words of the author's marks that stay, and the texts as your screen shows them, or
-- nil and why.
local function KeptStory(texts, known)
	local cut = {}
	for n, text in ipairs(texts) do
		cut[n] = AsWords(PeerText(text), Unlikely)
	end
	local keys, count = {}, 0
	local function Count(full)
		local key = KeyOf(full)
		if not keys[key] then
			keys[key], count = true, count + 1
		end
	end
	-- Each text with its marks, and the text between them marked by the reader's addon.
	local function Pieces(text, each)
		local at = 1
		for _, mark in ipairs(PeerMarks(text)) do
			each(text:sub(at, mark.open - 1), mark)
			at = mark.close + 1
		end
		each(text:sub(at), nil)
	end
	for _, text in ipairs(cut) do
		Pieces(text, function(rest)
			for _, full in ipairs(select(2, Mark(rest, "$N", known))) do
				Count(full)
			end
		end)
	end
	if count > TaskNames.MAX_PEER_MARKS then
		return nil, TaskNames.TOO_MANY_PLAYERS
	end
	local function PastTheLimit(mark)
		local key = TaskNames.Fold(mark.key)
		if IsMine(mark, known.me, known.mySurname) or keys[key] then
			return false
		end
		if count < TaskNames.MAX_PEER_MARKS then
			keys[key], count = true, count + 1
			return false
		end
		return true
	end
	for n, text in ipairs(cut) do
		cut[n] = AsWords(text, PastTheLimit)
	end
	local marked, shown, players, seen, words = {}, {}, {}, {}, {}
	local function Named(full)
		if full and not seen[full] then
			seen[full] = true
			players[#players + 1] = full
		end
	end
	keys, count = {}, 0
	for n, text in ipairs(cut) do
		local parts = {}
		Pieces(text, function(rest, mark)
			local restMarked, named = Mark(rest, "$N", known)
			parts[#parts + 1] = restMarked
			for _, full in ipairs(named) do
				Named(full)
				Count(full)
			end
			if mark then
				parts[#parts + 1] = text:sub(mark.open, mark.close)
				if not IsMine(mark, known.me, known.mySurname) then
					local full = ns.TaskPeople.Full(mark.surname and (mark.first .. "-" .. mark.surname) or mark.first)
					Named(full)
					if full then
						Count(full)
					end
					words[#words + 1] = MarkWord(mark)
				end
			end
		end)
		marked[n], shown[n] = table.concat(parts), ShownText(text)
	end
	if count > TaskNames.MAX_PEER_MARKS then
		return nil, TaskNames.TOO_MANY_PLAYERS
	end
	return marked, players, words, shown
end

-- A story that another player sent, for the story program: its texts, the title first.
-- Returns the texts and the players that they mark, or nil and why.
function TaskNames.MarkedStory(texts)
	return KeptStory(texts, Known())
end

-- The texts of a story as your screen shows them, as the desktop will keep them.
function TaskNames.ShownStory(texts)
	local _, _, _, shown = KeptStory(texts, Known())
	if shown then
		return shown
	end
	local each = {}
	for n, text in ipairs(texts) do
		each[n] = TaskNames.Shown(text)
	end
	return each
end

-- The names that the marks of the author show, once each in any case, for the page before
-- Accept: the ones that the reader keeps.
function TaskNames.PeerNames(texts)
	local _, _, words = KeptStory(texts, Known())
	local names, seen = {}, {}
	for _, word in ipairs(words or {}) do
		local folded = TaskNames.Fold(word)
		if not seen[folded] then
			seen[folded] = true
			names[#names + 1] = word
		end
	end
	return names
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
