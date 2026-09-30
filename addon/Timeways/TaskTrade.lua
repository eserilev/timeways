-- Watches the trade window for player tasks (GAMEPLAY.md 4.7): the items of a "bring an
-- item" step, and the reward that the giver hands over at the turn-in. Timeways never moves
-- an item itself. It only reads what the two players put in the window.

local _, ns = ...

local TaskTrade = {}
ns.TaskTrade = TaskTrade

-- The seventh slot holds an item that is not traded, such as one to enchant.
local TRADE_SLOTS = 6

-- The player on the other side of the open window, and the window as both players
-- accepted it.
local partner, window

local function Readable(value)
	return value ~= nil and not issecretvalue(value)
end

local function Add(items, name, count)
	if Readable(name) and type(name) == "string" and Readable(count) and type(count) == "number" then
		items[name] = (items[name] or 0) + count
	end
end

local function Money(value)
	return Readable(value) and tonumber(value) or 0
end

local function Snapshot()
	local gave, got = {}, {}
	for slot = 1, TRADE_SLOTS do
		local name, _, count = GetTradePlayerItemInfo(slot)
		Add(gave, name, count)
		local theirName, _, theirCount = GetTradeTargetItemInfo(slot)
		Add(got, theirName, theirCount)
	end
	return {
		with = partner,
		gave = gave,
		got = got,
		money = Money(GetPlayerTradeMoney()),
		moneyGot = Money(GetTargetTradeMoney()),
	}
end

function TaskTrade.Shown()
	partner = ns.TaskPeople.OfUnit("npc")
	window = nil
end

-- Once both players accept, the game makes the trade and closes the window. A change of an
-- item ends both accepts, and the window stays open.
function TaskTrade.AcceptChanged(playerAccepted, targetAccepted)
	local both = playerAccepted == 1 and targetAccepted == 1
	window = both and partner and Snapshot() or nil
end

-- A completed trade: { at, with, gave, got, money, moneyGot }.
function TaskTrade.Closed()
	local trade = window
	partner, window = nil, nil
	if trade then
		trade.at = time()
		ns.TaskTracker.Traded(trade)
	end
end
