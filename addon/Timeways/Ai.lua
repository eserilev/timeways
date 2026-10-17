-- AI on or off: the player chose at setup to play with a model or with none (GAMEPLAY.md
-- 5.6). The desktop says it in the `ai` field of each journal.

local _, ns = ...

local Ai = {}
ns.Ai = Ai

-- "unknown" until the first journal, then "on" or "off". Unknown counts as on, so a desktop
-- from before the switch keeps every feature.
local state = "unknown"

-- From the first page of each journal. Only "off" turns AI off.
function Ai.DesktopSays(value)
	state = value == "off" and "off" or "on"
end

function Ai.IsOn()
	return state ~= "off"
end

-- True once a journal said on or off.
function Ai.IsKnown()
	return state ~= "unknown"
end

-- TODO: Remove when the relay ships `setup --ai`. Until then, check the command before a release.
-- The second half of each line that says a feature needs AI.
Ai.TURN_ON = "To turn it on, run gnomish-relay setup --timeways --ai on."
Ai.TALK_OFF = "Talking to NPCs needs AI. " .. Ai.TURN_ON
Ai.QUESTS_OFF = "Quests from NPCs need AI. " .. Ai.TURN_ON
