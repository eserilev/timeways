-- AI on or off: the player chose at setup to play with a model or with none (GAMEPLAY.md
-- 5.6). The desktop says it in the `ai` field of each journal.

local _, ns = ...

local Ai = {}
ns.Ai = Ai

-- On until a journal says off, so a desktop from before the switch keeps every feature.
local on = true

-- From the first page of each journal. Only "off" turns AI off.
function Ai.DesktopSays(value)
	on = value ~= "off"
end

function Ai.IsOn()
	return on
end

-- The second half of each line that says a feature needs AI.
Ai.TURN_ON = "To turn it on, run gnomish-relay setup --timeways --ai on."
