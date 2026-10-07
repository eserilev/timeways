-- Dev mode: `/twdev` makes the moments of hours of play in seconds, through the real code
-- of the addon and the desktop (TESTING.md, "Dev mode"). It does nothing unless the
-- desktop says that dev mode is on, in the `dev` mark of the journal. A player turns it on
-- only at the desktop, so no player can start it by accident in the game.

-- The saved variables come from the TOC, so only _G can reach them.
--# selene: allow(global_usage)

local _, ns = ...

local Dev = {}
ns.Dev = Dev

local PREFIX = "|cffc8a064Timeways dev|r: "

-- The desktop said so in the newest journal. Off until a journal comes.
local desktopOn = false
-- Above 0 while a command of `/twdev` runs: each event line of the outbox gets the mark.
local marking = 0
-- The commands: name to { usage, run }, and the names in the order of the help.
local commands, order = {}, {}

function Dev.Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

-- From the first page of each journal. A journal with no `dev` mark turns it off. A
-- message that waits to go out belongs to the world before the change, so it goes nowhere.
function Dev.DesktopSays(value)
	local on = value == true
	if on == desktopOn then
		return
	end
	desktopOn = on
	ns.TaskChannel.DropWaiting()
	ns.Msp.DropWaiting()
	ns.DevPeer.Clear()
	if on then
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: Dev mode is on. Nothing is shared with other players.")
	end
end

function Dev.IsOn()
	return desktopOn
end

-- The quests and the stories of a dev world, by the name of their saved variable. They live
-- in memory only, so no fake player reaches the saved ones, and no real one reaches a dev
-- world.
local memory = {}

-- The table of the saved variable `name`, or its table in memory while dev mode is on.
function Dev.SavedTable(name)
	if desktopOn then
		memory[name] = memory[name] or {}
		return memory[name]
	end
	if type(_G[name]) ~= "table" then
		_G[name] = {}
	end
	return _G[name]
end

-- A dev world holds fake data, so no message goes to a real player while dev mode is on.
function Dev.CanTalkToPlayers()
	return not desktopOn
end

-- Every fake line carries `dev = true`, so the desktop refuses it while dev mode is off.
function Dev.Mark(input)
	if marking > 0 then
		input.dev = true
	end
end

-- Runs `run` with each event line that it adds marked as fake. An error still ends the mark.
function Dev.Faking(run, ...)
	marking = marking + 1
	local ok, problem = pcall(run, ...)
	marking = marking - 1
	if not ok then
		error(problem, 0)
	end
end

-- Runs `run` while the game seems to show the NPC `name` as your target and as the NPC of
-- the open window. `hostile` makes it an NPC that you can attack. The real code of the
-- addon reads the target only through `ns.Units`, so the rest of it runs as in play.
function Dev.WithNpc(name, hostile, run)
	local units = ns.Units
	local real = { NpcName = units.NpcName, FriendlyNpcName = units.FriendlyNpcName }
	local function Fake(unit)
		if unit == "target" or unit == "npc" then
			return name
		end
		return real.NpcName(unit)
	end
	units.NpcName = Fake
	units.FriendlyNpcName = function(unit)
		if hostile and (unit == "target" or unit == "npc") then
			return nil
		end
		return Fake(unit)
	end
	local ok, problem = pcall(Dev.Faking, run)
	units.NpcName, units.FriendlyNpcName = real.NpcName, real.FriendlyNpcName
	if not ok then
		error(problem, 0)
	end
end

-- `usage` is the line of the help: "level <n>".
function Dev.Add(name, usage, run)
	commands[name] = { usage = usage, run = run }
	order[#order + 1] = name
end

-- The usage line of each command, in the order of the help.
function Dev.Usages()
	local usages = {}
	for _, name in ipairs(order) do
		usages[#usages + 1] = commands[name].usage
	end
	return usages
end

local function Help()
	Dev.Say("each command fakes one moment of play. Commands:")
	for _, usage in ipairs(Dev.Usages()) do
		DEFAULT_CHAT_FRAME:AddMessage("  /twdev " .. usage)
	end
end

-- "kill Mor'Ladim boss" gives "kill" and "Mor'Ladim boss".
function Dev.Command(message)
	if not desktopOn then
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: Dev mode is off.")
		return
	end
	local name, rest = message:match("^%s*(%S*)%s*(.-)%s*$")
	local command = commands[name:lower()]
	if name == "" or name:lower() == "help" or not command then
		Help()
		return
	end
	Dev.Faking(command.run, rest)
	ns.Outbox.Flush()
end

-- Splits "Westfall / Moonbrook" into "Westfall" and "Moonbrook". The second part can be nil.
function Dev.Parts(text)
	local first, second = text:match("^(.-)%s*/%s*(.-)$")
	if not first then
		return text, nil
	end
	return first, second ~= "" and second or nil
end

-- Splits "Swift Brown Wolf epic" into "Swift Brown Wolf" and "epic", when the last word
-- is one of `words`.
function Dev.LastWord(text, words)
	local head, last = text:match("^(.-)%s+(%S+)$")
	if last and words[last:lower()] then
		return head, last:lower()
	end
	return text, nil
end
