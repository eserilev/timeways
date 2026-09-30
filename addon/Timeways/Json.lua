-- JSON for the lines of the story program: objects, arrays, strings, whole numbers,
-- true, false, and null. WoW has no JSON of its own.

local _, ns = ...

local Json = {}
ns.Json = Json

local ESCAPES = { ['"'] = '\\"', ["\\"] = "\\\\", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" }

local function EncodeString(s)
	return '"' .. s:gsub('[%c"\\]', function(c)
		return ESCAPES[c] or string.format("\\u%04x", c:byte())
	end) .. '"'
end

-- An object with string keys, in sorted order, so one input always gives one line. A value
-- that is a table is an object too.
function Json.Encode(object)
	local keys = {}
	for key in pairs(object) do
		keys[#keys + 1] = key
	end
	table.sort(keys)
	local fields = {}
	for _, key in ipairs(keys) do
		local value = object[key]
		local encoded
		if type(value) == "string" then
			encoded = EncodeString(value)
		elseif type(value) == "number" and value % 1 == 0 then
			encoded = string.format("%d", value)
		elseif type(value) == "table" then
			encoded = Json.Encode(value)
		else
			error("Json.Encode: " .. key .. " is not a string, a whole number, or an object")
		end
		fields[#fields + 1] = EncodeString(key) .. ":" .. encoded
	end
	return "{" .. table.concat(fields, ",") .. "}"
end

local Parse -- forward, for the recursion of arrays and objects

local function Fail(at, what)
	error(string.format("%s at %d", what, at), 0)
end

local function SkipSpace(s, i)
	return s:find("[^ \t\r\n]", i) or #s + 1
end

local UNESCAPES = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" }

local function Utf8(code)
	if code < 0x80 then
		return string.char(code)
	elseif code < 0x800 then
		return string.char(0xC0 + math.floor(code / 0x40), 0x80 + code % 0x40)
	end
	return string.char(0xE0 + math.floor(code / 0x1000), 0x80 + math.floor(code / 0x40) % 0x40, 0x80 + code % 0x40)
end

local function ParseString(s, i)
	local parts = {}
	i = i + 1
	while true do
		local stop = s:find('["\\]', i)
		if not stop then
			Fail(i, "an unclosed string")
		end
		parts[#parts + 1] = s:sub(i, stop - 1)
		if s:sub(stop, stop) == '"' then
			return table.concat(parts), stop + 1
		end
		local escape = s:sub(stop + 1, stop + 1)
		if escape == "u" then
			local code = tonumber(s:sub(stop + 2, stop + 5):match("^%x%x%x%x$") or "", 16)
			-- The story program escapes only control characters, so a surrogate never comes.
			if not code or (code >= 0xD800 and code <= 0xDFFF) then
				Fail(stop, "a bad \\u escape")
			end
			parts[#parts + 1] = Utf8(code)
			i = stop + 6
		elseif UNESCAPES[escape] then
			parts[#parts + 1] = UNESCAPES[escape]
			i = stop + 2
		else
			Fail(stop, "a bad escape")
		end
	end
end

local function ParseNumber(s, i)
	local text = s:match("^-?%d+%.?%d*[eE]?[-+]?%d*", i)
	local number = text and tonumber(text)
	if not number then
		Fail(i, "a bad number")
	end
	return number, i + #text
end

local function ParseArray(s, i)
	local array = {}
	i = SkipSpace(s, i + 1)
	if s:sub(i, i) == "]" then
		return array, i + 1
	end
	while true do
		array[#array + 1], i = Parse(s, i)
		i = SkipSpace(s, i)
		local c = s:sub(i, i)
		if c == "]" then
			return array, i + 1
		elseif c ~= "," then
			Fail(i, "a missing comma in an array")
		end
		i = i + 1
	end
end

local function ParseObject(s, i)
	local object = {}
	i = SkipSpace(s, i + 1)
	if s:sub(i, i) == "}" then
		return object, i + 1
	end
	while true do
		if s:sub(i, i) ~= '"' then
			Fail(i, "a missing key")
		end
		local key
		key, i = ParseString(s, i)
		i = SkipSpace(s, i)
		if s:sub(i, i) ~= ":" then
			Fail(i, "a missing colon")
		end
		object[key], i = Parse(s, i + 1)
		i = SkipSpace(s, i)
		local c = s:sub(i, i)
		if c == "}" then
			return object, i + 1
		elseif c ~= "," then
			Fail(i, "a missing comma in an object")
		end
		i = SkipSpace(s, i + 1)
	end
end

local LITERALS = { ["true"] = true, ["false"] = false }

-- `null` gives nil, so a null field is an absent field.
Parse = function(s, i)
	i = SkipSpace(s, i)
	local c = s:sub(i, i)
	if c == "{" then
		return ParseObject(s, i)
	elseif c == "[" then
		return ParseArray(s, i)
	elseif c == '"' then
		return ParseString(s, i)
	elseif c == "-" or c:match("%d") then
		return ParseNumber(s, i)
	end
	for word, value in pairs(LITERALS) do
		if s:sub(i, i + #word - 1) == word then
			return value, i + #word
		end
	end
	if s:sub(i, i + 3) == "null" then
		return nil, i + 4
	end
	Fail(i, "an unexpected character")
end

-- Returns the value, or nil and the reason. Text from outside never raises an error here.
function Json.Decode(s)
	local ok, value, stop = pcall(Parse, s, 1)
	if not ok then
		return nil, tostring(value)
	end
	if SkipSpace(s, stop) <= #s then
		return nil, string.format("text after the value at %d", stop)
	end
	return value
end
