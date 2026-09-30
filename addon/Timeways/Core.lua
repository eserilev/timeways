-- Events, replies, the slash commands, and the flush timer.

local _, ns = ...

local FLUSH_SECONDS = 60

local HANDLERS = {
	PLAYER_ENTERING_WORLD = function()
		ns.Watch.Login()
		ns.GameQuests.Scan()
	end,
	QUEST_ACCEPTED = ns.GameQuests.Accepted,
	QUEST_TURNED_IN = ns.GameQuests.TurnedIn,
	ZONE_CHANGED_NEW_AREA = ns.Watch.Zone,
	ZONE_CHANGED = ns.Watch.Zone,
	ZONE_CHANGED_INDOORS = ns.Watch.Zone,
	PLAYER_LEVEL_UP = ns.Watch.Level,
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
	PLAYER_TARGET_CHANGED = ns.Foes.SeeTarget,
	UPDATE_MOUSEOVER_UNIT = ns.Foes.SeeMouseover,
	NAME_PLATE_UNIT_ADDED = ns.Foes.See,
	PARTY_KILL = ns.Foes.PartyKill,
	ENCOUNTER_END = ns.Foes.EncounterEnd,
	PLAYER_DEAD = ns.Foes.Died,
}

local frame = CreateFrame("Frame")
-- Literal names, so the API gate of Gnomish Relay checks each one against the client.
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("PLAYER_LEVEL_UP")
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
frame:RegisterEvent("QUEST_ACCEPTED")
frame:RegisterEvent("QUEST_TURNED_IN")
frame:SetScript("OnEvent", function(_, event, ...)
	HANDLERS[event](...)
end)

C_Timer.NewTicker(FLUSH_SECONDS, ns.Outbox.Flush)

hooksecurefunc(C_ChatInfo, "PerformEmote", ns.Emotes.Performed)

local REPLIES = {
	lore_answer = ns.Lore.Show,
	talk_answer = ns.Talk.Show,
	journal = ns.Journal.Receive,
}

-- A reply holds one JSON line: an answer, a journal page, or `events_seen` for a batch of
-- game events. Each one can carry a line of the narrator.
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

SLASH_TIMEWAYSJOURNAL1 = "/journal"
SLASH_TIMEWAYSJOURNAL2 = "/timeways"
SlashCmdList.TIMEWAYSJOURNAL = function(message)
	if message:match("^%s*test%s*$") then
		ns.SelfTest.Start()
		return
	end
	ns.JournalFrame.Toggle()
end
