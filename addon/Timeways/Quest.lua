-- `/quest` asks the NPC that you target for a task. `/quest accept` and `/quest decline`
-- answer the offer that waits (GAMEPLAY.md 3.4). The offer comes back as a narrator line.

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

function Quest.Ask()
	local npc = ns.Units.NpcName("target")
	if not npc then
		Say("Who are you asking? Target someone first.")
		return
	end
	Send(ns.Inputs.QuestAsked(time(), npc))
	Say("You ask " .. ns.Plain(npc) .. " for a task.")
end

-- A meet step needs a new `npc_met`, also for an NPC that you met in this session. The
-- buttons of the book name their quest. The chat command answers the newest offer.
function Quest.Accept(number)
	ns.Watch.ForgetMet()
	Send(ns.Inputs.QuestAccepted(time(), number))
	Say("You take the task. It's in your journal.")
end

function Quest.Decline(number)
	Send(ns.Inputs.QuestDeclined(time(), number))
	Say("You turn the task down.")
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
		Say("Target someone and type /quest to ask for a task. Then /quest accept, or /quest decline.")
		return
	end
	run()
end
