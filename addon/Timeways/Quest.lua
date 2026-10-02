-- `/quest` asks the NPC that you target for a task. `/quest accept` and `/quest decline`
-- answer the offer that waits (GAMEPLAY.md 3.4). The offer comes back as a notice, a line of Timeways.

local _, ns = ...

local Quest = {}
ns.Quest = Quest

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Send(input)
	ns.Outbox.Add(input)
	ns.Outbox.Flush()
end

-- The answers that the book shows before the desktop confirms them, by quest number.
local answered = {}

function Quest.Answered(number)
	return answered[number]
end

function Quest.JournalCame()
	answered = {}
end

-- The journal comes back in the same batch as the answer, and confirms it.
local function Answer(number, answer, input)
	if number then
		answered[number] = answer
	end
	ns.Outbox.Add(input)
	ns.Journal.Request(0)
	ns.JournalFrame.Refresh()
end

function Quest.Ask()
	local npc = ns.Units.FriendlyNpcName("target")
	local other = ns.Units.NpcName("target")
	if not npc and other then
		Say(ns.Plain(other) .. " has no quests to give.")
		return
	end
	if not npc then
		Say("Target someone to ask first.")
		return
	end
	if ns.Welcome.OpenIfNoApp() then
		return
	end
	Send(ns.Inputs.QuestAsked(time(), npc))
	Say("You ask " .. ns.Plain(npc) .. " for a quest.")
end

-- A meet step needs a new `npc_met`, also for an NPC that you met in this session. The
-- buttons of the book name their quest. The chat command answers the newest offer.
function Quest.Accept(number)
	ns.Watch.ForgetMet()
	Answer(number, "accepted", ns.Inputs.QuestAccepted(time(), number))
	Say("Quest accepted.")
end

function Quest.Decline(number)
	Answer(number, "declined", ns.Inputs.QuestDeclined(time(), number))
	Say("Quest declined.")
end

StaticPopupDialogs.TIMEWAYS_QUEST_ABANDON = {
	text = "Abandon this quest?\n\n%s",
	button1 = "Abandon",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		Answer(data.number, "abandoned", ns.Inputs.QuestAbandoned(time(), data.number))
		Say("Quest abandoned.")
	end,
}

-- Asks first, because an abandoned task never comes back.
function Quest.Abandon(number, title)
	StaticPopup_Show("TIMEWAYS_QUEST_ABANDON", ns.Plain(tostring(title)), nil, { number = number })
end

local WORDS = {
	[""] = Quest.Ask,
	accept = function()
		Quest.Accept()
	end,
	decline = function()
		Quest.Decline()
	end,
}

function Quest.Command(message)
	local run = WORDS[message:match("^%s*(%S*)"):lower()]
	if not run then
		Say("Target someone and type /quest to ask for a quest. Then /quest accept or /quest decline.")
		return
	end
	run()
end
