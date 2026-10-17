-- Text that is valid UTF-8. The desktop and the logged addon channel read nothing else.

local _, ns = ...

local Utf8 = {}
ns.Utf8 = Utf8

-- The valid UTF-8 letters past ASCII, one pattern for each kind of first byte.
local LETTERS = {
	"[\194-\223][\128-\191]",
	"\224[\160-\191][\128-\191]",
	"[\225-\236\238\239][\128-\191][\128-\191]",
	"\237[\128-\159][\128-\191]",
	"\240[\144-\191][\128-\191][\128-\191]",
	"[\241-\243][\128-\191][\128-\191][\128-\191]",
	"\244[\128-\143][\128-\191][\128-\191]",
}

-- The first byte of each pattern says its kind, so a letter that one pattern takes out never
-- joins the bytes around it into a letter of another pattern.
function Utf8.IsValid(text)
	for _, letter in ipairs(LETTERS) do
		text = text:gsub(letter, "a")
	end
	return not text:find("[\128-\255]")
end

-- The start of a text with at most this many letters and bytes, cut between letters.
function Utf8.Cut(text, maxLetters, maxBytes)
	local letters, last = 0, 0
	for start, letter in text:gmatch("()([%z\1-\127\194-\244][\128-\191]*)") do
		local stop = start + #letter - 1
		if letters >= maxLetters or stop > maxBytes then
			break
		end
		letters, last = letters + 1, stop
	end
	return text:sub(1, last)
end

-- The size of a letter that starts with this byte, and the range of its second byte. Nil for
-- a byte that starts no letter.
local function Expected(lead)
	if lead >= 0xC2 and lead <= 0xDF then
		return 2, 0x80, 0xBF
	elseif lead == 0xE0 then
		return 3, 0xA0, 0xBF
	elseif lead == 0xED then
		return 3, 0x80, 0x9F
	elseif lead >= 0xE1 and lead <= 0xEF then
		return 3, 0x80, 0xBF
	elseif lead == 0xF0 then
		return 4, 0x90, 0xBF
	elseif lead >= 0xF1 and lead <= 0xF3 then
		return 4, 0x80, 0xBF
	elseif lead == 0xF4 then
		return 4, 0x80, 0x8F
	end
end

-- The bytes of the letter at `at` that are valid: all of them for a whole letter, and
-- fewer for a broken one, at least 1.
local function ValidBytes(text, at)
	local size, low, high = Expected(text:byte(at))
	local second = text:byte(at + 1)
	if not size or not second or second < low or second > high then
		return 1, false
	end
	local good = 2
	while good < size do
		local byte = text:byte(at + good)
		if not byte or byte < 0x80 or byte > 0xBF then
			return good, false
		end
		good = good + 1
	end
	return size, true
end

-- The text with each broken part as U+FFFD, as Rust's `String::from_utf8_lossy` reads it:
-- one U+FFFD for the longest start of a letter that could still be valid.
function Utf8.Lossy(text)
	if Utf8.IsValid(text) then
		return text
	end
	local parts, at = {}, 1
	while at <= #text do
		local ascii = text:match("^[%z\1-\127]+", at)
		if ascii then
			parts[#parts + 1] = ascii
			at = at + #ascii
		else
			local size, whole = ValidBytes(text, at)
			parts[#parts + 1] = whole and text:sub(at, at + size - 1) or "\239\191\189"
			at = at + size
		end
	end
	return table.concat(parts)
end
