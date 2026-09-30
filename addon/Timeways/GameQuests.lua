-- The quests of the game that you take and turn in (GAMEPLAY.md 5.4). The quest log puts
-- a quest of your class under a header with the name of the class.

local _, ns = ...

local GameQuests = {}
ns.GameQuests = GameQuests

-- The title and kind of each quest that the log showed, by quest ID. The turn-in event
-- carries only the ID, so the scan at the take keeps the rest.
local known = {}

-- The client can hide a value from addons. A hidden value is never compared or sent.
local function Readable(value)
	return type(value) == "string" and value ~= "" and not issecretvalue(value)
end

local function ClassName()
	local name = UnitClass("player")
	return Readable(name) and name or nil
end

function GameQuests.Scan()
	local class, header = ClassName(), nil
	for index = 1, C_QuestLog.GetNumQuestLogEntries() do
		local info = C_QuestLog.GetInfo(index)
		if type(info) == "table" and Readable(info.title) then
			if info.isHeader then
				header = info.title
			elseif type(info.questID) == "number" then
				local kind = class and header == class and "class" or "normal"
				known[info.questID] = { title = info.title, kind = kind }
			end
		end
	end
end

function GameQuests.Accepted(questID)
	GameQuests.Scan()
	local quest = known[questID]
	if quest then
		ns.Outbox.Add(ns.Inputs.GameQuestAccepted(time(), quest.title, quest.kind))
	end
end

function GameQuests.TurnedIn(questID)
	local quest = known[questID]
	if not quest then
		return
	end
	known[questID] = nil
	ns.Outbox.Add(ns.Inputs.GameQuestDone(time(), quest.title, quest.kind))
end
