-- The text of a story (GAMEPLAY.md 4.8): an optional title, and a body of paragraphs with
-- one "\n" between two of them. The wire, the saved variables, and the scroll share these
-- rules, and the desktop holds the same limits (`stories.rs`).

local _, ns = ...

local StoryText = {}
ns.StoryText = StoryText

StoryText.TITLE_LETTERS, StoryText.TITLE_BYTES = 60, 72
StoryText.BODY_LETTERS, StoryText.BODY_BYTES = 1000, 1200
StoryText.MAX_PARAGRAPHS = 20

StoryText.BAR = "Stories can't hold the || sign. Take it out and try again."
StoryText.ODD_SIGNS = "Some of these characters can't be sent. Take them out and try again."
StoryText.TOO_LONG = "Too long to send. Try a shorter version."
StoryText.TOO_MANY_PARAGRAPHS = "That's more than 20 paragraphs. Join some and try again."
StoryText.EMPTY = "Write your story first."

-- Each letter of valid UTF-8 has one byte that is no continuation byte.
function StoryText.Letters(text)
	return select(2, text:gsub("[^\128-\191]", ""))
end

local function Trimmed(line)
	return line:match("^%s*(.-)%s*$")
end

local function Lines(text)
	local lines = {}
	for line in (text .. "\n"):gmatch("([^\n]*)\n") do
		lines[#lines + 1] = line
	end
	return lines
end

-- The body of a typed text: each line trimmed, and the lines that are not empty joined by one
-- break. So a blank line and a single line break both give one paragraph break.
function StoryText.Body(typed)
	local kept = {}
	for _, line in ipairs(Lines(tostring(typed or ""):gsub("\r\n?", "\n"))) do
		line = Trimmed(line)
		if line ~= "" then
			kept[#kept + 1] = line
		end
	end
	return table.concat(kept, "\n")
end

function StoryText.Paragraphs(body)
	return Lines(body)
end

local function Fits(text, letters, bytes)
	return #text <= bytes and StoryText.Letters(text) <= letters
end

local function IsParagraph(paragraph)
	return paragraph ~= "" and Trimmed(paragraph) == paragraph and ns.TaskWire.IsCleanText(paragraph)
end

function StoryText.IsTitle(title)
	return type(title) == "string"
		and Fits(title, StoryText.TITLE_LETTERS, StoryText.TITLE_BYTES)
		and ns.TaskWire.IsCleanText(title)
end

-- The only control sign of a body is the break between two paragraphs.
function StoryText.IsBody(body)
	if type(body) ~= "string" or not Fits(body, StoryText.BODY_LETTERS, StoryText.BODY_BYTES) then
		return false
	end
	local paragraphs = StoryText.Paragraphs(body)
	if #paragraphs > StoryText.MAX_PARAGRAPHS then
		return false
	end
	for _, paragraph in ipairs(paragraphs) do
		if not IsParagraph(paragraph) then
			return false
		end
	end
	return true
end

-- The first sign that no wire takes, as its first and last byte, or nil. A `|`, a control
-- sign other than a break, a C1 control, a sign that the logged channel refuses, or a
-- broken letter.
function StoryText.BadSign(text)
	local at = 1
	for start, letter in text:gmatch("()([%z\1-\127\194-\244]?[\128-\191]*)") do
		local stop = start + #letter - 1
		if letter == "" then
			break
		end
		if letter ~= "\n" and not ns.TaskWire.IsCleanText(letter) then
			return start, stop
		end
		at = stop + 1
	end
	if at <= #text then
		return at, #text
	end
end

-- Why a story can't go, or nil. `title` and `body` are already trimmed.
function StoryText.Problem(title, body)
	if body == "" then
		return StoryText.EMPTY
	end
	local joined = title .. "\n" .. body
	if joined:find("|", 1, true) then
		return StoryText.BAR
	end
	if StoryText.BadSign(joined) then
		return StoryText.ODD_SIGNS
	end
	if not Fits(title, StoryText.TITLE_LETTERS, StoryText.TITLE_BYTES) then
		return StoryText.TOO_LONG
	end
	if not Fits(body, StoryText.BODY_LETTERS, StoryText.BODY_BYTES) then
		return StoryText.TOO_LONG
	end
	if #StoryText.Paragraphs(body) > StoryText.MAX_PARAGRAPHS then
		return StoryText.TOO_MANY_PARAGRAPHS
	end
end
