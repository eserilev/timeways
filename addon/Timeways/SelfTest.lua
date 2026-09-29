-- `/timeways test`: a round trip through Gnomish Relay to the story program and back
-- (TESTING.md). The requests only read the world, so the test changes nothing in it.
-- The result shows in the chat, and stays in the saved variables as `selfTest`.

local _, ns = ...

local SelfTest = {}
ns.SelfTest = SelfTest

local PREFIX = "|cffc8a064Timeways self-test|r: "
-- The search adds the places of the character, so the answer can cost one model call.
local QUESTION = "What is the news?"

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

-- The report of the test that runs, or nil.
local report

local function Check(name, ok, detail)
	report.checks[#report.checks + 1] = { name = name, ok = ok, detail = detail }
	local result = ok and "|cff40c040pass|r" or "|cffff4040fail|r"
	Say(name .. ": " .. result .. (detail and (" (" .. ns.Plain(detail) .. ")") or ""))
end

-- The first JSON line of the reply with this type, or nil.
local function Line(text, kind)
	for line in text:gmatch("[^\n]+") do
		local value = ns.Json.Decode(line)
		if type(value) == "table" and value.type == kind then
			return value
		end
	end
end

-- A Lua error here is a page that the book cannot show.
local function RendersAll(page)
	local journal = ns.Journal.FirstPage(page)
	for _, section in ipairs(ns.Journal.SECTIONS) do
		local ok, problem = pcall(ns.Journal.Render, journal, section)
		if not ok then
			return false, section .. ": " .. tostring(problem)
		end
	end
	return true
end

local function Answered()
	return true
end

local STEPS = {
	{
		name = "journal",
		kind = "journal",
		Input = function()
			return ns.Inputs.JournalAsked(0)
		end,
		Check = RendersAll,
	},
	{
		name = "lore",
		kind = "lore_answer",
		Input = function()
			return ns.Inputs.Question(time(), QUESTION)
		end,
		Check = Answered,
	},
}

local function Judge(step, status, text)
	if status ~= "done" then
		return false, tostring(text)
	end
	local value = Line(tostring(text), step.kind)
	if not value then
		return false, "no " .. step.kind .. " line in the reply"
	end
	return step.Check(value)
end

local function Finish()
	local passed = 0
	for _, check in ipairs(report.checks) do
		passed = passed + (check.ok and 1 or 0)
	end
	Say(string.format("%d of %d checks passed.", passed, #report.checks))
	report.ended = true
	report = nil
end

-- One step at a time: the next request goes out when the reply of the last one came.
local function Run(index)
	local step = STEPS[index]
	if not step then
		Finish()
		return
	end
	local text = ns.Outbox.Alone(step.Input())
	if not text then
		Check(step.name, false, "no character yet: wait for the login to end")
		Run(index + 1)
		return
	end
	local id = ns.Link.Send(text)
	if not id then
		Check(step.name, false, "the link did not take the message")
		Run(index + 1)
		return
	end
	ns.Link.Claim(id, function(status, reply)
		Check(step.name, Judge(step, status, reply))
		Run(index + 1)
	end)
end

function SelfTest.Start()
	if report then
		Say("A self-test runs already.")
		return
	end
	-- Saved at once, so a reply that never comes still leaves the steps before it.
	report = { at = time(), checks = {}, ended = false }
	ns.Saved().selfTest = report
	Say("Started. Each step waits for the bridge, so it can take a minute.")
	local missing = ns.Health.Missing()
	Check("client", missing == nil, missing and ("no " .. missing))
	Run(1)
end
