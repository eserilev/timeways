-- Checks the saved state of player tasks when the addon first reads it (GAMEPLAY.md 4.7).
-- Any addon can write the saved variables, so each task and record is checked like a
-- message from a peer. A broken one is dropped, never shown.

local _, ns = ...

local TaskSaved = {}
ns.TaskSaved = TaskSaved

local STATUSES = { offered = true, accepted = true, declined = true, cancelled = true, done = true }
local MAX_NAME = 64

local function IsNumber(value)
	return type(value) == "number" and value == value and value >= 0 and value < 2 ^ 53
end

local function IsText(value, limit, empty)
	return type(value) == "string" and #value <= limit and (empty or value ~= "") and ns.TaskWire.IsCleanText(value)
end

local function IsName(value)
	return IsText(value, MAX_NAME)
end

local function IsOptional(value, check)
	return value == nil or check(value)
end

local KINDS = {}
for _, kind in ipairs(ns.TaskWire.STEP_KINDS) do
	KINDS[kind] = true
end

local function IsStep(step)
	local count = type(step) == "table" and step.count
	local whole = IsNumber(count) and count % 1 == 0 and count >= 1 and count <= ns.TaskWire.MAX_COUNT
	return whole and KINDS[step.kind] == true and IsText(step.target, ns.TaskWire.LIMITS.target)
end

local function IsSteps(steps)
	if type(steps) ~= "table" or #steps < 1 or #steps > ns.TaskWire.MAX_STEPS then
		return false
	end
	for _, step in ipairs(steps) do
		if not IsStep(step) then
			return false
		end
	end
	return true
end

local LEVELS = { witnessed = true, unconfirmed = true, seen = true }

local function IsClaim(claim)
	local zone = type(claim) == "table" and claim.zone
	return zone
		and IsNumber(claim.at)
		and IsText(zone, ns.TaskWire.LIMITS.zone, true)
		and IsOptional(claim.near, function(near)
			return type(near) == "boolean"
		end)
		and IsOptional(claim.level, function(level)
			return LEVELS[level] == true
		end)
end

-- Keeps only the claims and counts of real steps, so a page never reads a broken one.
local function CleanSteps(task)
	local claims, progress = {}, {}
	local oldClaims = type(task.claims) == "table" and task.claims or {}
	local oldProgress = type(task.progress) == "table" and task.progress or {}
	for index = 1, #task.steps do
		claims[index] = IsClaim(oldClaims[index]) and oldClaims[index] or nil
		progress[index] = IsNumber(oldProgress[index]) and oldProgress[index] or nil
	end
	task.claims, task.progress = claims, progress
end

local function IsTask(task)
	local limits = ns.TaskWire.LIMITS
	return type(task) == "table"
		and type(task.id) == "string"
		and task.id:match("^%w+$") ~= nil
		and IsName(task.giver)
		and IsName(task.doer)
		and IsText(task.title, limits.title)
		and IsText(task.text, limits.text)
		and IsText(task.reward, limits.reward, true)
		and IsSteps(task.steps)
		and STATUSES[task.status] == true
		and IsNumber(task.sentAt)
		and IsOptional(task.answeredAt, IsNumber)
		and IsOptional(task.closedAt, IsNumber)
		and IsOptional(task.turnInAt, IsNumber)
		and IsOptional(task.place, function(place)
			return IsText(place, limits.zone, true)
		end)
end

local function CleanTasks(tasks)
	for key, task in pairs(tasks) do
		if type(key) ~= "string" or not IsTask(task) then
			tasks[key] = nil
		else
			CleanSteps(task)
		end
	end
end

local function IsItems(items)
	if type(items) ~= "table" then
		return false
	end
	for name, count in pairs(items) do
		if not IsText(name, ns.TaskWire.LIMITS.target) or not IsNumber(count) then
			return false
		end
	end
	return true
end

local RECORDS = {
	zones = function(record)
		return IsText(record.zone, ns.TaskWire.LIMITS.zone, true)
			and IsText(record.subzone, ns.TaskWire.LIMITS.zone, true)
	end,
	kills = function(record)
		return IsName(record.doer) and IsText(record.target, ns.TaskWire.LIMITS.target)
	end,
	npcs = function(record)
		return IsText(record.name, ns.TaskWire.LIMITS.target)
	end,
	near = function(record)
		return IsName(record.name)
	end,
	trades = function(record)
		return IsName(record.with)
			and IsItems(record.gave)
			and IsItems(record.got)
			and IsNumber(record.money)
			and IsNumber(record.moneyGot)
	end,
}

local function CleanRecords(data)
	for kind, check in pairs(RECORDS) do
		local kept = {}
		for _, record in ipairs(data[kind]) do
			if type(record) == "table" and IsNumber(record.at) and check(record) then
				kept[#kept + 1] = record
			end
		end
		data[kind] = kept
	end
end

local function IsStretch(stretch)
	return type(stretch) == "table" and IsNumber(stretch.from) and IsOptional(stretch.to, IsNumber)
end

local function CleanParty(party)
	for name, stretches in pairs(party) do
		local kept = {}
		for _, stretch in ipairs(type(stretches) == "table" and stretches or {}) do
			if IsStretch(stretch) then
				kept[#kept + 1] = stretch
			end
		end
		party[name] = IsName(name) and kept or nil
	end
end

-- A name maps to the time it came. A file of an older version has `true`.
local function CleanNames(names)
	for name, value in pairs(names) do
		if value == true then
			names[name] = 0
		elseif not IsName(name) or not IsNumber(value) then
			names[name] = nil
		end
	end
end

-- `data` has the shape of TaskStore.Data, with every table in place.
function TaskSaved.Clean(data)
	CleanTasks(data.given)
	CleanTasks(data.received)
	CleanRecords(data)
	CleanParty(data.party)
	CleanNames(data.blocked)
	CleanNames(data.refusedBy)
	if not IsNumber(data.nextId) or data.nextId % 1 ~= 0 then
		data.nextId = 0
	end
end
