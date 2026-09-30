-- The game events of player tasks (GAMEPLAY.md 4.7). For a task that you got, your addon
-- sees each step and tells the giver. For a task that you gave, your addon keeps what it
-- saw itself: where you were, who was in your party, and what happened near you.

local _, ns = ...

local TaskTracker = {}
ns.TaskTracker = TaskTracker

-- Names of units by GUID, only for the targets of open "defeat" steps. PARTY_KILL gives
-- only a GUID.
local MAX_GUIDS = 200
local guids, guidCount = {}, 0

local function Readable(...)
	for n = 1, select("#", ...) do
		local value = select(n, ...)
		if value == nil or issecretvalue(value) then
			return false
		end
	end
	return true
end

-- Each open step of every task that your addon watches: { task, index, step, doing }.
-- `doing` is true for a task that you got, and false for one that you gave.
local function OpenSteps()
	local steps = {}
	for _, task in ipairs(ns.PlayerTasks.Doing()) do
		for index, step in ipairs(task.steps) do
			if not task.claims[index] then
				steps[#steps + 1] = { task = task, index = index, step = step, doing = true }
			end
		end
	end
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		for index, step in ipairs(task.steps) do
			steps[#steps + 1] = { task = task, index = index, step = step, doing = false }
		end
	end
	return steps
end

local function IsTarget(kind, name)
	for _, open in ipairs(OpenSteps()) do
		if open.step.kind == kind and open.step.target == name then
			return true
		end
	end
	return false
end

-- A step with a count moves on by `amount`, and is done at its count.
local function Progress(task, index, amount)
	task.progress = task.progress or {}
	task.progress[index] = (task.progress[index] or 0) + amount
	if task.progress[index] >= task.steps[index].count then
		ns.PlayerTasks.Claim(task, index)
	end
end

-- Doing: each step of a task that you got, from what your addon sees ---------------------

local function IsHere(target)
	return target == GetRealZoneText() or target == GetSubZoneText()
end

local function CheckPlaces()
	for _, open in ipairs(OpenSteps()) do
		if open.doing and open.step.kind == "place" and IsHere(open.step.target) then
			ns.PlayerTasks.Claim(open.task, open.index)
		end
	end
end

-- A place where you stand when you accept counts at once: the zone changes only when you
-- move.
function TaskTracker.Accepted()
	CheckPlaces()
end

local function Talked(name)
	for _, open in ipairs(OpenSteps()) do
		if open.doing and open.step.kind == "npc" and open.step.target == name then
			ns.PlayerTasks.Claim(open.task, open.index)
		end
	end
end

local function Killed(name)
	for _, open in ipairs(OpenSteps()) do
		if open.doing and open.step.kind == "kill" and open.step.target == name then
			Progress(open.task, open.index, 1)
		end
	end
end

-- You met a player of a "find a player" step when you stand next to them.
local function MetPlayer(name)
	for _, open in ipairs(OpenSteps()) do
		if open.doing and open.step.kind == "meet" and open.step.target == name then
			ns.PlayerTasks.Claim(open.task, open.index)
		end
	end
end

local function Brought(trade)
	for _, open in ipairs(OpenSteps()) do
		local item = open.step.kind == "item" and open.task.giver == trade.with
		if open.doing and item and trade.gave[open.step.target] then
			Progress(open.task, open.index, trade.gave[open.step.target])
		end
	end
end

-- Witnessing: what your addon sees for a task that you gave --------------------------------

local function Watching()
	return #ns.PlayerTasks.Watching() > 0
end

local function IsDoer(name)
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		if task.doer == name then
			return true
		end
	end
	return false
end

local function IsPeer(name)
	if IsDoer(name) then
		return true
	end
	for _, task in ipairs(ns.PlayerTasks.Doing()) do
		if task.giver == name then
			return true
		end
	end
	return false
end

local function RecordZone()
	if Watching() then
		ns.TaskStore.Record("zones", { at = time(), zone = GetRealZoneText(), subzone = GetSubZoneText() })
	end
end

-- Each doer in your group has an open stretch of party time. One who left has its stretch
-- closed.
function TaskTracker.RosterChanged()
	local party = ns.TaskStore.Data().party
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		local stretches = party[task.doer] or {}
		party[task.doer] = stretches
		local last = stretches[#stretches]
		local open = last and not last.to
		local inGroup = ns.TaskPeople.GroupUnit(task.doer) ~= nil
		if inGroup and not open then
			stretches[#stretches + 1] = { from = time() }
		elseif open and not inGroup then
			last.to = time()
		end
	end
end

local function WitnessKill(attackerGUID, name)
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		local unit = ns.TaskPeople.GroupUnit(task.doer)
		if unit and UnitGUID(unit) == attackerGUID and IsTarget("kill", name) then
			ns.TaskStore.Record("kills", { at = time(), doer = task.doer, target = name })
		end
	end
end

-- Events ---------------------------------------------------------------------------------

function TaskTracker.Zone()
	CheckPlaces()
	RecordZone()
end

-- The NPC of a talk window. A player who shares a quest is the "npc" unit too, and never
-- counts.
function TaskTracker.Talk()
	local name = UnitName("npc")
	if Readable(name) and not UnitIsPlayer("npc") then
		Talked(name)
	end
end

local function Remember(guid, name)
	if guids[guid] then
		return
	end
	if guidCount >= MAX_GUIDS then
		guids, guidCount = {}, 0
	end
	guids[guid], guidCount = name, guidCount + 1
end

-- A unit that you see: the target of a "defeat" step, an NPC that a doer talks to, or a
-- player next to you.
function TaskTracker.See(unit)
	if not UnitExists(unit) or #OpenSteps() == 0 then
		return
	end
	local guid, name = UnitGUID(unit), UnitName(unit)
	if not Readable(name) then
		return
	end
	if Readable(guid) and IsTarget("kill", name) then
		Remember(guid, name)
	end
	if Watching() and IsTarget("npc", name) and not UnitIsPlayer(unit) then
		ns.TaskStore.Record("npcs", { at = time(), name = name })
	end
	local player = ns.TaskPeople.OfUnit(unit)
	if player and ns.TaskPeople.IsNear(player) then
		MetPlayer(player)
		if IsDoer(player) then
			ns.TaskStore.Record("near", { at = time(), name = player })
		end
	end
end

function TaskTracker.PartyKill(attackerGUID, targetGUID)
	if not Readable(attackerGUID, targetGUID) then
		return
	end
	local name = guids[targetGUID]
	if not name then
		return
	end
	Killed(name)
	WitnessKill(attackerGUID, name)
end

-- A trade with the other player of an open task, from TaskTrade.
function TaskTracker.Traded(trade)
	if not trade.with or not IsPeer(trade.with) then
		return
	end
	ns.TaskStore.Record("trades", trade)
	Brought(trade)
	ns.JournalFrame.Refresh()
end

local HANDLERS = {
	PLAYER_ENTERING_WORLD = function()
		TaskTracker.RosterChanged()
		RecordZone()
	end,
	ZONE_CHANGED_NEW_AREA = TaskTracker.Zone,
	ZONE_CHANGED = TaskTracker.Zone,
	ZONE_CHANGED_INDOORS = TaskTracker.Zone,
	GOSSIP_SHOW = TaskTracker.Talk,
	QUEST_GREETING = TaskTracker.Talk,
	QUEST_DETAIL = TaskTracker.Talk,
	QUEST_PROGRESS = TaskTracker.Talk,
	QUEST_COMPLETE = TaskTracker.Talk,
	PLAYER_TARGET_CHANGED = function()
		TaskTracker.See("target")
		ns.TaskForm.TargetChanged()
	end,
	UPDATE_MOUSEOVER_UNIT = function()
		TaskTracker.See("mouseover")
	end,
	NAME_PLATE_UNIT_ADDED = TaskTracker.See,
	PARTY_KILL = TaskTracker.PartyKill,
	GROUP_ROSTER_UPDATE = TaskTracker.RosterChanged,
	TRADE_SHOW = ns.TaskTrade.Shown,
	TRADE_ACCEPT_UPDATE = ns.TaskTrade.AcceptChanged,
	TRADE_CLOSED = ns.TaskTrade.Closed,
}

local frame = CreateFrame("Frame")
-- Literal names, so the API gate of Gnomish Relay checks each one against the client.
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("GOSSIP_SHOW")
frame:RegisterEvent("QUEST_GREETING")
frame:RegisterEvent("QUEST_DETAIL")
frame:RegisterEvent("QUEST_PROGRESS")
frame:RegisterEvent("QUEST_COMPLETE")
frame:RegisterEvent("PLAYER_TARGET_CHANGED")
frame:RegisterEvent("UPDATE_MOUSEOVER_UNIT")
frame:RegisterEvent("NAME_PLATE_UNIT_ADDED")
frame:RegisterEvent("PARTY_KILL")
frame:RegisterEvent("GROUP_ROSTER_UPDATE")
frame:RegisterEvent("TRADE_SHOW")
frame:RegisterEvent("TRADE_ACCEPT_UPDATE")
frame:RegisterEvent("TRADE_CLOSED")
frame:SetScript("OnEvent", function(_, event, ...)
	HANDLERS[event](...)
end)
