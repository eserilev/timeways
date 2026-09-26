-- Your emotes (GAMEPLAY.md 5.4.1). Each one is a flavor moment. A `/slap` on an NPC also
-- has a consequence in the world: the NPC counts it, and trusts you less.

local _, ns = ...

local Emotes = {}
ns.Emotes = Emotes

-- A name typed after the emote can be a player's name, so only the current target counts,
-- and only when it is an NPC (5.11).
local function NpcTarget(typedName)
	if typedName ~= nil and typedName ~= "" then
		return nil
	end
	return ns.Units.NpcName("target")
end

-- Runs after each emote of yours, through a hook that changes nothing. An emote token is
-- a word such as "DANCE". Anything else is not an emote of the game.
function Emotes.Performed(emote, typedName)
	local token = type(emote) == "string" and emote:lower()
	if not token or #token > 24 or not token:match("^%a+$") then
		return
	end
	local target = NpcTarget(typedName)
	ns.Outbox.Add(ns.Inputs.Emote(time(), token, target, ns.Inputs.Hour()))
	if token == "slap" and target then
		ns.Outbox.Add(ns.Inputs.Slapped(time(), target))
	end
end
