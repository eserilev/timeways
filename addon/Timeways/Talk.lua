-- `/talk <words>` to the NPC that you target, and what it says (GAMEPLAY.md 3.5). The talk
-- window shows the talk. An answer for a closed window goes to the chat.

local _, ns = ...

local Talk = {}
ns.Talk = Talk

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

-- `/talk` alone is how most players try it first, so it opens with a greeting.
local GREETING = "Hello."

-- The words on their way, oldest first: each { npc, words, at }. The desktop answers in
-- order, so an answer belongs to the oldest words to its NPC.
local sent = {}

local function TakeSent(npc)
	for n, words in ipairs(sent) do
		if words.npc == npc then
			return table.remove(sent, n)
		end
	end
	return nil
end

local function Failed(npc, words)
	TakeSent(npc)
	ns.TalkWindow.Failed(npc, words)
end

local function Send(npc, words)
	local at = time()
	local input = ns.Inputs.Talk(at, npc, words)
	if not ns.Outbox.Fits(input) then
		Say("That's too long. Try something shorter.")
		return
	end
	if ns.Welcome.OpenIfNoApp() then
		return
	end
	ns.Carry.Met(npc)
	sent[#sent + 1] = { npc = npc, words = words, at = at }
	ns.TalkWindow.Asked(npc, words)
	ns.Outbox.Add(input, function()
		Failed(npc, words)
	end)
	ns.Outbox.Flush()
end

function Talk.Ask(words)
	words = words:match("^%s*(.-)%s*$")
	if words == "" then
		words = GREETING
	end
	local npc = ns.Units.FriendlyNpcName("target")
	local other = ns.Units.NpcName("target")
	if not npc and other then
		Say(ns.Plain(other) .. " won't talk to you.")
		return
	end
	if not npc then
		Say("Target someone to talk to first.")
		return
	end
	Send(npc, words)
end

-- A reply in the talk window goes to the NPC of the window, also when the target changed.
function Talk.Reply(npc, words)
	Send(npc, words)
end

local function ShowInChat(npc, text)
	if text then
		DEFAULT_CHAT_FRAME:AddMessage(string.format("|cffffd100%s says:|r %s", npc, ns.Plain(text)))
	else
		DEFAULT_CHAT_FRAME:AddMessage(string.format("|cffffd100%s looks at you and says nothing.|r", npc))
	end
end

-- With no words, no model answered. An answer with words joins the past talks.
function Talk.Show(answer)
	local npc = type(answer.npc) == "string" and answer.npc or nil
	local text = type(answer.text) == "string" and answer.text or nil
	local words = TakeSent(npc)
	if words and text then
		ns.TalkHistory.Add(npc, words.words, text, words.at)
	end
	if not ns.TalkWindow.Takes(npc, text) then
		ShowInChat(npc and ns.Plain(npc) or "?", text)
	end
	-- A talk can change the trust of the NPC, or ask it for a quest, and the journal
	-- carries both.
	ns.Journal.Request(0)
end
