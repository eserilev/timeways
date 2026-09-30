-- Takes the names of players out of a text for a model (GAMEPLAY.md 4.7, 5.11). A name
-- matches as a whole word, in any case, with or without its realm: "élise", "ÉLISE", and
-- "Élise-Stormrage" are all Élise.

local _, ns = ...

local TaskNames = {}
ns.TaskNames = TaskNames

-- A word: ASCII letters and digits, and every byte of a UTF-8 letter such as "é".
local WORD = "[%w\128-\255]+"

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
		names[TaskNames.Fold(short)] = true
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

-- Every player whom the addon knows, folded: { [name] = true }.
local function KnownNames()
	local names = {}
	AddAll(names, ns.TaskPeople.GroupNames())
	AddAll(names, ns.TaskPeople.GuildNames())
	AddAll(names, ns.TaskPeople.FriendNames())
	AddTaskPlayers(names)
	return names
end

local function MyName()
	local me = UnitName("player")
	if type(me) == "string" and not issecretvalue(me) then
		return TaskNames.Fold(me)
	end
end

-- Other players become "my friend", and your own name becomes `$N`.
function TaskNames.WithoutNames(text)
	local names, me = KnownNames(), MyName()
	local function Alias(word)
		local folded = TaskNames.Fold(word)
		if folded == me then
			return "$N"
		end
		return names[folded] and "my friend" or nil
	end
	text = text:gsub("(" .. WORD .. ")%-" .. WORD, Alias)
	return (text:gsub(WORD, Alias))
end
