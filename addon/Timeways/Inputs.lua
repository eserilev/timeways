-- The input lines of the story program (crates/story/src/input.rs). `at` is time() in
-- seconds, which the world uses as its clock.

local _, ns = ...

local Inputs = {}
ns.Inputs = Inputs

local function Present(s)
	if s and s ~= "" then
		return s
	end
end

-- The name of your own character. It names the file of your world on your computer, and
-- never reaches a model (GAMEPLAY.md 5.11).
function Inputs.Character(realm, name)
	return { type = "character_entered", realm = realm, name = name }
end

function Inputs.Zone(at, zone, subzone)
	return { type = "zone_entered", at = at, zone = zone, subzone = Present(subzone) }
end

function Inputs.Npc(at, name)
	return { type = "npc_met", at = at, name = name }
end

function Inputs.Defeated(at, name)
	return { type = "npc_defeated", at = at, name = name }
end

function Inputs.Slapped(at, name)
	return { type = "npc_slapped", at = at, name = name }
end

function Inputs.Talk(at, npc, text)
	return { type = "talk_asked", at = at, npc = npc, text = text }
end

function Inputs.Died(at, killer)
	return { type = "died", at = at, killer = killer }
end

function Inputs.Level(at, level)
	return { type = "level_reached", at = at, level = level }
end

function Inputs.Question(at, question, target)
	return { type = "lore_asked", at = at, question = question, target = Present(target) }
end

function Inputs.JournalAsked(page)
	return { type = "journal_asked", page = page }
end
