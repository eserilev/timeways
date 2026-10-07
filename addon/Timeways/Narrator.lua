-- The narrator (GAMEPLAY.md 3.2): one line in the voice of the chronicle at a big moment.
-- The desktop writes each line, and any reply can carry one.

local _, ns = ...

local Narrator = {}
ns.Narrator = Narrator

local PREFIX = "|cffe6cc80Narrator|r: "

function Narrator.Say(text)
	if type(text) == "string" and text ~= "" then
		DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. ns.WithName(text))
		ns.Ratings.Heard()
	end
end
