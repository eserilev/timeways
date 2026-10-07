-- Events, replies, the slash commands, and the flush timer.

local ADDON_NAME, ns = ...

-- Each batch is a screenshot, so small events wait this long (GAMEPLAY.md 5.4).
local FLUSH_SECONDS = 600

local HANDLERS = {
	-- The key addon can fail to load while the files of this addon load, so the handoff
	-- tries again here. Only our own ADDON_LOADED counts.
	ADDON_LOADED = function(name)
		if name == ADDON_NAME then
			ns.KeyHandoff.Try("ADDON_LOADED")
		end
	end,
	PLAYER_LOGIN = function()
		ns.KeyHandoff.Try("PLAYER_LOGIN")
	end,
	PLAYER_ENTERING_WORLD = function()
		-- First, so an error in the rest never hides the setup window.
		ns.Welcome.Login()
		ns.Watch.Login()
		ns.GameQuests.Scan()
		ns.Gear.Login()
	end,
	QUEST_ACCEPTED = ns.GameQuests.Accepted,
	QUEST_TURNED_IN = ns.GameQuests.TurnedIn,
	QUEST_WATCH_UPDATE = ns.GameQuests.Progress,
	UNIT_AURA = function(unit, info)
		ns.QuestAuras.Changed(unit, info)
		ns.Mounts.AuraChanged(unit, info)
	end,
	PLAYER_MOUNT_DISPLAY_CHANGED = ns.Mounts.DisplayChanged,
	PLAYER_EQUIPMENT_CHANGED = ns.Gear.Changed,
	GET_ITEM_INFO_RECEIVED = ns.Gear.ItemLoaded,
	ZONE_CHANGED_NEW_AREA = ns.Watch.Zone,
	ZONE_CHANGED = ns.Watch.Zone,
	ZONE_CHANGED_INDOORS = ns.Watch.Zone,
	PLAYER_LEVEL_UP = ns.Watch.Level,
	PLAYER_UPDATE_RESTING = ns.Watch.Rest,
	UPDATE_BATTLEFIELD_STATUS = ns.Pvp.BattlefieldStatus,
	MAJOR_FACTION_RENOWN_LEVEL_CHANGED = ns.Pvp.RankChanged,
	GOSSIP_SHOW = function()
		ns.Watch.Npc()
		ns.Seen.Gossip()
	end,
	QUEST_GREETING = function()
		ns.Watch.Npc()
		ns.Seen.QuestGreeting()
	end,
	QUEST_DETAIL = function()
		ns.Watch.Npc()
		ns.Seen.QuestDetail()
	end,
	QUEST_PROGRESS = function()
		ns.Watch.Npc()
		ns.Seen.QuestProgress()
	end,
	QUEST_COMPLETE = function()
		ns.Watch.Npc()
		ns.Seen.QuestComplete()
	end,
	ITEM_TEXT_READY = ns.Seen.Book,
	PLAYER_TARGET_CHANGED = function()
		ns.Foes.SeeTarget()
		ns.Sightings.SeeTarget()
	end,
	UPDATE_MOUSEOVER_UNIT = function()
		ns.Foes.SeeMouseover()
		ns.Sightings.SeeMouseover()
	end,
	NAME_PLATE_UNIT_ADDED = ns.Foes.See,
	PARTY_KILL = ns.Foes.PartyKill,
	ENCOUNTER_END = ns.Foes.EncounterEnd,
	PLAYER_DEAD = ns.Foes.Died,
	BAG_UPDATE_DELAYED = ns.Journal.BagsChanged,
	-- The game still draws while the player camps or quits, so the waiting events go now.
	-- At PLAYER_LOGOUT no screenshot can go.
	-- The talk window steps aside in a fight, and comes back after it (GAMEPLAY.md 3.5).
	PLAYER_REGEN_DISABLED = ns.TalkWindow.CombatStarted,
	PLAYER_REGEN_ENABLED = ns.TalkWindow.CombatEnded,
	PLAYER_CAMPING = ns.Outbox.Flush,
	PLAYER_QUITING = ns.Outbox.Flush,
}

