-- The NPCs that you see by a hover or a target (GAMEPLAY.md 3.4). A task names only an NPC
-- that you saw or met, so each NPC goes out once in a session: its name, whether you can
-- attack it, and its creature type.

local _, ns = ...

local Sightings = {}
ns.Sightings = Sightings

-- The creature types by their id in the client, in English, so the story reads the same
-- words in every language of the client.
local CREATURES = {
	[1] = "beast",
	[2] = "dragonkin",
	[3] = "demon",
	[4] = "elemental",
	[5] = "giant",
	[6] = "undead",
	[7] = "humanoid",
	[8] = "critter",
	[9] = "mechanical",
}

-- The NPC ids that went out in this session. Only in memory.
local sent = {}

local function Creature(unit)
	local _, id = UnitCreatureType(unit)
	if not issecretvalue(id) then
		return CREATURES[id]
	end
end

local function Reaction(unit)
	local attackable = UnitCanAttack("player", unit)
	if issecretvalue(attackable) then
		return nil
	end
	return attackable and "hostile" or "friendly"
end

function Sightings.See(unit)
	local name = ns.Units.NpcName(unit)
	local id = name and ns.Units.NpcId(unit)
	local reaction = id and Reaction(unit)
	if not reaction or sent[id] then
		return
	end
	sent[id] = true
	ns.Outbox.Add(ns.Inputs.NpcSeen(time(), name, reaction, Creature(unit)))
end

function Sightings.SeeTarget()
	Sightings.See("target")
end

function Sightings.SeeMouseover()
	Sightings.See("mouseover")
end
