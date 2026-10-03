-- The messages of player tasks between two players with Timeways (GAMEPLAY.md 4.7). A
-- message comes from another computer, so each field is checked here: a known type, the
-- exact number of fields, and each text and number inside its bounds.

local _, ns = ...

local TaskWire = {}
ns.TaskWire = TaskWire

-- The first field of each message. A message of another version is dropped.
local VERSION = "1"
local SEPARATOR = ";"

TaskWire.MAX_STEPS = 5
TaskWire.MAX_COUNT = 250
TaskWire.LIMITS = { title = 60, text = 400, reward = 200, target = 96, zone = 96 }
-- An "other" step is a line that the giver wrote and the game can't check. The doer marks
-- it done, and the giver decides at the turn-in.
TaskWire.STEP_KINDS = { "place", "npc", "kill", "meet", "item", "other" }

local KINDS = {}
for _, kind in ipairs(TaskWire.STEP_KINDS) do
	KINDS[kind] = true
end

local VERDICTS = { done = true, notyet = true }

-- A time past this is no time of this century.
local MAX_TIME = 4294967295

-- `%` and the separator are the only bytes that need an escape: a text with `|` or a
-- control character is refused on both ends.
local function Escape(text)
	return (text:gsub("[%%;]", function(c)
		return string.format("%%%02X", c:byte())
	end))
end

local function Unescape(field)
	return (field:gsub("%%(%x%x)", function(hex)
		return string.char(tonumber(hex, 16))
	end))
end

-- A `|` starts a WoW escape, such as a color or a fake link, so no text of a peer holds one.
function TaskWire.IsCleanText(text)
	return type(text) == "string" and not text:find("[%c|]")
end

local function Text(limit, empty)
	return function(field)
		local text = Unescape(field)
		local fits = #text <= limit and (empty or text ~= "")
		if fits and TaskWire.IsCleanText(text) then
			return text
		end
	end
end

local function Whole(low, high)
	return function(field)
		local number = #field <= 10 and field:match("^%d+$") and tonumber(field)
		if number and number >= low and number <= high then
			return number
		end
	end
end

local function Id(field)
	if #field <= 16 and field:match("^%w+$") then
		return field
	end
end

local function Kind(field)
	if KINDS[field] then
		return field
	end
end

local function Verdict(field)
	if VERDICTS[field] then
		return field
	end
end

local READERS = {
	id = Id,
	title = Text(TaskWire.LIMITS.title),
	text = Text(TaskWire.LIMITS.text, true),
	reward = Text(TaskWire.LIMITS.reward, true),
	kind = Kind,
	target = Text(TaskWire.LIMITS.target),
	count = Whole(1, TaskWire.MAX_COUNT),
	index = Whole(1, TaskWire.MAX_STEPS),
	at = Whole(0, MAX_TIME),
	zone = Text(TaskWire.LIMITS.zone, true),
	verdict = Verdict,
}

-- A group is a list of items with the same fields, sent as its length and then the items.
local GROUPS = {
	steps = { fields = { "kind", "target", "count" }, least = 1 },
	claims = { fields = { "index", "at", "zone" }, least = 0 },
}

-- The fields of each type, in order.
local SCHEMAS = {
	hello = {},
	here = {},
	offer = { "id", "title", "text", "reward", "steps" },
	accept = { "id" },
	decline = { "id" },
	block = { "id" },
	cancel = { "id" },
	step = { "id", "index", "at", "zone" },
	turnin = { "id", "claims" },
	result = { "id", "verdict" },
	-- A story about the player who gets it (4.8), and its answer.
	story = { "id", "text" },
	story_accept = { "id" },
	story_decline = { "id" },
}

local function EncodeValue(value)
	return type(value) == "number" and string.format("%d", value) or Escape(value)
end

local function EncodeGroup(fields, items, group)
	fields[#fields + 1] = tostring(#items)
	for _, item in ipairs(items) do
		for _, name in ipairs(group.fields) do
			fields[#fields + 1] = EncodeValue(item[name])
		end
	end
end

-- `message` is { type, and the fields of its type }. The caller builds it from checked
-- values, so a wrong field is a bug and raises an error.
function TaskWire.Encode(message)
	local schema = SCHEMAS[message.type]
	assert(schema, "unknown task message type")
	local fields = { VERSION, message.type }
	for _, name in ipairs(schema) do
		if GROUPS[name] then
			EncodeGroup(fields, message[name], GROUPS[name])
		else
			fields[#fields + 1] = EncodeValue(message[name])
		end
	end
	return table.concat(fields, SEPARATOR)
end

local function Split(text)
	local fields = {}
	for field in (text .. SEPARATOR):gmatch("([^;]*);") do
		fields[#fields + 1] = field
	end
	return fields
end

-- Returns the item and the next position, or nil.
local function ReadItem(fields, at, group)
	local item = {}
	for _, name in ipairs(group.fields) do
		local value = fields[at] and READERS[name](fields[at])
		if not value then
			return nil
		end
		item[name], at = value, at + 1
	end
	return item, at
end

-- Returns the list and the next position, or nil.
local function ReadGroup(fields, at, group)
	local count = fields[at] and Whole(group.least, TaskWire.MAX_STEPS)(fields[at])
	if not count then
		return nil
	end
	local items = {}
	at = at + 1
	for n = 1, count do
		items[n], at = ReadItem(fields, at, group)
		if not items[n] then
			return nil
		end
	end
	return items, at
end

local function ReadField(fields, at, name)
	if GROUPS[name] then
		return ReadGroup(fields, at, GROUPS[name])
	end
	local value = fields[at] and READERS[name](fields[at])
	if value then
		return value, at + 1
	end
end

-- Returns the message, or nil and the reason.
function TaskWire.Decode(text)
	if type(text) ~= "string" then
		return nil, "not a text"
	end
	local fields = Split(text)
	if fields[1] ~= VERSION then
		return nil, "unknown version"
	end
	local schema = SCHEMAS[fields[2]]
	if not schema then
		return nil, "unknown type"
	end
	local message, at = { type = fields[2] }, 3
	for _, name in ipairs(schema) do
		message[name], at = ReadField(fields, at, name)
		if not message[name] then
			return nil, "bad " .. name
		end
	end
	if at ~= #fields + 1 then
		return nil, "too many fields"
	end
	return message
end
