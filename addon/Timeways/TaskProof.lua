-- How sure the giver can be of each step of a task (GAMEPLAY.md 4.7). The addon of the doer
-- runs on the doer's computer, so its word is a claim. The giver's own addon is the witness.
--   "witnessed": the giver's addon saw it too.
--   "seen": only the doer's addon recorded it.
--   "unconfirmed": the giver's addon was in a place to see it, and saw nothing.
--   "missing": no claim yet.

local _, ns = ...

local TaskProof = {}
ns.TaskProof = TaskProof

-- The clocks of two computers differ, and a kill and its report come a moment apart.
TaskProof.SAME_TIME = 120

-- `intervals` is a list of { from, to }, with no `to` while the party lasts.
function TaskProof.InParty(intervals, at)
	for _, interval in ipairs(intervals or {}) do
		if interval.from <= at and (not interval.to or at <= interval.to) then
			return true
		end
	end
	return false
end

-- The last place in `zones` at or before the time: { at, zone, subzone }, oldest first.
function TaskProof.PlaceAt(zones, at)
	local place
	for _, entry in ipairs(zones or {}) do
		if entry.at > at then
			break
		end
		place = entry
	end
	return place
end

local function Near(a, b)
	return math.abs(a - b) <= TaskProof.SAME_TIME
end

local function IsPlace(place, name)
	return place ~= nil and (place.zone == name or place.subzone == name)
end

-- Each checker gets the step, the claim, the task, and the records of the giver.
local WITNESSES = {
	place = function(step, claim, task, records)
		local inParty = TaskProof.InParty(records.party[task.doer], claim.at)
		return inParty and IsPlace(TaskProof.PlaceAt(records.zones, claim.at), step.target)
	end,
	npc = function(step, claim, task, records)
		if not TaskProof.InParty(records.party[task.doer], claim.at) then
			return false
		end
		for _, seen in ipairs(records.npcs) do
			if seen.name == step.target and Near(seen.at, claim.at) then
				return true
			end
		end
		return false
	end,
	kill = function(step, _, task, records)
		local count = 0
		for _, kill in ipairs(records.kills) do
			if kill.doer == task.doer and kill.target == step.target and kill.at >= task.sentAt then
				count = count + 1
			end
		end
		return count >= step.count
	end,
	-- Only the giver can witness a meeting with the giver.
	meet = function(step, claim, task, records)
		if step.target ~= task.giver then
			return false
		end
		for _, near in ipairs(records.near) do
			if near.name == task.doer and Near(near.at, claim.at) then
				return true
			end
		end
		return false
	end,
	item = function(step, _, task, records)
		local count = 0
		for _, trade in ipairs(records.trades) do
			if trade.with == task.doer and trade.at >= task.sentAt then
				count = count + ((trade.got or {})[step.target] or 0)
			end
		end
		return count >= step.count
	end,
	-- Only the doer's word says that a step that the game can't see is done.
	other = function()
		return false
	end,
}

function TaskProof.Witnessed(step, claim, task, records)
	return WITNESSES[step.kind](step, claim, task, records) == true
end

-- A trade happens face to face, so the giver's addon always sees an item that it got. For
-- the rest, the giver's addon was in a place to see it when the doer stood close to the
-- giver as the step came (`claim.near`, from the giver's own range check).
local function InSight(step, claim)
	if step.kind == "other" then
		return false
	end
	return step.kind == "item" or claim.near == true
end

function TaskProof.Level(step, claim, task, records)
	if not claim then
		return "missing"
	end
	if TaskProof.Witnessed(step, claim, task, records) then
		return "witnessed"
	end
	if InSight(step, claim) then
		return "unconfirmed"
	end
	return "seen"
end

-- The level that the giver's addon kept when the claim came, so records that the store cut
-- since never change it. A witness that came later still raises it.
function TaskProof.Settled(step, claim, task, records)
	local now = TaskProof.Level(step, claim, task, records)
	if now == "witnessed" or not claim or not claim.level then
		return now
	end
	return claim.level
end

-- The reward went over when the giver gave the doer anything in a trade after the task.
function TaskProof.Paid(task, records)
	for _, trade in ipairs(records.trades) do
		local gave = trade.gave or {}
		local something = (trade.money or 0) > 0 or next(gave) ~= nil
		if trade.with == task.doer and trade.at >= task.sentAt and something then
			return true
		end
	end
	return false
end
