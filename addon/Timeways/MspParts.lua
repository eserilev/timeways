-- Cuts an MSP message into addon messages, and puts the parts of a player back together,
-- in the format of Chomp, the message library of LibMSP (GAMEPLAY.md 3.7.1). Each part
-- starts with 12 hex digits: the flags, the session, the number of the part, and the count.

local _, ns = ...

local MspParts = {}
ns.MspParts = MspParts

local SIZE = 255
local HEADER = 12
local PIECE = SIZE - HEADER
-- Chomp v16 and codec 2. Chomp drops a part without them.
local FLAGS = 0x00A
-- These two and one unused flag that Chomp accepts. A part with another flag is serialized
-- Lua data, a broadcast to a group, or a format that Chomp itself refuses.
local ACCEPTED_FLAGS = 0x02A
local SESSIONS = 4096
MspParts.MAX_PARTS = 64

-- A sender with more open messages than this loses its oldest one.
local OPEN_PER_SENDER = 4
local MAX_SENDERS = 32
-- The parts of one message come within seconds. A part older than this is dropped.
local EXPIRE_SECONDS = 60

local ESCAPE = 126 -- "~"

-- The length of a piece that ends before `last` and splits no escape "~XX" and no letter of
-- several bytes, as Chomp.SafeSubString does.
local function PieceEnd(text, first)
	local last = first + PIECE - 1
	if last >= #text then
		return #text
	end
	local b3, b2, b1 = text:byte(last - 2, last)
	if b1 == ESCAPE or (b1 >= 194 and b1 <= 244) then
		return last - 1
	elseif b2 == ESCAPE or (b2 >= 224 and b2 <= 244) then
		return last - 2
	elseif b3 >= 240 and b3 <= 244 then
		return last - 3
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

-- The parts of a text, or nil for a text too long for MAX_PARTS parts.
function MspParts.Split(text, session)
	local pieces = Pieces(text)
	if #pieces > MspParts.MAX_PARTS then
		return nil
	end
	local parts = {}
	for n, piece in ipairs(pieces) do
		parts[n] = string.format("%03X%03X%03X%03X%s", FLAGS, session, n, #pieces, piece)
	end
	return parts
end

function MspParts.NextSession(session)
	return (session + 1) % SESSIONS
end

-- Returns the session, the part, the count, and the text of a part, or nil.
local function Header(part)
	local flags, session, number, count, text = part:match("^(%x%x%x)(%x%x%x)(%x%x%x)(%x%x%x)(.*)$")
	if not flags then
		return nil
	end
	flags, session, number, count =
		tonumber(flags, 16), tonumber(session, 16), tonumber(number, 16), tonumber(count, 16)
	if bit.band(flags, bit.bnot(ACCEPTED_FLAGS)) ~= 0 then
		return nil
	end
	if count < 1 or count > MspParts.MAX_PARTS or number < 1 or number > count then
		return nil
	end
	return session, number, count, text
end

function MspParts.NewCollector()
	return { senders = {} }
end

local function ForgetOld(open, now)
	for session, message in pairs(open) do
		if now - message.started > EXPIRE_SECONDS then
			open[session] = nil
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

-- The open messages of a sender, or nil when too many senders have open messages.
local function OpenOf(collector, sender, now)
	if collector.senders[sender] then
		return collector.senders[sender]
	end
	for name, open in pairs(collector.senders) do
		ForgetOld(open, now)
		if next(open) == nil then
			collector.senders[name] = nil
		end
	end
	if CountOf(collector.senders) >= MAX_SENDERS then
		return nil
	end
	collector.senders[sender] = {}
	return collector.senders[sender]
end

local function DropOldest(open)
	local oldest
	for session, message in pairs(open) do
		if not oldest or message.started < open[oldest].started then
			oldest = session
		end
	end
	open[oldest] = nil
end

local function Started(open, session, count, now)
	if open[session] and open[session].count == count then
		return open[session]
	end
	open[session] = nil
	if CountOf(open) >= OPEN_PER_SENDER then
		DropOldest(open)
	end
	open[session] = { count = count, parts = {}, got = 0, started = now, logged = true }
	return open[session]
end

local function Joined(message)
	local pieces = {}
	for n = 1, message.count do
		pieces[n] = message.parts[n]
	end
	return table.concat(pieces)
end

-- Returns the whole text once its last part came, and whether every part came logged.
-- A logged part is decoded here, because Chomp escapes it part by part.
function MspParts.Add(collector, sender, part, logged, now)
	local session, number, count, text = Header(part)
	local open = session and OpenOf(collector, sender, now)
	if not open then
		return nil
	end
	ForgetOld(open, now)
	local message = Started(open, session, count, now)
	if not message.parts[number] then
		message.parts[number] = logged and ns.MspWire.Unescape(text) or text
		message.got = message.got + 1
		message.logged = message.logged and logged
	end
	if message.got < message.count then
		return nil
	end
	open[session] = nil
	return Joined(message), message.logged
end
