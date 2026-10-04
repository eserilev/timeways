-- One budget of addon messages for all of Timeways: the parts of player quests (GAMEPLAY.md
-- 4.7) and of MSP (3.7.1). The game drops addon messages past about 10 in a burst and 1 each
-- second after it.

local _, ns = ...

local AddonBudget = {}
ns.AddonBudget = AddonBudget

local BURST, PER_SECOND = 8, 1

local tokens, filled = BURST, nil

-- True while a part can go now.
function AddonBudget.Has()
	local now = GetTime()
	filled = filled or now
	tokens = math.min(BURST, tokens + (now - filled) * PER_SECOND)
	filled = now
	return tokens >= 1
end

-- Only a part that the game took costs a token.
function AddonBudget.Spend()
	tokens = tokens - 1
end
