-- The commands of `/twdev` that fake a moment of your own play (TESTING.md, "Dev mode").
-- Each one starts at the narrowest point after the game API, so the real code of the addon
-- and of the desktop runs after it: the input lines of `Inputs.lua`, the outbox, the
-- bridge, and the story program.

local _, ns = ...

local Dev = ns.Dev

local function Add(input)
	ns.Outbox.Add(input)
end

local function Number(text, low, high)
	local number = tonumber(text)
	if number and number % 1 == 0 and number >= low and number <= high then
		return number
	end
end

local function NeedsName(name, usage)
	if name == "" then
		Dev.Say("usage: /twdev " .. usage)
		return true
	end
	return false
end

-- The zone where you stand, for a moment that needs one.
local function Here()
	return GetRealZoneText()
end

Dev.Add("level", "level <n>: reach level n (1 to 60).", function(rest)
	local level = Number(rest, 1, 60)
	if not level then
		Dev.Say("usage: /twdev level <1 to 60>")
		return
	end
	ns.Watch.Level(level)
end)

Dev.Add("zone", "zone <zone> [/ subzone]: walk into a place.", function(rest)
	local zone, subzone = Dev.Parts(rest)
	if NeedsName(zone, "zone <zone> [/ subzone]") then
		return
	end
	Add(ns.Inputs.Zone(time(), zone, subzone, nil, nil))
end)

Dev.Add("taxi", "taxi: fly over Elwynn Forest and Westfall, and land in Duskwood.", function()
	Add(ns.Inputs.Zone(time(), "Elwynn Forest", "Goldshire", nil, "yes"))
	Add(ns.Inputs.Zone(time(), "Westfall", "Sentinel Hill", nil, "yes"))
	Add(ns.Inputs.Zone(time(), "Duskwood", "Darkshire", nil, nil))
end)

local function EnterInstance(zone, kind)
	Add(ns.Inputs.Zone(time(), zone, nil, nil, nil))
	Add(ns.Inputs.Instance(time(), zone, kind))
end

Dev.Add("dungeon", "dungeon <name>: enter a dungeon.", function(rest)
	if not NeedsName(rest, "dungeon <name>") then
		EnterInstance(rest, "party")
	end
end)

Dev.Add("raid", "raid <name>: enter a raid.", function(rest)
	if not NeedsName(rest, "raid <name>") then
		EnterInstance(rest, "raid")
	end
end)

Dev.Add("bg-win", "bg-win: win a battle in Warsong Gulch.", function()
	EnterInstance("Warsong Gulch", "pvp")
	Add(ns.Inputs.BgWon(time(), "Warsong Gulch"))
end)

Dev.Add("pvp-rank", "pvp-rank <n>: reach PvP rank n (0 to 14).", function(rest)
	local rank = Number(rest, 0, 14)
	if not rank then
		Dev.Say("usage: /twdev pvp-rank <0 to 14>")
		return
	end
	Add(ns.Inputs.PvpRank(time(), rank))
end)

Dev.Add("rest", "rest: rest at an inn, and walk out of it.", function()
	Add(ns.Inputs.RestChanged(time(), "yes"))
	Add(ns.Inputs.RestChanged(time(), "no"))
end)

local MOUNT_SPEEDS = { epic = 200 }

Dev.Add("mount", "mount <name> [epic]: ride a mount, or a swift one.", function(rest)
	local mount, epic = Dev.LastWord(rest, MOUNT_SPEEDS)
	if not NeedsName(mount, "mount <name> [epic]") then
		Add(ns.Inputs.MountRidden(time(), mount, epic and MOUNT_SPEEDS[epic] or 160))
	end
end)

local KILL_KINDS = { rare = true, boss = true, worldboss = true }

Dev.Add(
	"kill",
	"kill <name> [rare|boss|worldboss]: defeat a rare or a boss. With no kind: a kill for a quest step.",
	function(rest)
		local name, kind = Dev.LastWord(rest, KILL_KINDS)
		if NeedsName(name, "kill <name> [rare|boss|worldboss]") then
			return
		end
		if kind then
			Add(ns.Inputs.Defeated(time(), name, kind))
		else
			Add(ns.Inputs.Killed(time(), name))
		end
	end
)

