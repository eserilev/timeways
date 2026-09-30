-- "Help me write this" on the form of a player task (GAMEPLAY.md 4.7): a model turns an idea
-- into a title, a text, and steps. The player stays the author and picks what to keep.

local _, ns = ...

local TaskDraftHelp = {}
ns.TaskDraftHelp = TaskDraftHelp

-- The relay takes an idea of at most 255 bytes. A letter outside ASCII takes up to 4.
local IDEA_LETTERS = 200
local IDEA_BYTES = 255

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

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

local function Changed()
	ns.JournalFrame.Refresh()
end

local function Failed()
	state = "failed"
	Changed()
end

local function Ask(idea)
	idea = TaskDraftHelp.WithoutNames(ns.TaskForm.Clean(idea))
	if idea == "" then
		return
	end
	if #idea > IDEA_BYTES then
		Say("That idea is too long. Try a shorter one.")
		return
	end
	state, suggestion = "asking", nil
	ns.Outbox.Add(ns.Inputs.DraftAsked(time(), idea), Failed)
	ns.Outbox.Flush()
	Changed()
end

function TaskDraftHelp.Open()
	ns.JournalFrame.Edit({
		title = "What's your idea?",
		hint = "Say it in plain words. Timeways turns it into a title, a task text, and steps the game can check.",
		text = "",
		limit = IDEA_LETTERS,
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

-- The story program checked the draft against the world, and the bridge doubled each `|`.
-- So a field that breaks a rule of the addon drops the whole draft.
function TaskDraftHelp.Check(draft)
	local limits = ns.TaskWire.LIMITS
	if type(draft) ~= "table" or not CleanText(draft.title, limits.title) or not CleanText(draft.text, limits.text) then
		return nil
	end
	local steps = type(draft.steps) == "table" and draft.steps or {}
	if #steps > ns.TaskWire.MAX_STEPS then
		return nil
	end
	local checked = {}
	for n, step in ipairs(steps) do
		checked[n] = FormStep(step)
		if not checked[n] then
			return nil
		end
	end
	return { title = draft.title, text = draft.text, steps = checked }
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
