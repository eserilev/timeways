-- Events, replies, the slash commands, and the flush timer.

local _, ns = ...

local FLUSH_SECONDS = 60

local HANDLERS = {
	PLAYER_ENTERING_WORLD = ns.Watch.Login,
	ZONE_CHANGED_NEW_AREA = ns.Watch.Zone,
	ZONE_CHANGED = ns.Watch.Zone,
	ZONE_CHANGED_INDOORS = ns.Watch.Zone,
	PLAYER_LEVEL_UP = ns.Watch.Level,
	GOSSIP_SHOW = ns.Watch.Npc,
	QUEST_DETAIL = ns.Watch.Npc,
}

local frame = CreateFrame("Frame")
-- Literal names, so the API gate of Gnomish Relay checks each one against the client.
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("PLAYER_LEVEL_UP")
frame:RegisterEvent("GOSSIP_SHOW")
frame:RegisterEvent("QUEST_DETAIL")
frame:SetScript("OnEvent", function(_, event, ...)
	HANDLERS[event](...)
end)

C_Timer.NewTicker(FLUSH_SECONDS, ns.Outbox.Flush)

local REPLIES = {
	lore_answer = ns.Lore.Show,
	journal = ns.Journal.Receive,
}

-- A reply holds one JSON line, or nothing for a batch of game events.
function ns.OnReply(text)
	for line in text:gmatch("[^\n]+") do
		local value = ns.Json.Decode(line)
		local handler = type(value) == "table" and REPLIES[value.type]
		if handler then
			handler(value)
		end
	end
end

SLASH_TIMEWAYSLORE1 = "/lore"
SlashCmdList.TIMEWAYSLORE = ns.Lore.Ask

SLASH_TIMEWAYSJOURNAL1 = "/journal"
SLASH_TIMEWAYSJOURNAL2 = "/timeways"
SlashCmdList.TIMEWAYSJOURNAL = ns.JournalFrame.Toggle
