-- Cuts a task message into addon messages of at most 255 bytes, and puts the parts of a
-- peer back together (GAMEPLAY.md 4.7). A part is "<number>:<part>:<parts>:<text>".

local _, ns = ...

local TaskChunks = {}
ns.TaskChunks = TaskChunks

-- The limit of one addon message of the game.
TaskChunks.SIZE = 255
TaskChunks.MAX_PARTS = 16
local MAX_NUMBER = 9999
-- The longest header: "9999:16:16:".
local HEADER = 11
local PIECE = TaskChunks.SIZE - HEADER

-- A peer that sends more open messages than this loses its oldest one.
local OPEN_PER_SENDER = 4
local MAX_SENDERS = 32
-- The parts of one message come within seconds. A part older than this is dropped.
local EXPIRE_SECONDS = 60

local function IsContinuation(byte)
	return byte ~= nil and byte >= 128 and byte < 192
end

-- The last byte of the piece that starts at `first`. The logged channel takes only whole
-- UTF-8 letters, so a cut never falls inside one.
local function PieceEnd(text, first)
	local last = first + PIECE - 1
	while last > first and IsContinuation(text:byte(last + 1)) do
		last = last - 1
	end
	return last
end

local function Pieces(text)
	local pieces, first = {}, 1
	repeat
		local last = PieceEnd(text, first)
		pieces[#pieces + 1] = text:sub(first, last)
		first = last + 1
	until first > #text
	return pieces
end

-- Returns the parts, or nil for a text too long for the parts.
function TaskChunks.Split(text, number)
	if #text > TaskChunks.MAX_PARTS * PIECE then
		return nil
	end
	local pieces = Pieces(text)
	if #pieces > TaskChunks.MAX_PARTS then
		return nil
	end
	local parts = {}
	for n, piece in ipairs(pieces) do
		parts[n] = string.format("%d:%d:%d:%s", number, n, #pieces, piece)
	end
	return parts
end

function TaskChunks.NextNumber(number)
	return number % MAX_NUMBER + 1
end

-- Returns the number, the part, the count, and the text of a part, or nil.
local function Header(chunk)
	local number, part, count, text = chunk:match("^(%d%d?%d?%d?):(%d%d?):(%d%d?):(.*)$")
	number, part, count = tonumber(number), tonumber(part), tonumber(count)
	if not number or count < 1 or count > TaskChunks.MAX_PARTS or part < 1 or part > count then
		return nil
	end
	return number, part, count, text
end

function TaskChunks.NewCollector()
	return { senders = {} }
end

local function ForgetOld(open, now)
	for number, message in pairs(open) do
		if now - message.started > EXPIRE_SECONDS then
			open[number] = nil
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
	for number, message in pairs(open) do
		if not oldest or message.started < open[oldest].started then
			oldest = number
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

local function Joined(message)
	local pieces = {}
	for n = 1, message.count do
		pieces[n] = message.parts[n]
	end
	return table.concat(pieces)
end

local function Started(open, number, count, now)
	if open[number] and open[number].count == count then
		return open[number]
	end
	open[number] = nil
	if CountOf(open) >= OPEN_PER_SENDER then
		DropOldest(open)
	end
	open[number] = { count = count, parts = {}, got = 0, started = now }
	return open[number]
end

-- Returns the whole text once its last part came, or nil.
function TaskChunks.Add(collector, sender, chunk, now)
	local number, part, count, text = Header(chunk)
	local open = number and OpenOf(collector, sender, now)
	if not open then
		return nil
	end
	-- A part of the same number from before a reload of the peer never joins a new one.
	ForgetOld(open, now)
	local message = Started(open, number, count, now)
	if not message.parts[part] then
		message.parts[part] = text
		message.got = message.got + 1
	end
	if message.got < message.count then
		return nil
	end
	open[number] = nil
	return Joined(message)
end
