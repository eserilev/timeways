-- Marks the names of players in a text for the story program, which gives each player an
-- ID before a model reads the text (GAMEPLAY.md 4.7, 4.8, 5.11). A name matches as a whole
-- word, in any case, with or without its realm: "élise", "ÉLISE", and "Élise-Stormrage"
-- are all Élise.

local _, ns = ...

local TaskNames = {}
ns.TaskNames = TaskNames

-- One character of UTF-8: an ASCII byte, or a lead byte and its continuation bytes.
local CHAR = "[%z\1-\127\194-\244][\128-\191]*"

-- The signs past ASCII that end a word, as the desktop's `char::is_alphanumeric` does. A
-- name before a curly apostrophe, a dash, or an emoji still gets its mark.
local NOT_LETTERS = {
	"^\194", -- U+0080 to U+00BF: the no-break space, « », ¡, ¿, ·
	"^\195[\151\183]", -- × and ÷
	"^\226", -- U+2000 to U+2FFF: ’, —, …, and other signs
	"^\227\128", -- U+3000 to U+303F: the signs of CJK, such as 、 and 。
	"^\239\188[\128-\143\154-\160\187-\191]", -- full-width signs, such as ，
	"^\239\189[\128\155-\165]",
	"^[\240-\244]", -- emoji
}

local function IsWordChar(char)
	if #char == 1 then
		return char:find("^%w$") ~= nil
	end
	for _, sign in ipairs(NOT_LETTERS) do
		if char:find(sign) then
			return false
		end
	end
	return true
end

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

local function MyName()
	local me = UnitName("player")
	if type(me) == "string" and not issecretvalue(me) then
		return TaskNames.Fold(me)
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

-- Each player that the addon knows becomes "{Name}", as the game writes the name, and your
-- own name becomes `$N`. Returns the text, and the players that it marks as "Name-Realm",
-- once each.
function TaskNames.Marked(text)
	text = WithoutBraces(text)
	local names, me = KnownNames(), MyName()
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
			last = RealmEnd(text, last)
		elseif full then
			parts[#parts + 1] = "{" .. full:match("^[^-]+") .. "}"
			last = RealmEnd(text, last)
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

-- The text for the story program. The lines that describe its players go first.
function TaskNames.Send(text)
	local marked, players = TaskNames.Marked(text)
	for _, line in ipairs(TaskNames.Described(players)) do
		ns.Outbox.Add(line)
	end
	return marked
end
