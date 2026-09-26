-- `/lore <question>`, and the answer in the chat window (GAMEPLAY.md 3.1).

local _, ns = ...

local Lore = {}
ns.Lore = Lore

local PREFIX = "|cffc8a064Timeways|r: "

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

-- The name of a player never leaves the computer (GAMEPLAY.md 5.11).
local function TargetName()
	if UnitExists("target") and not UnitIsPlayer("target") then
		return UnitName("target")
	end
end

function Lore.Ask(question)
	question = question:match("^%s*(.-)%s*$")
	if question == "" then
		Say("Ask a question, for example: /lore why is this tower in ruins?")
		return
	end
	local input = ns.Inputs.Question(question, TargetName())
	if not ns.Outbox.Fits(input) then
		Say("That question is too long.")
		return
	end
	ns.Outbox.Add(input)
	ns.Outbox.Flush()
end

-- The number stays the one that the answer cites, also when an entry before it is broken.
local function ShowPassage(n, passage, text)
	local source = type(passage.source) == "string" and passage.source:gsub("^https?://", "") or "?"
	if type(text) == "string" then
		Say(string.format("[%d] %s", n, ns.Plain(source)))
	elseif type(passage.text) == "string" then
		Say(string.format("[%d] %s (%s)", n, ns.Plain(passage.text), ns.Plain(source)))
	end
end

-- With no text, no model answered, and the passages show as they are (GAMEPLAY.md 5.6).
function Lore.Show(answer)
	local passages = type(answer.passages) == "table" and answer.passages or {}
	if type(answer.text) == "string" then
		Say(ns.Plain(answer.text))
	elseif #passages == 0 then
		Say("Nobody here knows.")
	end
	for n, passage in ipairs(passages) do
		if type(passage) == "table" then
			ShowPassage(n, passage, answer.text)
		end
	end
end
