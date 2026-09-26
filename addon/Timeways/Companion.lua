-- Sprocket, the companion (GAMEPLAY.md 3.2). The desktop writes each line, and any reply
-- can carry one.

local _, ns = ...

local Companion = {}
ns.Companion = Companion

local PREFIX = "|cff8fbfffSprocket|r: "

function Companion.Say(text)
	if type(text) == "string" and text ~= "" then
		DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. ns.Plain(text))
	end
end
