-- Puts the parts of the addon messages of each sender back together, for player quests
-- (TaskChunks.lua) and for MSP (MspParts.lua). The two wires write their headers in their
-- own way, and both keep their open messages here (GAMEPLAY.md 3.7.1, 4.7).

local _, ns = ...

local PartCollector = {}
ns.PartCollector = PartCollector

-- A sender with more open messages than this loses its oldest one.
local OPEN_PER_SENDER = 4
local MAX_SENDERS = 32
-- The parts of one message come within seconds. A part older than this is dropped.
local EXPIRE_SECONDS = 60

function PartCollector.New()
	return { senders = {} }
end

local function ForgetOld(open, now)
	for id, message in pairs(open) do
		if now - message.started > EXPIRE_SECONDS then
			open[id] = nil
		end
	end
end

local function Forget(collector, now)
	for sender, open in pairs(collector.senders) do
		ForgetOld(open, now)
		if next(open) == nil then
			collector.senders[sender] = nil
		end
	end
end

local function CountOf(map)
	local count = 0
	for _ in pairs(map) do
		count = count + 1
	end
	return count
end

local function DropOldest(open)
	local oldest
	for id, message in pairs(open) do
		if not oldest or message.started < open[oldest].started then
			oldest = id
		end
	end
	open[oldest] = nil
end

-- The open messages of a sender, or nil when too many senders have open messages.
local function OpenOf(collector, sender, now)
	if collector.senders[sender] then
		return collector.senders[sender]
	end
	Forget(collector, now)
	if CountOf(collector.senders) >= MAX_SENDERS then
		return nil
	end
	collector.senders[sender] = {}
	return collector.senders[sender]
end

local function Started(open, id, count, now)
	if open[id] and open[id].count == count then
		return open[id]
	end
	open[id] = nil
	if CountOf(open) >= OPEN_PER_SENDER then
		DropOldest(open)
	end
	open[id] = { count = count, parts = {}, got = 0, started = now, logged = true }
	return open[id]
end

local function Joined(message)
	local pieces = {}
	for n = 1, message.count do
		pieces[n] = message.parts[n]
	end
	return table.concat(pieces)
end

-- `part` is { id, number, count, text, logged }: part `number` of `count` of the message
-- `id`. Returns the whole text once its last part came, and whether every part came logged.
function PartCollector.Add(collector, sender, part, now)
	local open = OpenOf(collector, sender, now)
	if not open then
		return nil
	end
	-- A part with the same id from before a reload of the sender never joins a new one.
	ForgetOld(open, now)
	local message = Started(open, part.id, part.count, now)
	if not message.parts[part.number] then
		message.parts[part.number] = part.text
		message.got = message.got + 1
		message.logged = message.logged and part.logged ~= false
	end
	if message.got < message.count then
		return nil
	end
	open[part.id] = nil
	return Joined(message), message.logged
end