local frame = CreateFrame("Frame")
-- Literal names, so the API gate of Gnomish Relay checks each one against the client.
frame:RegisterEvent("ADDON_LOADED")
frame:RegisterEvent("PLAYER_LOGIN")
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("PLAYER_LEVEL_UP")
frame:RegisterEvent("PLAYER_UPDATE_RESTING")
frame:RegisterEvent("UPDATE_BATTLEFIELD_STATUS")
frame:RegisterEvent("MAJOR_FACTION_RENOWN_LEVEL_CHANGED")
frame:RegisterEvent("GOSSIP_SHOW")
frame:RegisterEvent("QUEST_GREETING")
frame:RegisterEvent("QUEST_DETAIL")
frame:RegisterEvent("QUEST_PROGRESS")
frame:RegisterEvent("QUEST_COMPLETE")
frame:RegisterEvent("ITEM_TEXT_READY")
frame:RegisterEvent("PLAYER_TARGET_CHANGED")
frame:RegisterEvent("UPDATE_MOUSEOVER_UNIT")
frame:RegisterEvent("NAME_PLATE_UNIT_ADDED")
frame:RegisterEvent("PARTY_KILL")
frame:RegisterEvent("ENCOUNTER_END")
frame:RegisterEvent("PLAYER_DEAD")
frame:RegisterEvent("BAG_UPDATE_DELAYED")
frame:RegisterEvent("PLAYER_CAMPING")
frame:RegisterEvent("PLAYER_QUITING")
frame:RegisterEvent("QUEST_ACCEPTED")
frame:RegisterEvent("QUEST_TURNED_IN")
frame:RegisterEvent("QUEST_WATCH_UPDATE")
frame:RegisterEvent("PLAYER_REGEN_DISABLED")
frame:RegisterEvent("PLAYER_REGEN_ENABLED")
frame:RegisterEvent("PLAYER_MOUNT_DISPLAY_CHANGED")
frame:RegisterEvent("PLAYER_EQUIPMENT_CHANGED")
frame:RegisterEvent("GET_ITEM_INFO_RECEIVED")
-- Only the player: the auras of every unit around would fire this all the time.
frame:RegisterUnitEvent("UNIT_AURA", "player")
frame:SetScript("OnEvent", function(_, event, ...)
	HANDLERS[event](...)
end)

C_Timer.NewTicker(FLUSH_SECONDS, ns.Outbox.Flush)
C_Timer.NewTicker(60, ns.QuestSteps.HourTick)

hooksecurefunc(C_ChatInfo, "PerformEmote", ns.Emotes.Performed)

local REPLIES = {
	lore_answer = ns.Lore.Show,
	talk_answer = ns.Talk.Show,
	journal = ns.Journal.Receive,
	draft_answer = ns.TaskDraftHelp.Receive,
	events_seen = ns.QuestSteps.EventsSeen,
}

-- A notice is a line of Timeways itself, such as why a task was refused. It never takes
-- the voice of the narrator.
local function ShowNotice(text)
	if type(text) == "string" and text ~= "" then
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. ns.Plain(text))
	end
end

-- A reply holds one JSON line: an answer, a journal page, or `events_seen` for a batch of
-- game events. Each one can carry a line of the narrator and a notice.
function ns.OnReply(text)
	if type(text) ~= "string" then
		return
	end
	for line in text:gmatch("[^\n]+") do
		local value = ns.Json.Decode(line)
		local handler = type(value) == "table" and REPLIES[value.type]
		if handler then
			handler(value)
		end
		if type(value) == "table" then
			ns.Narrator.Say(value.narrator)
			ShowNotice(value.notice)
		end
	end
end

SLASH_TIMEWAYSLORE1 = "/lore"
SlashCmdList.TIMEWAYSLORE = ns.Lore.Ask

SLASH_TIMEWAYSTALK1 = "/talk"
SlashCmdList.TIMEWAYSTALK = ns.Talk.Ask

SLASH_TIMEWAYSQUEST1 = "/quest"
SlashCmdList.TIMEWAYSQUEST = ns.Quest.Command

SLASH_TIMEWAYSHERO1 = "/hero"
SlashCmdList.TIMEWAYSHERO = ns.Hero.Command

SLASH_TIMEWAYSSTORY1 = "/story"
SlashCmdList.TIMEWAYSSTORY = ns.PlayerStories.Command

SLASH_TIMEWAYSSTORIES1 = "/stories"
SlashCmdList.TIMEWAYSSTORIES = function()
	ns.JournalFrame.Open("stories")
end

-- Dev mode (TESTING.md): it does nothing unless the desktop turned it on.
SLASH_TIMEWAYSDEV1 = "/twdev"
SlashCmdList.TIMEWAYSDEV = ns.Dev.Command

SLASH_TIMEWAYSJOURNAL1 = "/journal"
SLASH_TIMEWAYSJOURNAL2 = "/timeways"
SlashCmdList.TIMEWAYSJOURNAL = function(message)
	if message:match("^%s*test%s*$") then
		ns.SelfTest.Start()
		return
	end
	if message:match("^%s*help%s*$") then
		ns.Welcome.Open(ns.Welcome.Reason())
		return
	end
	ns.JournalFrame.Toggle()
end
