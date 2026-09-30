-- "Help me write this" on the form of a player task (GAMEPLAY.md 4.7): a model turns an idea
-- into a title, a text, and steps. The player stays the author and picks what to keep.
-- The answer needs the reply type `draft_answer`, which the relay does not have yet, so the
-- button stays hidden until `enabled` is true.

local _, ns = ...

local TaskDraftHelp = {}
ns.TaskDraftHelp = TaskDraftHelp

TaskDraftHelp.enabled = false

local IDEA_LIMIT = 300

-- The last suggestion of the model, checked: { title, text, steps }.
local suggestion

function TaskDraftHelp.Available()
	return TaskDraftHelp.enabled
end

function TaskDraftHelp.Suggestion()
	return suggestion
end

local function Word(name)
	return "%f[%w]" .. name:gsub("%W", "%%%0") .. "%f[%W]"
end

-- The name of a real player never goes to a model (5.11): the players who can get a task
-- become "my friend", and your own name becomes `$N`.
function TaskDraftHelp.WithoutNames(idea)
	for _, recipient in ipairs(ns.PlayerTasks.Recipients()) do
		idea = idea:gsub(Word(ns.TaskPeople.Short(recipient.name)), "my friend")
	end
	local me = UnitName("player")
	if type(me) == "string" and not issecretvalue(me) then
		idea = idea:gsub(Word(me), "$N")
	end
	return idea
end

local function Ask(idea)
	idea = TaskDraftHelp.WithoutNames(ns.TaskForm.Clean(idea))
	if idea == "" then
		return
	end
	suggestion = nil
	ns.Outbox.Add({ type = "draft_asked", at = time(), idea = idea })
	ns.Outbox.Flush()
end

function TaskDraftHelp.Open()
	ns.JournalFrame.Edit({
		title = "What's your idea?",
		hint = "Say it in plain words. Timeways turns it into a title, a task text, and steps.",
		text = "",
		limit = IDEA_LIMIT,
		save = Ask,
	})
end

local function CleanText(value, limit)
	return type(value) == "string" and value ~= "" and #value <= limit and ns.TaskWire.IsCleanText(value)
end

local function CleanStep(step)
	local kinds = {}
	for _, kind in ipairs(ns.TaskWire.STEP_KINDS) do
		kinds[kind] = true
	end
	local count = type(step) == "table" and step.count
	local whole = type(count) == "number" and count % 1 == 0 and count >= 1 and count <= ns.TaskWire.MAX_COUNT
	return whole and kinds[step.kind] and CleanText(step.target, ns.TaskWire.LIMITS.target)
end

-- A model's answer is hostile text: a field that breaks a rule drops the whole answer.
function TaskDraftHelp.Check(answer)
	local limits = ns.TaskWire.LIMITS
	if not CleanText(answer.title, limits.title) or not CleanText(answer.text, limits.text) then
		return nil
	end
	local steps = type(answer.steps) == "table" and answer.steps or {}
	if #steps > ns.TaskWire.MAX_STEPS then
		return nil
	end
	local checked = {}
	for n, step in ipairs(steps) do
		if not CleanStep(step) then
			return nil
		end
		checked[n] = { kind = step.kind, target = step.target, count = step.count }
	end
	return { title = answer.title, text = answer.text, steps = checked }
end

-- The reply `draft_answer`.
function TaskDraftHelp.Receive(answer)
	suggestion = TaskDraftHelp.Check(answer)
	ns.JournalFrame.Refresh()
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
	ns.JournalFrame.Refresh()
end

function TaskDraftHelp.Keep()
	suggestion = nil
	ns.JournalFrame.Refresh()
end
