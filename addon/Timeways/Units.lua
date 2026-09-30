-- Which units are NPCs. A player, and a pet that a player named, never count (GAMEPLAY.md
-- 5.11), and a name that the client hides is never read.

local _, ns = ...

local Units = {}
ns.Units = Units

function Units.NpcName(unit)
	if not UnitExists(unit) or UnitIsPlayer(unit) or UnitPlayerControlled(unit) then
		return nil
	end
	local name = UnitName(unit)
	if type(name) == "string" and not issecretvalue(name) then
		return name
	end
end

-- An NPC that you cannot attack: someone to talk to, or to ask for a task. A bat or a boar
-- that you can attack never counts, so it never becomes someone that you met.
function Units.FriendlyNpcName(unit)
	local name = Units.NpcName(unit)
	if name and not UnitCanAttack("player", unit) then
		return name
	end
end
