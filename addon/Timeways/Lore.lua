-- `/lore <question>`, and the answer in the chat window (GAMEPLAY.md 3.1).

local _, ns = ...

local Lore = {}
ns.Lore = Lore

local PREFIX = "|cffc8a064Timeways|r: "

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

function Lore.Ask(question)
	question = question:match("^%s*(.-)%s*$")
	if question == "" then
		Say("Ask a question, for example: /lore why is this tower in ruins?")
		return
	end
	local input = ns.Inputs.Question(time(), question, ns.Units.NpcName("target"))
	if not ns.Outbox.Fits(input) then
		Say("That question is too long. Try a shorter one.")
		return
	end
	ns.Outbox.Add(input)
	ns.Outbox.Flush()
end

-- The answer cites its passages as "[1]". The player reads the story, not the sources.
local function WithoutCitations(text)
	return (text:gsub("%s*%[%d+%]", ""))
end

-- One chat message taller than the chat window gets cut at the top, and the window cannot
-- scroll inside a message. So a long text goes out as short lines, cut after a sentence.
local LINE_CHARS = 150

local function SayLong(text)
	local line = ""
	for sentence in text:gmatch("[^%.!?]*[%.!?]*%s*") do
		if line ~= "" and #line + #sentence > LINE_CHARS then
			Say(line:match("^(.-)%s*$"))
			line = ""
		end
		line = line .. sentence
	end
	if line:match("%S") then
		Say(line:match("^(.-)%s*$"))
	end
end

-- With no text, no model answered, and the passages show as they are (GAMEPLAY.md 5.6).
function Lore.Show(answer)
	local passages = type(answer.passages) == "table" and answer.passages or {}
	if type(answer.text) == "string" then
		SayLong(ns.Plain(WithoutCitations(answer.text)))
		return
	end
	if #passages == 0 then
		Say("Nobody here knows.")
	end
	for _, passage in ipairs(passages) do
		if type(passage) == "table" and type(passage.text) == "string" then
			SayLong(ns.Plain(passage.text))
		end
	end
end
