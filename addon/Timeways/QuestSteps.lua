-- The open steps of your side quests that the addon must watch (docs/plans/quest-variety.md
-- 10.2). The journal says which steps are open. Only the desktop moves a step, so a step
-- that opens later needs a new journal.

local _, ns = ...

local QuestSteps = {}
ns.QuestSteps = QuestSteps

-- The goals that the addon watches itself, while their steps are open.
local WATCHED = { kill = true }

-- The creatures of the open kill steps, as a set.
local hunted = {}
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
end

-- From each journal: the open steps that the addon watches, and whether one waits.
function QuestSteps.Read(quests)
	hunted, watchedStepWaits = {}, false
	for _, quest in ipairs(type(quests) == "table" and quests or {}) do
		if Accepted(quest) then
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

-- The reply `events_seen`: the desktop read a batch of events, so a step can be open now.
function QuestSteps.EventsSeen()
	local now = time()
	if not watchedStepWaits or (journalAskedAt and now - journalAskedAt < JOURNAL_SECONDS) then
		return
	end
	journalAskedAt = now
	ns.Journal.Request(0)
end
