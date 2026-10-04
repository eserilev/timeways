-- The pins of one task on the journal map (GAMEPLAY.md 3.6): the person who gave it, and
-- each step whose place or person has a position in the journal.

local _, ns = ...

local TaskPins = {}
ns.TaskPins = TaskPins

local function Whole(value, low, high)
	return type(value) == "number" and value % 1 == 0 and value >= low and value <= high
end

-- A position from the desktop, or nil when it is not one.
local function Spot(entry)
	local spot = type(entry) == "table" and entry.spot
	if type(spot) ~= "table" or not Whole(spot.map, 1, math.huge) then
		return nil
	end
	if not (Whole(spot.x, 0, 1000) and Whole(spot.y, 0, 1000)) then
		return nil
	end
	return spot
end

-- The first entry of the list with this name, which the journal sends as a list of tables.
local function Named(list, name)
	for _, entry in ipairs(type(list) == "table" and list or {}) do
		if type(entry) == "table" and entry.name == name then
			return entry
		end
	end
end

-- The goals whose step names an NPC.
local PERSON_GOALS = { meet = true, talk = true, carry = true, slap = true }

-- A place can be a zone or a subzone. A step names it as the game does. An emote names
-- an NPC or a place.
local function StepSpot(journal, step)
	if step.goal == "visit" or (step.goal == "emote" and type(step.place) == "string") then
		return Spot(Named(journal.places, step.place))
	end
	if PERSON_GOALS[step.goal] or step.goal == "emote" then
		return Spot(Named(journal.people, step.npc))
	end
end

local function Pin(spot, fields)
	fields.map, fields.x, fields.y = spot.map, spot.x / 1000, spot.y / 1000
	return fields
end

-- Each pin is { kind = "giver" or "step", map, x, y }, with x and y from 0 to 1. A step pin
-- has its number and `done`. A giver pin has `offered` while the task is an offer.
function TaskPins.For(journal, quest)
	local pins = {}
	local giver = Spot(Named(journal.people, quest.giver))
	if giver then
		pins[#pins + 1] = Pin(giver, { kind = "giver", offered = quest.status == "offered" })
	end
	for n, step in ipairs(type(quest.steps) == "table" and quest.steps or {}) do
		local spot = type(step) == "table" and StepSpot(journal, step)
		if spot then
			pins[#pins + 1] = Pin(spot, { kind = "step", number = n, done = step.state == "done" })
		end
	end
	return pins
end

-- The map of the first pin: the giver's, when the journal knows it.
function TaskPins.MapOf(pins)
	return pins[1] and pins[1].map
end