Dev.Add("seen", "seen <name> [friendly] [beast]: see an NPC, hostile unless friendly.", function(rest)
	local name, beast = Dev.LastWord(rest, { beast = true })
	local npc, friendly = Dev.LastWord(name, { friendly = true })
	if not NeedsName(npc, "seen <name> [friendly] [beast]") then
		Add(ns.Inputs.NpcSeen(time(), npc, friendly and "friendly" or "hostile", beast and "beast" or "humanoid"))
	end
end)

Dev.Add("death", "death <killer>: die to an NPC.", function(rest)
	if not NeedsName(rest, "death <killer>") then
		Add(ns.Inputs.Died(time(), rest, nil, nil, ns.Inputs.Hour()))
	end
end)

-- The causes of the death recap, as Foes.lua sends them.
local CAUSES = { fall = "falling", drown = "drowning", lava = "lava" }

Dev.Add("fall", "fall: die from a fall.", function()
	Add(ns.Inputs.Died(time(), nil, CAUSES.fall, nil, ns.Inputs.Hour()))
end)
Dev.Add("drown", "drown: drown.", function()
	Add(ns.Inputs.Died(time(), nil, CAUSES.drown, nil, ns.Inputs.Hour()))
end)
Dev.Add("lava", "lava: die in lava.", function()
	Add(ns.Inputs.Died(time(), nil, CAUSES.lava, nil, ns.Inputs.Hour()))
end)

-- An item of the main hand, as Gear.lua reads it: the quality of the game and the level.
local QUALITIES = { rare = 3, epic = 4 }

Dev.Add("item", "item <name> epic|rare: put on an item, 12 levels above the one before.", function(rest)
	local item, quality = Dev.LastWord(rest, QUALITIES)
	if NeedsName(item, "item <name> epic|rare") or not quality then
		Dev.Say("usage: /twdev item <name> epic|rare")
		return
	end
	local level = math.max(UnitLevel("player") or 1, 10) + 5
	local before = { item = "Worn Mace", quality = 2, level = level - 12 }
	Add(ns.Inputs.ItemEquipped(time(), 16, { item = item, quality = QUALITIES[quality], level = level }, before))
end)

Dev.Add("game-quest", "game-quest <title> [class]: take a quest of the game, and turn it in.", function(rest)
	local title, class = Dev.LastWord(rest, { class = true })
	if NeedsName(title, "game-quest <title> [class]") then
		return
	end
	local kind = class and "class" or "normal"
	Add(ns.Inputs.GameQuestAccepted(time(), title, kind))
	Add(ns.Inputs.GameQuestDone(time(), title, kind))
end)

Dev.Add("mark", "mark <quest> / <buff>: a quest of the game leaves a lasting buff.", function(rest)
	local quest, mark = Dev.Parts(rest)
	if NeedsName(quest, "mark <quest> / <buff>") or not mark then
		return
	end
	Add(ns.Inputs.QuestMarked(time(), quest, mark))
end)

-- A weight of 16 in the zone where you stand, then a quest in a new zone: a natural break
-- of the chronicle that closes the chapter (docs/plans/chapters.md). Outside an instance
-- only: a quest in an instance goes to its tale.
Dev.Add("chapter-end", "chapter-end [new zone]: close the open chapter, and start one.", function(rest)
	local stamp = time() % 100000
	for n = 1, 16 do
		Add(ns.Inputs.GameQuestDone(time(), string.format("Dev errand %d-%d", stamp, n), "normal"))
	end
	local zone = rest ~= "" and rest or (Here() == "Duskwood" and "Westfall" or "Duskwood")
	Add(ns.Inputs.Zone(time(), zone, nil, nil, nil))
	Add(ns.Inputs.GameQuestDone(time(), string.format("Dev errand %d-17", stamp), "normal"))
end)

Dev.Add("hour", "hour <0 to 23>: the hour of your computer changes.", function(rest)
	local hour = Number(rest, 0, 23)
	if hour then
		Add(ns.Inputs.HourChanged(time(), hour))
	end
end)

