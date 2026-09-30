-- Kills of rares and bosses, kills for the kill steps of your tasks, and your deaths
-- (GAMEPLAY.md 3.4 and 5.13). Addons cannot read the combat log in this client, so this
-- module reads what the client does give: the units that you see, PARTY_KILL,
-- ENCOUNTER_END, and the death recap.

local _, ns = ...

local Foes = {}
ns.Foes = Foes

local NOTABLE = { rare = true, rareelite = true, worldboss = true }

-- A raid boss gives both PARTY_KILL and ENCOUNTER_END, so a name counts once in this time.
local SAME_KILL_SECONDS = 120

-- Only in memory, and never sent: the names of NPCs and players that you saw, and the rares
-- and bosses by GUID.
local npcs, players, notable, levels = {}, {}, {}, {}
local lastKill = {}
-- The creatures of the next kill step of each task in progress, and the units of them that
-- you saw and can attack, by GUID. A common mob counts only when a task hunts it.
local hunted, prey = {}, {}

-- The client can hide a value from addons. A hidden value is never compared or stored.
local function Readable(...)
	for n = 1, select("#", ...) do
		if issecretvalue(select(n, ...)) then
			return false
		end
	end
	return true
end

function Foes.See(unit)
	if not UnitExists(unit) then
		return
	end
	local guid, name = UnitGUID(unit), UnitName(unit)
	if not Readable(guid, name) or type(guid) ~= "string" or type(name) ~= "string" then
		return
	end
	-- A pet has a name that a player chose, so it counts as a player.
	if UnitIsPlayer(unit) or UnitPlayerControlled(unit) then
		players[name] = true
		return
	end
	npcs[name] = true
	levels[name] = UnitLevel(unit)
	if hunted[name] and UnitCanAttack("player", unit) then
		prey[guid] = name
	end
	if NOTABLE[UnitClassification(unit)] then
		notable[guid] = name
	end
end

function Foes.SeeTarget()
	Foes.See("target")
end

function Foes.SeeMouseover()
	Foes.See("mouseover")
end

local function Defeated(name)
	local now = time()
	if lastKill[name] and now - lastKill[name] < SAME_KILL_SECONDS then
		return
	end
	lastKill[name] = now
	ns.Outbox.Add(ns.Inputs.Defeated(now, name))
end

-- The creature of the next step of a task in progress, when it is a kill step.
local function NextKill(quest)
	if type(quest) ~= "table" or quest.status ~= "accepted" or type(quest.steps) ~= "table" then
		return nil
	end
	local done = type(quest.steps_done) == "number" and quest.steps_done or 0
	local step = quest.steps[done + 1]
	if type(step) == "table" and step.goal == "kill" and type(step.creature) == "string" then
		return step.creature
	end
end

-- The tasks of the journal say what to hunt. A unit of a creature that no task hunts now
-- is forgotten.
function Foes.Hunt(quests)
	hunted = {}
	for _, quest in ipairs(type(quests) == "table" and quests or {}) do
		local creature = NextKill(quest)
		if creature then
			hunted[creature] = true
		end
	end
	for guid, name in pairs(prey) do
		if not hunted[name] then
			prey[guid] = nil
		end
	end
end

local function CountKill(guid)
	local name = prey[guid]
	if name then
		prey[guid] = nil
		ns.Outbox.Add(ns.Inputs.Killed(time(), name))
	end
end

-- PARTY_KILL fires for a killing blow of you or your group.
function Foes.PartyKill(_, targetGUID)
	if not Readable(targetGUID) then
		return
	end
	CountKill(targetGUID)
	local name = notable[targetGUID]
	if name then
		notable[targetGUID] = nil
		Defeated(name)
	end
end

function Foes.EncounterEnd(_, name, _, _, success)
	if success == 1 and Readable(name) and type(name) == "string" then
		Defeated(name)
	end
end

-- The killing blow is the first event of the recap.
local function KillingBlow()
	if not C_DeathRecap.HasRecapEvents() then
		return nil
	end
	local blow = (C_DeathRecap.GetRecapEvents() or {})[1]
	if type(blow) == "table" and Readable(blow.sourceName, blow.hideCaster, blow.environmentalType) then
		return blow
	end
end

-- A name that was ever on a player stays out, so the name of a real player never leaves
-- the computer (5.11).
local function Killer(blow)
	if not blow or blow.hideCaster then
		return nil
	end
	local name = blow.sourceName
	if type(name) == "string" and npcs[name] and not players[name] then
		return name
	end
end

-- "Falling", "Drowning", "Lava": the world itself killed you.
local function Cause(blow)
	local cause = blow and blow.environmentalType
	if type(cause) == "string" and cause:match("^%a+$") and #cause <= 24 then
		return cause:lower()
	end
end

-- A level of -1 means a boss far above you, so it never makes a humbling death.
local function KillerLevel(killer)
	local level = killer and levels[killer]
	if type(level) == "number" and level > 0 then
		return level
	end
end

function Foes.Died()
	local blow = KillingBlow()
	local killer = Killer(blow)
	ns.Outbox.Add(ns.Inputs.Died(time(), killer, Cause(blow), KillerLevel(killer), ns.Inputs.Hour()))
end
