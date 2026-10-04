-- The open steps of your side quests that the addon must watch (docs/plans/quest-variety.md
-- 10.2). The journal says which steps are open. Only the desktop moves a step, so a step
-- that opens later needs a new journal.

local _, ns = ...

local QuestSteps = {}
ns.QuestSteps = QuestSteps

-- The goals that the addon watches itself, while their steps are open.
local WATCHED = { kill = true, carry = true, visit_at = true }

-- The creatures of the open kill steps, as a set.
local hunted = {}
-- The items of the open carry steps, by NPC.
local carries = {}
-- True while a time-of-day step is open.
local watchesHour = false
-- True while a watched step waits for a step before it.
local watchedStepWaits = false
-- A batch of events goes out about once a minute, so this asks at most once for each batch.
local JOURNAL_SECONDS = 60
local journalAskedAt

local function Accepted(quest)
	return type(quest) == "table" and quest.status == "accepted" and type(quest.steps) == "table"
end

local function Watch(step)
	if step.state == "later" and WATCHED[step.goal] then
		watchedStepWaits = true
	end
	if step.state == "open" and step.goal == "kill" and type(step.creature) == "string" then
		hunted[step.creature] = true
	end
	if step.state == "open" and step.goal == "visit_at" then
		watchesHour = true
	end
	if step.state == "open" and step.goal == "carry" and type(step.npc) == "string" and type(step.item) == "string" then
		carries[step.npc] = carries[step.npc] or {}
		table.insert(carries[step.npc], step.item)
	end
end

-- From each journal: the open steps that the addon watches, and whether one waits.
function QuestSteps.Read(quests)
	hunted, carries, watchesHour, watchedStepWaits = {}, {}, false, false
	for _, quest in ipairs(type(quests) == "table" and quests or {}) do
		if Accepted(quest) then
			-- A hidden step can be a kill, a carry, or a time.
			if type(quest.hidden_steps) == "number" and quest.hidden_steps > 0 then
				watchedStepWaits = true
			end
			for _, step in ipairs(quest.steps) do
				if type(step) == "table" then
					Watch(step)
				end
			end
		end
	end
	ns.Foes.Hunt(hunted)
end

-- The creatures of the open kill steps, as a set.
function QuestSteps.Hunted()
	return hunted
end

-- The items of the open carry steps of this NPC.
function QuestSteps.CarriesFor(npc)
	return carries[npc] or {}
end

-- True while a carry step is open, so the page shows the count of your bags.
function QuestSteps.Carries()
	return next(carries) ~= nil
end

-- True while a visit_at step is open, so a change of the hour goes out.
function QuestSteps.WatchesHour()
	return watchesHour
end

-- The hour at the last tick, to see it change.
local lastHour

-- Each minute: a change of the local hour goes out while a visit_at step is open, so a
-- player who stands in the place as night falls gets the step.
function QuestSteps.HourTick()
	local hour = ns.Inputs.Hour()
	if lastHour and hour ~= lastHour and watchesHour then
		ns.Outbox.Add(ns.Inputs.HourChanged(time(), hour))
	end
	lastHour = hour
end

-- The reply `events_seen`: the desktop read a batch of events, so a step can be open now.
function QuestSteps.EventsSeen()
	local now = time()
	if not watchedStepWaits or (journalAskedAt and now - journalAskedAt < JOURNAL_SECONDS) then
		return
	end
	journalAskedAt = now
	ns.Journal.Request(0)
end
