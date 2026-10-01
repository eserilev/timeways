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

-- Your part in a task: you do it, or you gave it and witness it.
local ROLE = { DOER = "doer", GIVER = "giver" }

local function Readable(...)
	for n = 1, select("#", ...) do
		local value = select(n, ...)
		if value == nil or issecretvalue(value) then
			return false
		end
	end
	return true
end

-- The open steps, built again only after a task changes: nameplates and the mouse fire
-- many events.
local cache

-- Each open step of every task that your addon watches: { task, index, step, role }, and
-- each target by kind: { kill = { [name] = true }, ... }.
local function Build()
	local steps, targets = {}, {}
	local function Add(task, index, role)
		local step = task.steps[index]
		steps[#steps + 1] = { task = task, index = index, step = step, role = role }
		targets[step.kind] = targets[step.kind] or {}
		targets[step.kind][step.target] = true
	end
	for _, task in ipairs(ns.PlayerTasks.Doing()) do
		for index in ipairs(task.steps) do
			if not task.claims[index] then
				Add(task, index, ROLE.DOER)
			end
		end
	end
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		for index in ipairs(task.steps) do
			Add(task, index, ROLE.GIVER)
		end
	end
	return { data = ns.TaskStore.Data(), steps = steps, targets = targets }
end

local function Open()
	if not cache or cache.data ~= ns.TaskStore.Data() then
		cache = Build()
	end
	return cache
end

-- A task began, changed, or ended.
function TaskTracker.TasksChanged()
	cache = nil
end

local function OpenSteps()
	return Open().steps
end

local function IsTarget(kind, name)
	local targets = Open().targets[kind]
	return targets ~= nil and targets[name] == true
end

-- The open steps of tasks that you do, of one kind and target.
local function DoerSteps(kind, target)
	local found = {}
	for _, open in ipairs(OpenSteps()) do
		if open.role == ROLE.DOER and open.step.kind == kind and open.step.target == target then
			found[#found + 1] = open
		end
	end
	return found
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

local function CheckPlaces()
	for _, place in ipairs({ GetRealZoneText(), GetSubZoneText() }) do
		for _, open in ipairs(DoerSteps("place", place)) do
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
	for _, open in ipairs(DoerSteps("npc", name)) do
		ns.PlayerTasks.Claim(open.task, open.index)
	end
end

-- PARTY_KILL comes for a kill by you or by a member of your group, and each one counts, as
-- quest credit does in the game.
local function Killed(name)
	for _, open in ipairs(DoerSteps("kill", name)) do
		Progress(open.task, open.index, 1)
	end
end

-- You met a player of a "find a player" step when you stand next to them.
local function MetPlayer(name)
	for _, open in ipairs(DoerSteps("meet", name)) do
		ns.PlayerTasks.Claim(open.task, open.index)
	end
end

local function Brought(trade)
	for name, count in pairs(trade.gave) do
		for _, open in ipairs(DoerSteps("item", name)) do
			if open.task.giver == trade.with then
				Progress(open.task, open.index, count)
			end
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

-- A finished task with a promised reward still waits for its trade.
local function OwesReward(name)
	local data = ns.TaskStore.Data()
	for _, entry in ipairs(ns.PlayerTasks.Given()) do
		local task = entry.task
		local owed = task.status == "done" and task.reward ~= "" and not ns.TaskProof.Paid(task, data)
		if task.doer == name and owed then
			return true
		end
	end
	return false
end

local function IsPeer(name)
	if IsDoer(name) or OwesReward(name) then
		return true
	end
	for _, task in ipairs(ns.PlayerTasks.Doing()) do
		if task.giver == name then
			return true
		end
	end
	return false
end

-- Only a real change of place makes a record, so the history reaches back far.
local function RecordPlace(zone, subzone)
	local zones = ns.TaskStore.Data().zones
	local last = zones[#zones]
	if last and last.zone == zone and last.subzone == subzone then
		return
	end
	ns.TaskStore.Record("zones", { at = time(), zone = zone, subzone = subzone })
end

local function RecordZone()
	if Watching() then
		RecordPlace(GetRealZoneText(), GetSubZoneText())
	end
end

local function CloseStretch(stretches)
	local last = stretches[#stretches]
	if last and not last.to then
		last.to = time()
	end
end

local function OpenStretch(stretches)
	local last = stretches[#stretches]
	if not last or last.to then
		stretches[#stretches + 1] = { from = time() }
	end
	while #stretches > ns.TaskStore.MAX_STRETCHES do
		table.remove(stretches, 1)
	end
end

-- Stretches of party time stay only for the doers of the tasks that you keep.
local function DropStrangers(party)
	local doers = {}
	for _, entry in ipairs(ns.PlayerTasks.Given()) do
		doers[entry.task.doer] = true
	end
	for name in pairs(party) do
		if not doers[name] then
			party[name] = nil
		end
	end
end

-- Each doer in your group has an open stretch of party time. One who left has its stretch
-- closed.
function TaskTracker.RosterChanged()
	local party = ns.TaskStore.Data().party
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		party[task.doer] = party[task.doer] or {}
		if ns.TaskPeople.GroupUnit(task.doer) then
			OpenStretch(party[task.doer])
		else
			CloseStretch(party[task.doer])
		end
	end
	DropStrangers(party)
end

-- A task that you gave or that its doer took: from now on, where you are and who is in
-- your party count.
function TaskTracker.Watch()
	TaskTracker.RosterChanged()
	RecordZone()
end

-- While you are offline, your addon sees nothing: no party time, and no place.
function TaskTracker.LoggedOut()
	for _, stretches in pairs(ns.TaskStore.Data().party) do
		CloseStretch(stretches)
	end
	if Watching() then
		RecordPlace("", "")
	end
end

-- A kill in your group while the doer is in it: the doer's addon counts the same kill.
local function WitnessKill(name)
	if not IsTarget("kill", name) then
		return
	end
	local recorded = {}
	for _, task in ipairs(ns.PlayerTasks.Watching()) do
		if not recorded[task.doer] and ns.TaskPeople.GroupUnit(task.doer) then
			recorded[task.doer] = true
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
	local name = ns.Units.NpcName("npc")
	if name then
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

local function SeePlayer(unit)
	local player = ns.TaskPeople.OfUnit(unit)
	if player and ns.TaskPeople.IsNear(player) then
		MetPlayer(player)
		if IsDoer(player) then
			ns.TaskStore.Record("near", { at = time(), name = player })
		end
	end
end

-- A unit that you see: the target of a "defeat" step, an NPC that a doer talks to, or a
-- player next to you.
function TaskTracker.See(unit)
	if #OpenSteps() == 0 or not UnitExists(unit) then
		return
	end
	local guid, name = UnitGUID(unit), UnitName(unit)
	if not Readable(name) then
		return
	end
	if Readable(guid) and IsTarget("kill", name) then
		Remember(guid, name)
	end
	local npc = ns.Units.NpcName(unit)
	if npc and Watching() and IsTarget("npc", npc) then
		ns.TaskStore.Record("npcs", { at = time(), name = npc })
	end
	SeePlayer(unit)
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
	WitnessKill(name)
end

-- A trade with the other player of an open task, or with a doer who still waits for a
-- reward, from TaskTrade.
function TaskTracker.Traded(trade)
	if not trade.with or not IsPeer(trade.with) then
		return
	end
	ns.TaskStore.Record("trades", trade)
	Brought(trade)
	ns.JournalFrame.Refresh()
end

local HANDLERS = {
	PLAYER_ENTERING_WORLD = TaskTracker.Watch,
	PLAYER_LOGOUT = TaskTracker.LoggedOut,
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
	end,
	UPDATE_MOUSEOVER_UNIT = function()
		TaskTracker.See("mouseover")
	end,
	NAME_PLATE_UNIT_ADDED = TaskTracker.See,
	PARTY_KILL = TaskTracker.PartyKill,
	GROUP_ROSTER_UPDATE = TaskTracker.RosterChanged,
	TRADE_SHOW = ns.TaskTrade.Shown,
	TRADE_PLAYER_ITEM_CHANGED = ns.TaskTrade.Changed,
	TRADE_TARGET_ITEM_CHANGED = ns.TaskTrade.Changed,
	TRADE_MONEY_CHANGED = ns.TaskTrade.Changed,
	TRADE_ACCEPT_UPDATE = ns.TaskTrade.AcceptChanged,
	TRADE_CLOSED = ns.TaskTrade.Closed,
	UI_INFO_MESSAGE = ns.TaskTrade.InfoMessage,
}

local frame = CreateFrame("Frame")
-- Literal names, so the API gate of Gnomish Relay checks each one against the client.
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("PLAYER_LOGOUT")
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
frame:RegisterEvent("TRADE_PLAYER_ITEM_CHANGED")
frame:RegisterEvent("TRADE_TARGET_ITEM_CHANGED")
frame:RegisterEvent("TRADE_MONEY_CHANGED")
frame:RegisterEvent("TRADE_ACCEPT_UPDATE")
frame:RegisterEvent("TRADE_CLOSED")
frame:RegisterEvent("UI_INFO_MESSAGE")
frame:SetScript("OnEvent", function(_, event, ...)
	HANDLERS[event](...)
end)
