-- Marks the names of players in a text for the story program, which gives each player an
-- ID before a model reads the text (GAMEPLAY.md 4.7, 4.8, 5.11). A name matches as a whole
-- word, in any case, with or without its realm: "élise", "ÉLISE", and "Élise-Stormrage"
-- are all Élise.

local _, ns = ...

local TaskNames = {}
ns.TaskNames = TaskNames

-- One character of UTF-8: an ASCII byte, or a lead byte and its continuation bytes.
local CHAR = "[%z\1-\127\194-\244][\128-\191]*"

-- Past ASCII, a code point is a letter unless it is in one of these ranges. The desktop
-- holds the same table (`aliases::is_word_char`), and the addon test
-- `the_addon_and_the_desktop_agree_on_every_letter` checks every code point. A mark or a
-- symbol is a sign, so a name before it still gets its mark.
local SIGNS = {
	{ 0x0080, 0x00BF }, -- the no-break space, « », ¡, ¿, ·
	{ 0x00D7, 0x00D7 }, -- ×
	{ 0x00F7, 0x00F7 }, -- ÷
	{ 0x02C2, 0x02C5 }, -- modifier symbols, such as ˂
	{ 0x02D2, 0x02DF },
	{ 0x02E5, 0x02EB },
	{ 0x02ED, 0x02ED },
	{ 0x02EF, 0x02FF },
	{ 0x0300, 0x036F }, -- combining marks, such as the acute of "e\204\129"
	{ 0x037E, 0x037E }, -- the Greek question mark
	{ 0x0387, 0x0387 }, -- the Greek ano teleia
	{ 0x0591, 0x05C7 }, -- Hebrew points and signs, such as ׃
	{ 0x0600, 0x061F }, -- Arabic signs, such as ،
	{ 0x064B, 0x065F }, -- Arabic marks
	{ 0x066A, 0x066D },
	{ 0x06D4, 0x06D4 },
	{ 0x2000, 0x2FFF }, -- ’, —, …, ⁿ, ℕ, Ⓐ, and other signs
	{ 0x3000, 0x303F }, -- the signs of CJK, such as 、 and 。
	{ 0xFE00, 0xFE6F }, -- variation selectors, and small and vertical forms
	{ 0xFEFF, 0xFEFF }, -- the zero-width no-break space
	{ 0xFF00, 0xFF0F }, -- full-width signs, such as ，
	{ 0xFF1A, 0xFF20 },
	{ 0xFF3B, 0xFF40 },
	{ 0xFF5B, 0xFF65 },
	{ 0xFFF0, 0xFFFF }, -- specials, such as �
	{ 0x10000, 0x10FFFF }, -- past the BMP: emoji
}

-- The letters and numbers inside SIGNS: ª ² ³ µ ¹ º ¼ ½ ¾, and the Roman numerals.
local LETTERS_AMONG_SIGNS = {
	{ 0x00AA, 0x00AA },
	{ 0x00B2, 0x00B3 },
	{ 0x00B5, 0x00B5 },
	{ 0x00B9, 0x00BA },
	{ 0x00BC, 0x00BE },
	{ 0x2160, 0x2188 },
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
	return InRanges(code, LETTERS_AMONG_SIGNS) or not InRanges(code, SIGNS)
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

local function Add(names, name)
	if type(name) ~= "string" or issecretvalue(name) then
		return
	end
	local short = name:match("^[^-]+")
	if short then
		names[TaskNames.Fold(short)] = ns.TaskPeople.Full(name) or name
	end
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

-- Every player whom the addon knows, by the folded name: { [name] = "Name-Realm" }.
local function KnownNames()
	local names = {}
	AddAll(names, ns.TaskPeople.GroupNames())
	AddAll(names, ns.TaskPeople.GuildNames())
	AddAll(names, ns.TaskPeople.FriendNames())
	AddTaskPlayers(names)
	-- An author can leave your group long before you accept the story.
	AddAll(names, ns.PlayerStories.Names())
	return names
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

-- The end of a realm right after a word that ends at `last`, as in "Ada-Stormrage".
local function RealmEnd(text, last)
	if text:sub(last + 1, last + 1) ~= "-" then
		return last
	end
	local first, realmEnd = NextWord(text, last + 2)
	return first == last + 2 and realmEnd or last
end

-- True when the word starts with a capital: its first letter changes when folded.
local function IsCapitalized(word)
	local first = word:match("^" .. CHAR)
	return first ~= nil and TaskNames.Fold(first) ~= first
end

-- The end of the surname right after a word that ends at `last`, as in "Kobee Bean". A
-- surname with a capital is part of the name, so it gets the mark too. In lower case it
-- is a word: "Kobee will help" keeps "will" for Kobee Will.
local function SurnameEnd(text, last, surname)
	if not surname or text:sub(last + 1, last + 1) ~= " " then
		return last
	end
	local first, wordEnd = NextWord(text, last + 2)
	local word = first == last + 2 and text:sub(first, wordEnd)
	if word and IsCapitalized(word) and TaskNames.Fold(word) == TaskNames.Fold(surname) then
		return wordEnd
	end
	return last
end

-- Each player that the addon knows becomes "{Name}", as the game writes the name, and your
-- own name becomes `$N`. Returns the text, and the players that it marks as "Name-Realm",
-- once each.
function TaskNames.Marked(text)
	text = WithoutBraces(text)
	local names, me, mySurname = KnownNames(), MyName()
	local parts, players, seen, at = {}, {}, {}, 1
	while true do
		local first, last = NextWord(text, at)
		if not first then
			break
		end
		parts[#parts + 1] = text:sub(at, first - 1)
		local folded = TaskNames.Fold(text:sub(first, last))
		local full = names[folded]
		if folded == me then
			parts[#parts + 1] = "$N"
			last = SurnameEnd(text, RealmEnd(text, last), mySurname)
		elseif full then
			parts[#parts + 1] = "{" .. full:match("^[^-]+") .. "}"
			last = SurnameEnd(text, RealmEnd(text, last), select(2, ns.PlayerNames.Parts(full)))
			if not seen[full] then
				seen[full] = true
				players[#players + 1] = full
			end
		else
			parts[#parts + 1] = text:sub(first, last)
		end
		at = last + 1
	end
	parts[#parts + 1] = text:sub(at)
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
