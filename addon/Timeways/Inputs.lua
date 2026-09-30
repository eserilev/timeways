-- The input lines of the story program (crates/story/src/input.rs). `at` is time() in
-- seconds, which the world uses as its clock.

local _, ns = ...

local Inputs = {}
ns.Inputs = Inputs

-- The local hour of the player, from 0 to 23, for "a dance in Goldshire at 3 AM".
function Inputs.Hour()
	return tonumber(date("%H", time()))
end

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

-- `kind` is "party" for a dungeon, or "raid".
function Inputs.Instance(at, zone, kind)
	return { type = "instance_entered", at = at, zone = zone, kind = kind }
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

function Inputs.QuestAsked(at, npc)
	return { type = "quest_asked", at = at, npc = npc }
end

-- With no number, the answer takes the newest offer.
function Inputs.QuestAccepted(at, number)
	return { type = "quest_accepted", at = at, number = number }
end

function Inputs.QuestDeclined(at, number)
	return { type = "quest_declined", at = at, number = number }
end

-- `kind` is "class" for a quest of your class, and "normal" for any other.
function Inputs.GameQuestAccepted(at, title, kind)
	return { type = "game_quest_accepted", at = at, title = title, kind = kind }
end

function Inputs.QuestMarked(at, quest, mark)
	return { type = "quest_marked", at = at, quest = quest, mark = mark }
end

function Inputs.GameQuestDone(at, title, kind)
	return { type = "game_quest_done", at = at, title = title, kind = kind }
end

function Inputs.QuestAbandoned(at, number)
	return { type = "quest_abandoned", at = at, number = number }
end

function Inputs.Died(at, killer, cause, killerLevel, hour)
	return { type = "died", at = at, killer = killer, cause = cause, killer_level = killerLevel, hour = hour }
end

function Inputs.HeroSet(at, field, text)
	return { type = "hero_set", at = at, field = field, text = text }
end

function Inputs.HeroAdded(at, text, npc)
	return { type = "hero_added", at = at, text = text, npc = npc }
end

function Inputs.HeroRemoved(at, number)
	return { type = "hero_removed", at = at, number = number }
end

function Inputs.Emote(at, emote, target, hour)
	return { type = "emote_done", at = at, emote = emote, target = target, hour = hour }
end

function Inputs.Level(at, level)
	return { type = "level_reached", at = at, level = level }
end

function Inputs.Seen(at, kind, title, npc, zone, text)
	return { type = "text_seen", at = at, kind = kind, title = title, npc = npc, zone = zone, text = text }
end

function Inputs.Question(at, question, target)
	return { type = "lore_asked", at = at, question = question, target = Present(target) }
end

function Inputs.JournalAsked(page)
	return { type = "journal_asked", page = page }
end
