-- Kills of rares and bosses, and your deaths (GAMEPLAY.md 5.13). Addons cannot read the
-- combat log in this client, so this module reads what the client does give: the units
-- that you see, PARTY_KILL, ENCOUNTER_END, and the death recap.

local _, ns = ...

local Foes = {}
ns.Foes = Foes

local NOTABLE = { rare = true, rareelite = true, worldboss = true }

-- A raid boss gives both PARTY_KILL and ENCOUNTER_END, so a name counts once in this time.
local SAME_KILL_SECONDS = 120

-- Only in memory, and never sent: the names of NPCs and players that you saw, and the rares
-- and bosses by GUID.
local npcs, players, notable = {}, {}, {}
local lastKill = {}

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
	if UnitIsPlayer(unit) then
		players[name] = true
		return
	end
	npcs[name] = true
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

-- PARTY_KILL fires for a killing blow of you or your group.
function Foes.PartyKill(_, targetGUID)
	if not Readable(targetGUID) then
		return
	end
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

-- The killing blow is the first event of the recap. A name that was ever on a player
-- stays out, so the name of a real player never leaves the computer (5.11).
local function Killer()
	if not C_DeathRecap.HasRecapEvents() then
		return nil
	end
	local blow = (C_DeathRecap.GetRecapEvents() or {})[1]
	if type(blow) ~= "table" or not Readable(blow.sourceName, blow.hideCaster) or blow.hideCaster then
		return nil
	end
	local name = blow.sourceName
	if type(name) == "string" and npcs[name] and not players[name] then
		return name
	end
end

function Foes.Died()
	ns.Outbox.Add(ns.Inputs.Died(time(), Killer()))
end
