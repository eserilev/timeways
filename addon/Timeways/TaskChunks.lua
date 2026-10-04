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

TaskChunks.NewCollector = ns.PartCollector.New

-- Returns the whole text once its last part came, or nil.
function TaskChunks.Add(collector, sender, chunk, now)
	local number, part, count, text = Header(chunk)
	if not number then
		return nil
	end
	local piece = { id = number, number = part, count = count, text = text }
	return (ns.PartCollector.Add(collector, sender, piece, now))
end
