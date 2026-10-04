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
