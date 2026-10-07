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

-- The file tokens of the race and the class, which no language changes: "Scourge",
-- "WARLOCK". The narrator calls you by them (GAMEPLAY.md 3.2.1).
function Inputs.Described(at, race, class)
	return { type = "character_described", at = at, race = race, class = class }
end

-- The past of the character before Timeways saw it (GAMEPLAY.md 3.3): `past` holds the
-- fields of the line.
function Inputs.Past(at, past)
	past.type = "past_read"
	past.at = at
	return past
end

-- Another player whom a text names, with the file tokens of the race and the class. The
-- story program keeps them for the card of the player that a model reads (GAMEPLAY.md
-- 5.11).
function Inputs.PlayerDescribed(at, name, race, class)
	return { type = "player_described", at = at, name = name, race = race, class = class }
end

-- `spot` is where the player stands, from `Position.Here`, or nil. `taxi` is "yes" on a
-- flight path: a place seen from the air is no visit.
function Inputs.Zone(at, zone, subzone, spot, taxi)
	return {
		type = "zone_entered",
		at = at,
		zone = zone,
		subzone = Present(subzone),
		spot = spot,
		hour = Inputs.Hour(),
		taxi = taxi,
	}
end

-- The local hour changed while a time-of-day step is open.
function Inputs.HourChanged(at, hour)
	return { type = "hour_changed", at = at, hour = hour }
end

-- `kind` is "party" for a dungeon, "raid", or "pvp" for a battleground.
function Inputs.Instance(at, zone, kind)
	return { type = "instance_entered", at = at, zone = zone, kind = kind }
end

function Inputs.Npc(at, name, spot)
	return { type = "npc_met", at = at, name = name, spot = spot }
end

-- `reaction` is "hostile" or "friendly". `creature` is an English creature type, or nil.
function Inputs.NpcSeen(at, name, reaction, creature)
	return { type = "npc_seen", at = at, name = name, reaction = reaction, creature = creature }
end

function Inputs.Killed(at, name)
	return { type = "npc_killed", at = at, name = name }
end

-- `kind` is the classification of the game ("rare", "rareelite", "worldboss"), "boss" for
-- the boss of an encounter, or nil.
function Inputs.Defeated(at, name, kind)
	return { type = "npc_defeated", at = at, name = name, kind = kind }
end

-- The player's own words for a chapter, a tale, or the summary (docs/plans/chapters.md 11).
-- `entry` is { kind, first }. `text` is "keep", "replace", or "narrator". Each name of a
-- player is marked: "{Ada}". An empty table of Lua goes out as a JSON object, so a restore
-- sends no paragraphs at all.
function Inputs.EntryEdited(at, entry, title, text, paragraphs)
	return {
		type = "entry_edited",
		at = at,
		entry = { kind = entry.kind, first = entry.first },
		title = title,
		text = text,
		paragraphs = #paragraphs > 0 and paragraphs or nil,
	}
end

-- You won a battle in this battleground.
function Inputs.BgWon(at, zone)
	return { type = "bg_won", at = at, zone = zone }
end

-- The PvP rank, 0 for none.
function Inputs.PvpRank(at, rank)
	return { type = "pvp_rank", at = at, rank = rank }
end

-- `resting` is "yes" or "no": JSON here has no booleans.
function Inputs.RestChanged(at, resting)
	return { type = "rest_changed", at = at, resting = resting }
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

-- `speed` is the run speed on the mount, in percent of a run on foot, or nil.
function Inputs.MountRidden(at, mount, speed)
	return { type = "mount_ridden", at = at, mount = mount, speed = speed }
end

-- `item` and `before` are { item, quality, level } from `Gear`. No `before` means that the
-- slot held nothing since the login. JSON here has no booleans, so `was` is a word.
function Inputs.ItemEquipped(at, slot, item, before)
	return {
		type = "item_equipped",
		at = at,
		slot = slot,
		item = item.item,
		quality = item.quality,
		level = item.level,
		replaced = before and before.level,
		was = before and "worn" or "empty",
	}
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

-- The bridge takes no line break, so the body goes as a list of paragraphs. An empty
-- title is no title.
function Inputs.StoryAccepted(at, number, title, paragraphs)
	return { type = "story_accepted", at = at, number = number, title = Present(title), paragraphs = paragraphs }
end

-- A like ("up") or a dislike ("down") of a narrator text (GAMEPLAY.md 3.2.2). `first` is
-- the first event of a chapter or a tale, and nil for a narrator line or the summary.
function Inputs.LineRated(at, rated, first, rating)
	return { type = "line_rated", at = at, rated = rated, first = first, rating = rating }
end

function Inputs.StoryRemoved(at, number)
	return { type = "story_removed", at = at, number = number }
end

function Inputs.Emote(at, emote, target, hour)
	return { type = "emote_done", at = at, emote = emote, target = target, hour = hour }
end

-- The count of one item in your bags, for an open carry step of this NPC.
function Inputs.ItemsHeld(at, npc, item, count)
	return { type = "items_held", at = at, npc = npc, item = item, count = count }
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

-- "Help me write", or a typed step, for a player task (GAMEPLAY.md 4.7). The idea holds no
-- name of a player.
function Inputs.DraftAsked(at, idea)
	return { type = "draft_asked", at = at, idea = idea }
end

function Inputs.JournalAsked(page)
	return { type = "journal_asked", page = page }
end