-- NPCs: the real code of the addon reads your target through ns.Units, and Dev.WithNpc
-- fakes it.

Dev.Add("meet", "meet <npc>: open the gossip window of an NPC.", function(rest)
	if not NeedsName(rest, "meet <npc>") then
		Dev.WithNpc(rest, false, ns.Watch.Npc)
	end
end)

Dev.Add("gossip", "gossip <npc> / <text>: read what an NPC says in its window.", function(rest)
	local npc, text = Dev.Parts(rest)
	if NeedsName(npc, "gossip <npc> / <text>") or not text then
		return
	end
	Dev.WithNpc(npc, false, function()
		ns.Watch.Npc()
		ns.Seen.Read("gossip", nil, npc, text)
	end)
end)

Dev.Add("quest-text", "quest-text <npc> / <title>: read the text of a quest of the game.", function(rest)
	local npc, title = Dev.Parts(rest)
	if NeedsName(npc, "quest-text <npc> / <title>") or not title then
		return
	end
	local text = "Will you help us, $N? " .. title .. " is a task for the brave."
	Dev.WithNpc(npc, false, function()
		ns.Watch.Npc()
		-- The game writes your name where the quest says $N, and Seen.lua takes it out again.
		local me = UnitName("player")
		ns.Seen.Read("quest", title, npc, me and (text:gsub("%$N", me)) or text)
	end)
end)

Dev.Add("book", "book <title> [/ text]: read a book.", function(rest)
	local title, text = Dev.Parts(rest)
	if NeedsName(title, "book <title> [/ text]") then
		return
	end
	ns.Seen.Read("book", title, nil, text or ("The pages of " .. title .. " tell of the old kingdoms."))
end)

Dev.Add("talk", "talk <npc> [/ words]: /talk to an NPC, as if you targeted it.", function(rest)
	local npc, words = Dev.Parts(rest)
	if not NeedsName(npc, "talk <npc> [/ words]") then
		Dev.WithNpc(npc, false, function()
			ns.Talk.Ask(words or "")
		end)
	end
end)

Dev.Add("quest", "quest <npc>: /quest to an NPC, as if you targeted it.", function(rest)
	if not NeedsName(rest, "quest <npc>") then
		Dev.WithNpc(rest, false, ns.Quest.Ask)
	end
end)

Dev.Add("slap", "slap <npc>: /slap an NPC, as if you targeted it.", function(rest)
	if not NeedsName(rest, "slap <npc>") then
		Dev.WithNpc(rest, false, function()
			ns.Emotes.Performed("SLAP")
		end)
	end
end)

Dev.Add("emote", "emote <emote> [/ npc]: an emote, at an NPC or in the place where you stand.", function(rest)
	local emote, npc = Dev.Parts(rest)
	if NeedsName(emote, "emote <emote> [/ npc]") then
		return
	end
	if npc then
		Dev.WithNpc(npc, false, function()
			ns.Emotes.Performed(emote:upper())
		end)
	else
		ns.Emotes.Performed(emote:upper())
	end
end)

Dev.Add("carry", "carry <count> <item> / <npc>: show an NPC what you carry.", function(rest)
	local what, npc = Dev.Parts(rest)
	local count, item = what:match("^(%d+)%s+(.+)$")
	if not count or not npc then
		Dev.Say("usage: /twdev carry <count> <item> / <npc>")
		return
	end
	Add(ns.Inputs.ItemsHeld(time(), npc, item, tonumber(count)))
end)

-- The windows and the tooltips of the addon --------------------------------------------------

Dev.Add("journal", "journal: ask the desktop for the journal now, and open it.", function()
	ns.Journal.Request(0)
	ns.JournalFrame.Open()
end)

local REASONS = { setup = true, files = true, offline = true }

Dev.Add("welcome", "welcome setup|files|offline: the setup window for that reason.", function(rest)
	if REASONS[rest] then
		ns.Welcome.Open(rest)
	else
		Dev.Say("usage: /twdev welcome setup|files|offline")
	end
end)
