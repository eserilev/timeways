-- The quests of the game that you take and turn in (GAMEPLAY.md 5.4). The quest log puts
-- a quest of your class under a header with the name of the class.

local _, ns = ...

local GameQuests = {}
ns.GameQuests = GameQuests

-- The title and kind of each quest that the log showed, by quest ID. The turn-in event
-- carries only the ID, so the scan at the take keeps the rest.
local known = {}

-- A buff or debuff that comes this soon after an event of a quest belongs to that quest.
local QUEST_EVENT_SECONDS = 60
local lastEvent

local function Noted(quest)
	lastEvent = { title = quest.title, at = time() }
end

-- The title of the quest of the last quest event, while it is recent.
function GameQuests.Recent()
	if lastEvent and time() - lastEvent.at <= QUEST_EVENT_SECONDS then
		return lastEvent.title
	end
end

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
		Noted(quest)
	end
end

-- An objective of the quest moved on.
function GameQuests.Progress(questID)
	if not known[questID] then
		GameQuests.Scan()
	end
	if known[questID] then
		Noted(known[questID])
	end
end

function GameQuests.TurnedIn(questID)
	local quest = known[questID]
	if not quest then
		return
	end
	known[questID] = nil
	ns.Outbox.Add(ns.Inputs.GameQuestDone(time(), quest.title, quest.kind))
	Noted(quest)
end
