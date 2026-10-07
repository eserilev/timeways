-- Kills of rares and bosses, kills for the kill steps of your tasks, the first kill of each
-- other foe in a session, and your deaths (GAMEPLAY.md 3.4 and 5.13). Addons cannot read the combat log in this client, so this
-- module reads what the client does give: the units that you see, PARTY_KILL,
-- ENCOUNTER_END, and the death recap.

local _, ns = ...

local Foes = {}
ns.Foes = Foes

local NOTABLE = { rare = true, rareelite = true, worldboss = true }

-- A raid boss gives both PARTY_KILL and ENCOUNTER_END, so a name counts once in this time.
local SAME_KILL_SECONDS = 120

-- Only in memory, and never sent: the names of players that you saw, the levels of NPCs,
-- and the rares and bosses by GUID, each with its classification.
local players, notable, levels = {}, {}, {}
local lastKill = {}
-- The creatures of the open kill steps of each task in progress, and the units of them that
-- you saw and can attack, by GUID. A common mob counts only when a task hunts it.
local hunted, prey = {}, {}
-- The other foes that you saw and can attack, by GUID, and the names that went out as a
-- kill in this session. The story program keeps only a kill of a foe that its lore waits
-- for, such as Mor'Ladim, who is neither rare nor a boss.
local foes, killedOnce = {}, {}
local foeCount = 0
-- Nameplates show many units, so the table starts again when it holds this many.
local MAX_FOES = 300

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
	levels[name] = UnitLevel(unit)
	local attackable = UnitCanAttack("player", unit)
	if hunted[name] and attackable then
		prey[guid] = name
	end
	if attackable and not foes[guid] then
		if foeCount >= MAX_FOES then
			foes, foeCount = {}, 0
		end
		foes[guid] = name
		foeCount = foeCount + 1
	end
	local kind = UnitClassification(unit)
	if NOTABLE[kind] then
		notable[guid] = { name = name, kind = kind }
	end
end

function Foes.SeeTarget()
	Foes.See("target")
end

function Foes.SeeMouseover()
	Foes.See("mouseover")
end

local function Defeated(name, kind)
	local now = time()
	if lastKill[name] and now - lastKill[name] < SAME_KILL_SECONDS then
		return
	end
	lastKill[name] = now
	ns.Outbox.Add(ns.Inputs.Defeated(now, name, kind))
end

-- The creatures of the open kill steps (`QuestSteps`). A unit of a creature that no task
-- hunts now is forgotten, and the unit that you target or hover now counts at once.
function Foes.Hunt(creatures)
	hunted = creatures
	for guid, name in pairs(prey) do
		if not hunted[name] then
			prey[guid] = nil
		end
	end
	Foes.See("target")
	Foes.See("mouseover")
end

-- A hunted unit counts each time. Any other foe goes out once for each name in a session,
-- but never a rare, which goes out as a defeat, and never inside an instance, where a boss
-- goes out as a defeat by its encounter.
local function CountKill(guid)
	if type(guid) ~= "string" then
		return
	end
	local hunt = prey[guid]
	if hunt then
		prey[guid], foes[guid] = nil, nil
		ns.Outbox.Add(ns.Inputs.Killed(time(), hunt))
		return
	end
	local name = foes[guid]
	foes[guid] = nil
	if name and not killedOnce[name] and not notable[guid] and not IsInInstance() then
		killedOnce[name] = true
		ns.Outbox.Add(ns.Inputs.Killed(time(), name))
	end
end

-- PARTY_KILL fires for a killing blow of you or your group.
function Foes.PartyKill(_, targetGUID)
	if not Readable(targetGUID) then
		return
	end
	CountKill(targetGUID)
	local foe = notable[targetGUID]
	if foe then
		notable[targetGUID] = nil
		Defeated(foe.name, foe.kind)
	end
end

function Foes.EncounterEnd(_, name, _, _, success)
	if success == 1 and Readable(name) and type(name) == "string" then
		Defeated(name, "boss")
	end
end

-- The killing blow is the first event of the recap.
local function KillingBlow()
	if not C_DeathRecap.HasRecapEvents() then
		return nil
	end
	local blow = (C_DeathRecap.GetRecapEvents() or {})[1]
	if
		type(blow) == "table" and Readable(blow.sourceName, blow.sourceGUID, blow.hideCaster, blow.environmentalType)
	then
		return blow
	end
end

local function IsNpcGuid(guid)
	return type(guid) == "string" and (guid:match("^Creature%-") or guid:match("^Vehicle%-")) ~= nil
end

-- Only a killer with the GUID of an NPC is named, because a name alone can be a player's.
-- A name that was ever on a player stays out too. So the name of a real player never
-- leaves the computer (5.11).
local function Killer(blow)
	if not blow or blow.hideCaster or not IsNpcGuid(blow.sourceGUID) then
		return nil
	end
	local name = blow.sourceName
	if type(name) == "string" and not players[name] then
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
