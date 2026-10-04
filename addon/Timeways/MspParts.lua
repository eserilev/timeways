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

MspParts.NewCollector = ns.PartCollector.New

-- Returns the whole text once its last part came, and whether every part came logged.
-- A logged part is decoded here, because Chomp escapes it part by part.
function MspParts.Add(collector, sender, part, logged, now)
	local session, number, count, text = Header(part)
	if not session then
		return nil
	end
	text = logged and ns.MspWire.Unescape(text) or text
	local piece = { id = session, number = number, count = count, text = text, logged = logged }
	return ns.PartCollector.Add(collector, sender, piece, now)
end
