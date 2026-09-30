-- "Help me write this" on the form of a player task (GAMEPLAY.md 4.7): a model turns an idea
-- into a title, a text, and steps. The player stays the author and picks what to keep.

local _, ns = ...

local TaskDraftHelp = {}
ns.TaskDraftHelp = TaskDraftHelp

-- The relay takes an idea of at most 255 bytes. A letter outside ASCII takes up to 4.
local IDEA_LETTERS = 200
local IDEA_BYTES = 255

-- The state of the help: nil, "asking", "failed" (no answer came), or "empty" (the answer
-- held no draft). A draft that came waits in `suggestion`.
local state
local suggestion

function TaskDraftHelp.State()
	return state
end

function TaskDraftHelp.Suggestion()
	return suggestion
end

local function Changed()
	ns.JournalFrame.Refresh()
end

local function Failed()
	state = "failed"
	Changed()
end

-- The name of a real player never goes to a model (5.11).
local function Scrubbed(idea)
	return ns.TaskNames.WithoutNames(ns.TaskForm.Clean(idea))
end

-- A name that becomes "my friend" makes the idea longer, so the check runs after it.
local function IdeaProblem(idea)
	if #Scrubbed(idea) > IDEA_BYTES then
		return "That idea is too long. Try a shorter one."
	end
end

local function Ask(idea)
	idea = Scrubbed(idea)
	if idea == "" then
		return
	end
	state, suggestion = "asking", nil
	ns.Outbox.Add(ns.Inputs.DraftAsked(time(), idea), Failed)
	ns.Outbox.Flush()
	Changed()
end

function TaskDraftHelp.Open()
	if ns.Welcome.OpenIfNoApp() then
		return
	end
	ns.JournalFrame.Edit({
		title = "What's your idea?",
		hint = "Say it in plain words. Timeways turns it into a title, a task text, and steps the game can check.",
		text = "",
		limit = IDEA_LETTERS,
		bytes = IDEA_BYTES,
		problem = IdeaProblem,
		save = Ask,
	})
end

local GOALS = { place = true, npc = true, kill = true, item = true }

-- A step of the draft as a step of the form: "3 Rattlecage Soldier" is a count and a name.
local function FormStep(step)
	if type(step) ~= "table" or not GOALS[step.goal] or not ns.TaskWire.IsCleanText(step.target) then
		return nil
	end
	local name, count = step.target, 1
	if step.goal == "kill" or step.goal == "item" then
		name, count = ns.TaskForm.ParseItem(step.target)
	end
	if name and #name <= ns.TaskWire.LIMITS.target then
		return { kind = step.goal, target = name, count = count }
	end
end

local function CleanText(value, limit)
	return type(value) == "string" and value ~= "" and #value <= limit and ns.TaskWire.IsCleanText(value)
end

-- The model writes `$N` for the giver, as the idea did. Any other `$` is a code of the game
-- that shows as it is, so the draft drops.
local function WithGiver(text)
	local me = UnitName("player")
	if type(text) ~= "string" or type(me) ~= "string" or issecretvalue(me) then
		return nil
	end
	text = text:gsub("%$[Nn]", function()
		return me
	end)
	if text:find("$", 1, true) then
		return nil
	end
	return text
end

-- The same step twice becomes one, with both counts.
local function AddStep(steps, step)
	for _, known in ipairs(steps) do
		if known.kind == step.kind and known.target == step.target then
			known.count = math.min(known.count + step.count, ns.TaskWire.MAX_COUNT)
			return
		end
	end
	steps[#steps + 1] = step
end

-- The story program checked the draft against the world, and the bridge doubled each `|`.
-- So a field that breaks a rule of the addon drops the whole draft.
function TaskDraftHelp.Check(draft)
	if type(draft) ~= "table" then
		return nil
	end
	local limits = ns.TaskWire.LIMITS
	local title, text = WithGiver(draft.title), WithGiver(draft.text)
	if not CleanText(title, limits.title) or not CleanText(text, limits.text) then
		return nil
	end
	local steps = type(draft.steps) == "table" and draft.steps or {}
	if #steps > ns.TaskWire.MAX_STEPS then
		return nil
	end
	local checked = {}
	for _, step in ipairs(steps) do
		local formStep = FormStep(step)
		if not formStep then
			return nil
		end
		AddStep(checked, formStep)
	end
	return { title = title, text = text, steps = checked }
end

-- The reply `draft_answer`. With no draft, no model answered, or the draft broke a rule.
function TaskDraftHelp.Receive(answer)
	suggestion = TaskDraftHelp.Check(answer.draft)
	state = not suggestion and "empty" or nil
	Changed()
end

-- Every field stays editable after the player picks.
function TaskDraftHelp.Use()
	if not suggestion then
		return
	end
	local draft = ns.TaskForm.Draft()
	draft.title, draft.text = suggestion.title, suggestion.text
	draft.steps = suggestion.steps
	suggestion = nil
	Changed()
end

function TaskDraftHelp.Keep()
	suggestion, state = nil, nil
	Changed()
end
