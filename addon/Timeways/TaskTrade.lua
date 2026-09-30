-- Watches the trade window for player tasks (GAMEPLAY.md 4.7): the items of a "bring an
-- item" step, and the reward that the giver hands over at the turn-in. Timeways never moves
-- an item itself. It only reads what the two players put in the window.

local _, ns = ...

local TaskTrade = {}
ns.TaskTrade = TaskTrade

-- The seventh slot holds an item that is not traded, such as one to enchant.
local TRADE_SLOTS = 6

-- The player on the other side of the open window, the window as it was at its last
-- change, and whether both players accepted it.
local partner, window, bothAccepted
-- A closed window that the game has not said is a trade yet.
local closed

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

-- A completed trade: { at, with, gave, got, money, moneyGot }.
local function Finish(trade)
	closed = nil
	trade.at = time()
	ns.TaskTracker.Traded(trade)
end

function TaskTrade.Shown()
	partner = ns.TaskPeople.OfUnit("npc")
	window, bothAccepted, closed = nil, false, nil
	if partner then
		window = Snapshot()
	end
end

-- The window empties as it closes, so each change of an item or of money takes a new
-- snapshot. A change ends both accepts.
function TaskTrade.Changed()
	if partner then
		window, bothAccepted = Snapshot(), false
	end
end

-- The game can make the trade on the second accept before it tells of that accept.
function TaskTrade.AcceptChanged(playerAccepted, targetAccepted)
	if partner then
		window = Snapshot()
		bothAccepted = playerAccepted == 1 and targetAccepted == 1
	end
end

-- A window that closes after both accepts is a trade. So is one that the game calls a
-- trade with "Trade complete." (ERR_TRADE_COMPLETE), before or after it closes.
function TaskTrade.Closed()
	local trade, done = window, bothAccepted
	partner, window, bothAccepted = nil, nil, false
	closed = trade
	if trade and done then
		Finish(trade)
	end
end

function TaskTrade.InfoMessage(messageType)
	if not Readable(messageType) or GetGameMessageInfo(messageType) ~= "ERR_TRADE_COMPLETE" then
		return
	end
	if window then
		bothAccepted = true
	elseif closed then
		Finish(closed)
	end
end
