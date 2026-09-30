-- `/talk <words>` to the NPC that you target, and what it says (GAMEPLAY.md 3.5).

local _, ns = ...

local Talk = {}
ns.Talk = Talk

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

function Talk.Ask(words)
	words = words:match("^%s*(.-)%s*$")
	if words == "" then
		Say("Target someone, then type: /talk hello")
		return
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
	local input = ns.Inputs.Talk(time(), npc, words)
	if not ns.Outbox.Fits(input) then
		Say("That is too long to say.")
		return
	end
	ns.Outbox.Add(input)
	ns.Outbox.Flush()
end

-- With no words, no model answered.
function Talk.Show(answer)
	local npc = type(answer.npc) == "string" and ns.Plain(answer.npc) or "?"
	if type(answer.text) == "string" then
		DEFAULT_CHAT_FRAME:AddMessage(string.format("|cffffd100%s says:|r %s", npc, ns.Plain(answer.text)))
	else
		DEFAULT_CHAT_FRAME:AddMessage(string.format("|cffffd100%s looks at you and says nothing.|r", npc))
	end
end
