-- The reward of a player task: money and items, as the trade window holds them (GAMEPLAY.md
-- 4.7). It is a promise. The giver hands it over in a normal trade at the turn-in, so the
-- offer carries only its text.

local _, ns = ...

local TaskReward = {}
ns.TaskReward = TaskReward

-- The trade window holds 6 items that change hands.
TaskReward.MAX_ITEMS = 6
local SILVER, GOLD = 100, 10000
TaskReward.MAX_COPPER = 99999 * GOLD + 99 * SILVER + 99

function TaskReward.Coins(copper)
	return math.floor(copper / GOLD), math.floor(copper / SILVER) % 100, copper % SILVER
end

-- Each part is cut to its own box, as the money boxes of the game do.
function TaskReward.Copper(gold, silver, copper)
	local function Part(value, most)
		value = math.floor(tonumber(value) or 0)
		return math.max(0, math.min(value, most))
	end
	return Part(gold, 99999) * GOLD + Part(silver, 99) * SILVER + Part(copper, 99)
end

local function MoneyText(copper)
	local parts = {}
	local gold, silver, rest = TaskReward.Coins(copper)
	for _, coin in ipairs({ { gold, "gold" }, { silver, "silver" }, { rest, "copper" } }) do
		if coin[1] > 0 then
			parts[#parts + 1] = coin[1] .. " " .. coin[2]
		end
	end
	return table.concat(parts, " ")
end

local function ItemText(item)
	return item.count > 1 and (item.count .. " " .. item.name) or item.name
end

-- "5 gold 20 silver, 10 Linen Cloth", or "" for no reward.
function TaskReward.Text(copper, items)
	local parts = {}
	if copper > 0 then
		parts[1] = MoneyText(copper)
	end
	for _, item in ipairs(items) do
		parts[#parts + 1] = ItemText(item)
	end
	return table.concat(parts, ", ")
end

function TaskReward.Fits(copper, items)
	return #TaskReward.Text(copper, items) <= ns.TaskWire.LIMITS.reward
end

-- The same item again adds to its count. Returns false when the item does not fit.
function TaskReward.AddItem(copper, items, item)
	for _, known in ipairs(items) do
		if known.id == item.id then
			local before = known.count
			known.count = math.min(known.count + item.count, ns.TaskWire.MAX_COUNT)
			if TaskReward.Fits(copper, items) then
				return true
			end
			known.count = before
			return false
		end
	end
	if #items >= TaskReward.MAX_ITEMS then
		return false
	end
	items[#items + 1] = item
	if TaskReward.Fits(copper, items) then
		return true
	end
	items[#items] = nil
	return false
end
