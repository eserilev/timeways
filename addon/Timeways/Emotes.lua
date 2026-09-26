-- Your emotes (GAMEPLAY.md 5.4.1). Only `/slap` on an NPC has a consequence in the world
-- so far: the NPC counts it, and trusts you less.

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

-- Runs after each emote of yours, through a hook that changes nothing.
function Emotes.Performed(emote, typedName)
	if type(emote) ~= "string" or emote:upper() ~= "SLAP" then
		return
	end
	local name = NpcTarget(typedName)
	if name then
		ns.Outbox.Add(ns.Inputs.Slapped(time(), name))
	end
end
