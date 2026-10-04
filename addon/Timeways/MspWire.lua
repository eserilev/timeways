-- The commands of the Mary Sue Protocol (MSP), as LibMSP v32 writes and reads them
-- (GAMEPLAY.md 3.7.1): requests, fields, and versions. Pure functions, for the fuzzer.

local _, ns = ...

local MspWire = {}
ns.MspWire = MspWire

MspWire.PREFIX = "MSP2"
MspWire.PROTOCOL = "3"

local SEPARATOR = "`"
MspWire.MAX_COMMANDS = 64

-- The fields of a tooltip, in the order of the reply. A request for any of them, or for the
-- other fields that LibMSP adds to a tooltip, asks for "TT".
MspWire.TOOLTIP = { "VP", "VA", "NA", "NH", "NI", "NT", "RA", "CU", "FR", "FC" }
local IN_TOOLTIP = {}
for _, field in ipairs(MspWire.TOOLTIP) do
	IN_TOOLTIP[field] = true
end
for _, field in ipairs({ "RC", "CO", "IC", "PX", "PN" }) do
	IN_TOOLTIP[field] = true
end

function MspWire.InTooltip(field)
	return IN_TOOLTIP[field] == true
end

-- CRC32C, the version of LibMSP: polynomial 0x82F63B78, bits in reverse.
local CRC_TABLE = {}
for byte = 0, 255 do
	local crc = byte
	for _ = 1, 8 do
		local low = bit.band(crc, 1)
		crc = bit.rshift(crc, 1)
		if low == 1 then
			crc = bit.bxor(crc, 0x82F63B78)
		end
	end
	CRC_TABLE[byte] = crc
end

local WORD = 2 ^ 32

local function Crc(text)
	local crc = 0xFFFFFFFF
	for n = 1, #text do
		local index = bit.band(bit.bxor(crc, text:byte(n)), 0xFF)
		crc = bit.bxor(bit.rshift(crc, 8), CRC_TABLE[index])
	end
	return bit.bxor(crc, 0xFFFFFFFF) % WORD
end

-- Upper-case hex with no leading zero, in two halves, because "%X" of a number past 2^31
-- breaks where a C long has 32 bits. LibMSP writes 0 as no digit at all ("%.X").
local function Hex(number)
	local high, low = math.floor(number / 65536), number % 65536
	if high > 0 then
		return string.format("%X%04X", high, low)
	end
	return low > 0 and string.format("%X", low) or ""
end

-- An empty text has no version.
function MspWire.Version(text)
	if text == nil or text == "" then
		return ""
	end
	return Hex(Crc(text))
end

-- A logged message refuses "|", "\", and a line break, so Chomp escapes them as "~XX".
local ESCAPED = "[~|\\\n]"
local UNESCAPED = { [10] = true, [92] = true, [124] = true, [126] = true }

function MspWire.Escape(text)
	return (text:gsub(ESCAPED, function(letter)
		return string.format("~%02X", letter:byte())
	end))
end

-- Chomp decodes only the four escapes of Escape. Any other "~XX" stays as it is.
function MspWire.Unescape(text)
	return (
		text:gsub("~(%x%x)", function(hex)
			local byte = tonumber(hex, 16)
			if UNESCAPED[byte] then
				return string.char(byte)
			end
		end)
	)
end

-- A backtick ends a command, so LibMSP sends it as a quote.
local function Value(text)
	return (text:gsub(SEPARATOR, "'"))
end

-- "?NA", or "?DE1A2B3C4D" with the version that the asker has.
function MspWire.Request(field, version)
	return "?" .. field .. (version or "")
end

-- "!DE1A2B3C4D": the asker has this version already.
function MspWire.Same(field, version)
	return "!" .. field .. version
end

-- "NA1A2B3C4D:Mary Sue", or "NA" for an empty field.
function MspWire.Field(field, text)
	if text == nil or text == "" then
		return field
	end
	text = Value(text)
	return field .. MspWire.Version(text) .. ":" .. text
end

-- The fields of a tooltip as LibMSP sends them: each one with no version, then "TT" with
-- the version of them all. Returns the commands and that version.
function MspWire.Tooltip(fields)
	local parts = {}
	for _, field in ipairs(MspWire.TOOLTIP) do
		local text = fields[field]
		parts[#parts + 1] = (text and text ~= "") and (field .. ":" .. Value(text)) or field
	end
	local contents = table.concat(parts, SEPARATOR)
	local version = MspWire.Version(contents)
	return contents .. SEPARATOR .. "TT" .. version, version
end

function MspWire.Join(commands)
	return table.concat(commands, SEPARATOR)
end

local ACTIONS = { [""] = "field", ["?"] = "request", ["!"] = "same" }

-- One command: { kind = "request" | "same" | "field", field, version, text }. A version
-- of "0" counts as none, as in LibMSP, and so does an empty text.
local function Command(text)
	local action, field, version, rest = text:match("^(%p?)(%u%u)(%x*)(.*)$")
	local kind = action and ACTIONS[action]
	if not kind then
		return nil
	end
	local value = rest:match("^:(.*)$")
	if rest ~= "" and not value then
		return nil
	end
	if version == "0" then
		version = ""
	end
	return { kind = kind, field = field, version = version, text = value ~= "" and value or nil }
end

-- The commands of a whole message, in their order. A broken command is skipped, as LibMSP
-- skips it.
function MspWire.Commands(message)
	local commands = {}
	for text in (message .. SEPARATOR):gmatch("([^" .. SEPARATOR .. "]*)" .. SEPARATOR) do
		local command = Command(text)
		if command then
			commands[#commands + 1] = command
		end
		if #commands >= MspWire.MAX_COMMANDS then
			break
		end
	end
	return commands
end
