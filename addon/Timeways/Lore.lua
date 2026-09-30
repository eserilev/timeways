-- `/lore <question>`, and the answers of this session (GAMEPLAY.md 3.1). The lore book
-- shows them.

local _, ns = ...

local Lore = {}
ns.Lore = Lore

local PREFIX = "|cffc8a064Timeways|r: "

-- Enough to look back over a talk, and few enough that the book never grows.
local MAX_KEPT = 10

-- Oldest first. Each entry is { question, state, text }. The state is "asking", "answered",
-- or "failed". An answer with no text means that nobody knows.
local entries = {}

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

function Lore.Entries()
	return entries
end

local function Keep(entry)
	entries[#entries + 1] = entry
	while #entries > MAX_KEPT do
		table.remove(entries, 1)
	end
end

local function Failed(entry)
	return function()
		entry.state = "failed"
		ns.LoreBook.Refresh()
	end
end

-- With no question, `/lore` opens the book again.
function Lore.Ask(question)
	question = question:match("^%s*(.-)%s*$")
	if question == "" and #entries > 0 then
		ns.LoreBook.Open(entries[#entries])
		return
	end
	if question == "" then
		Say("Ask a question, for example: /lore why is this tower in ruins?")
		return
	end
	local input = ns.Inputs.Question(time(), question, ns.Units.NpcName("target"))
	if not ns.Outbox.Fits(input) then
		Say("That question is too long. Try a shorter one.")
		return
	end
	if ns.Welcome.OpenIfNoApp() then
		return
	end
	local entry = { question = question, state = "asking" }
	Keep(entry)
	ns.Outbox.Add(input, Failed(entry))
	ns.Outbox.Flush()
	ns.LoreBook.Open(entry)
end

-- The answer cites its passages as "[1]". The player reads the story, not the sources.
local function WithoutCitations(text)
	return (text:gsub("%s*%[%d+%]", ""))
end

-- With no text, no model answered, and the passages show as they are (GAMEPLAY.md 5.6).
local function AnswerText(answer)
	if type(answer.text) == "string" then
		return ns.Plain(WithoutCitations(answer.text))
	end
	local paragraphs = {}
	for _, passage in ipairs(type(answer.passages) == "table" and answer.passages or {}) do
		if type(passage) == "table" and type(passage.text) == "string" then
			paragraphs[#paragraphs + 1] = ns.Plain(passage.text)
		end
	end
	return #paragraphs > 0 and table.concat(paragraphs, "\n\n") or nil
end

-- The desktop answers the questions in order, so an answer belongs to the oldest question
-- that waits. An answer to a question from before a reload has no question here.
local function Waiting()
	for _, entry in ipairs(entries) do
		if entry.state == "asking" then
			return entry
		end
	end
	local entry = { state = "asking" }
	Keep(entry)
	return entry
end

function Lore.Show(answer)
	local entry = Waiting()
	entry.state = "answered"
	entry.text = AnswerText(answer)
	if not ns.LoreBook.IsShown() then
		Say("Your answer is ready. Type /lore to read it.")
	end
	ns.LoreBook.Refresh()
end
