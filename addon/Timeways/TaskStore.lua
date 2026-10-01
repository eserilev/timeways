-- The saved state of player tasks, for each character (GAMEPLAY.md 4.7): the tasks that
-- you gave, the tasks that you wrote and did not send yet, the tasks that you got, the players that you blocked, and what your addon saw.
-- Other addons can read saved variables, so this holds only what the two players already
-- share in the game: the task, the names, and times. It never holds a key.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local TaskStore = {}
ns.TaskStore = TaskStore

local GLOBAL = "TimewaysTasks"

-- The newest records of each kind stay. A task lasts days, not months. The zones change
-- most often.
local MAX_RECORDS = 100
local MAX_ZONES = 300
local MAX_DONE = 50
TaskStore.MAX_STRETCHES = MAX_RECORDS

local RECORD_KINDS = { "zones", "kills", "npcs", "near", "trades" }

local function Table(value)
	return type(value) == "table" and value or {}
end

-- The table that TaskSaved checked. The check runs once for each table that the game loads.
local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
function TaskStore.Data()
	local data = Table(_G[GLOBAL])
	_G[GLOBAL] = data
	data.given = Table(data.given)
	data.received = Table(data.received)
	data.drafts = Table(data.drafts)
	data.blocked = Table(data.blocked)
	data.refusedBy = Table(data.refusedBy)
	data.party = Table(data.party)
	for _, kind in ipairs(RECORD_KINDS) do
		data[kind] = Table(data[kind])
	end
	data.nextId = type(data.nextId) == "number" and data.nextId or 0
	if checked ~= data then
		ns.TaskSaved.Clean(data)
		checked = data
	end
	return data
end

function TaskStore.Record(kind, record)
	local list = TaskStore.Data()[kind]
	list[#list + 1] = record
	local limit = kind == "zones" and MAX_ZONES or MAX_RECORDS
	while #list > limit do
		table.remove(list, 1)
	end
end

-- A new id for a task that you give: the time and a counter, in base 36.
function TaskStore.NewId(now)
	local data = TaskStore.Data()
	data.nextId = data.nextId % 1296 + 1
	local digits, value = {}, now * 1296 + data.nextId
	repeat
		local digit = value % 36
		table.insert(digits, 1, ("0123456789abcdefghijklmnopqrstuvwxyz"):sub(digit + 1, digit + 1))
		value = math.floor(value / 36)
	until value == 0
	return table.concat(digits)
end

-- A set of names, such as the blocked players, keeps its newest names: { [name] = time }.
function TaskStore.AddName(names, name)
	names[name] = time()
	local count, oldest = 0, nil
	for known, at in pairs(names) do
		count = count + 1
		if not oldest or at < names[oldest] then
			oldest = known
		end
	end
	if count > MAX_RECORDS then
		names[oldest] = nil
	end
end

function TaskStore.ReceivedKey(giver, id)
	return giver .. "/" .. id
end

local FINISHED = { done = true, declined = true, cancelled = true }

function TaskStore.IsOpen(task)
	return not FINISHED[task.status]
end

function TaskStore.CountOpen(tasks)
	local count = 0
	for _, task in pairs(tasks) do
		if TaskStore.IsOpen(task) then
			count = count + 1
		end
	end
	return count
end

local function Finished(tasks)
	local finished = {}
	for key, task in pairs(tasks) do
		if not TaskStore.IsOpen(task) then
			finished[#finished + 1] = { key = key, at = task.closedAt or 0 }
		end
	end
	table.sort(finished, function(a, b)
		return a.at < b.at
	end)
	return finished
end

-- The oldest finished tasks go first, so the file stays small.
function TaskStore.Trim(tasks)
	local finished = Finished(tasks)
	for n = 1, #finished - MAX_DONE do
		tasks[finished[n].key] = nil
	end
end

-- The tasks of a map as a list, newest first. A task that you did not send yet has only
-- the time that you saved it.
function TaskStore.List(tasks)
	local list = {}
	for key, task in pairs(tasks) do
		list[#list + 1] = { key = key, task = task }
	end
	table.sort(list, function(a, b)
		local left, right = a.task.sentAt or a.task.savedAt or 0, b.task.sentAt or b.task.savedAt or 0
		if left ~= right then
			return left > right
		end
		return a.key < b.key
	end)
	return list
end
