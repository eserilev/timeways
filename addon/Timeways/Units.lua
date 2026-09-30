-- Which units are NPCs. A player, and a pet that a player named, never count (GAMEPLAY.md
-- 5.11), and a name that the client hides is never read.

local _, ns = ...

local Units = {}
ns.Units = Units

-- No UnitExists: the name alone says that the unit is there, so a quest object such as a
-- Wanted poster counts too.
function Units.NpcName(unit)
	local name = UnitName(unit)
	if type(name) ~= "string" or issecretvalue(name) then
		return nil
	end
	if UnitIsPlayer(unit) or UnitPlayerControlled(unit) then
		return nil
	end
	return name
end

-- The NPC id of the GUID "Creature-0-1-2-3-<npc id>-<spawn>". A player, a pet, and a
-- hidden GUID give none.
function Units.NpcId(unit)
	local guid = UnitGUID(unit)
	if issecretvalue(guid) or type(guid) ~= "string" then
		return nil
	end
	return guid:match("^Creature%-%d+%-%d+%-%d+%-%d+%-(%d+)%-")
end

-- An NPC that you cannot attack: someone to talk to, or to ask for a task. A bat or a boar
-- that you can attack never counts, so it never becomes someone that you met.
function Units.FriendlyNpcName(unit)
	local name = Units.NpcName(unit)
	if name and not UnitCanAttack("player", unit) then
		return name
	end
end
