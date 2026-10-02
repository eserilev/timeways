-- The task that you write for another player (GAMEPLAY.md 4.7). You save it, and send it
-- now or later. A step is a line that you type: a model turns it into a step that the game
-- can check, or it stays as you wrote it, and you check it yourself at the turn-in.

local _, ns = ...

local TaskForm = {}
ns.TaskForm = TaskForm

local LIMITS = ns.TaskWire.LIMITS
-- Saved tasks wait for a player, so a few are enough.
local MAX_SAVED = 20

local draft

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Changed()
	ns.JournalFrame.Refresh()
end

local function Fresh()
	return { title = "", text = "", money = 0, items = {}, steps = {}, stepText = "" }
end

local function CopySteps(steps)
	local copy = {}
	for n, step in ipairs(steps) do
		copy[n] = { kind = step.kind, target = step.target, count = step.count }
	end
	return copy
end

local function CopyItems(items)
	local copy = {}
	for n, item in ipairs(items) do
		copy[n] = { id = item.id, name = item.name, count = item.count }
	end
	return copy
end

-- The form edits a copy, so Cancel leaves the saved task as it was.
local function Loaded(saved)
	local form = Fresh()
	form.id, form.title, form.text, form.money = saved.id, saved.title, saved.text, saved.money
	form.items, form.steps = CopyItems(saved.items), CopySteps(saved.steps)
	return form
end

function TaskForm.Draft()
	draft = draft or Fresh()
	return draft
end

-- The form of a saved task, loaded when the player opens it, or a new form for nil.
function TaskForm.For(id)
	if not draft or draft.id ~= id then
		local saved = id and ns.TaskStore.Data().drafts[id]
		draft = saved and Loaded(saved) or Fresh()
	end
	return draft
end

-- A `|` starts a WoW escape, and the wire refuses one, so a typed text loses it here.
function TaskForm.Clean(text)
	return (tostring(text):gsub("[%c|]", " "):gsub("^%s+", ""):gsub("%s+$", ""))
end

-- Opens an empty form, and asks who can get a task.
function TaskForm.Open()
	draft = Fresh()
	ns.PlayerTasks.Call()
	ns.JournalFrame.Select("give")
end

function TaskForm.OpenSaved(id)
	TaskForm.For(id)
	ns.PlayerTasks.Call()
	ns.JournalFrame.Select("draft:" .. id)
end

-- Title and text -----------------------------------------------------------------------------

-- The box keeps what the player types. The limits of the wire are in bytes: "é" takes two.
function TaskForm.SetTitle(text)
	TaskForm.Draft().title = text
	Changed()
end

function TaskForm.EditText()
	ns.JournalFrame.Edit({
		title = "Description",
		hint = "",
		text = TaskForm.Draft().text,
		limit = LIMITS.text,
		bytes = LIMITS.text,
		problem = function(text)
			if #TaskForm.Clean(text) > LIMITS.text then
				return "Too long to save. Try a shorter version."
			end
		end,
		save = function(text)
			TaskForm.Draft().text = TaskForm.Clean(text)
		end,
	})
end

function TaskForm.TitleTooLong()
	return #TaskForm.Clean(TaskForm.Draft().title) > LIMITS.title
end

-- Steps --------------------------------------------------------------------------------------

function TaskForm.Full()
	return #TaskForm.Draft().steps >= ns.TaskWire.MAX_STEPS
end

-- The same step again raises its count, as "defeat 3 zombies". Returns false when full.
local function AddStep(step)
	local steps = TaskForm.Draft().steps
	for _, known in ipairs(steps) do
		if known.kind == step.kind and known.target == step.target then
			known.count = math.min(known.count + step.count, ns.TaskWire.MAX_COUNT)
			return true
		end
	end
	if TaskForm.Full() then
		return false
	end
	steps[#steps + 1] = step
	return true
end

function TaskForm.SetStepText(text)
	TaskForm.Draft().stepText = text
	Changed()
end

function TaskForm.StepTooLong()
	return #TaskForm.Clean(TaskForm.Draft().stepText) > LIMITS.target
end

-- The typed line goes to the model. The box empties at once, and the step shows when the
-- answer comes.
function TaskForm.AddStep()
	local form = TaskForm.Draft()
	local text = TaskForm.Clean(form.stepText)
	if text == "" or TaskForm.StepTooLong() or TaskForm.Full() or ns.TaskDraftHelp.Busy() then
		return
	end
	form.stepText = ""
	ns.TaskDraftHelp.CheckStep(text)
	Changed()
end

-- The steps that the model made of one typed line.
function TaskForm.AddChecked(steps)
	for _, step in ipairs(steps) do
		if not AddStep(step) then
			Say(string.format("A quest has at most %d steps.", ns.TaskWire.MAX_STEPS))
			break
		end
	end
	Changed()
end

-- A line that the model could not turn into a step, or that never reached it.
function TaskForm.AddWritten(text)
	AddStep({ kind = "other", target = text, count = 1 })
	Changed()
end

function TaskForm.RemoveStep(index)
	table.remove(TaskForm.Draft().steps, index)
	Changed()
end

-- Reward -------------------------------------------------------------------------------------

function TaskForm.SetMoney(gold, silver, copper)
	local form = TaskForm.Draft()
	local money = ns.TaskReward.Copper(gold, silver, copper)
	if ns.TaskReward.Fits(money, form.items) then
		form.money = money
	end
	Changed()
end

local function Readable(...)
	for n = 1, select("#", ...) do
		local value = select(n, ...)
		if value == nil or issecretvalue(value) then
			return false
		end
	end
	return true
end

-- The count of the stack on the cursor, as the trade window takes the whole stack.
local function CursorCount()
	local location = C_Cursor.GetCursorItem()
	local count = location and C_Item.GetStackCount(location)
	if type(count) == "number" and Readable(count) and count >= 1 then
		return math.min(count, ns.TaskWire.MAX_COUNT)
	end
	return 1
end

-- The item on the cursor: { id, name, count }, or nil. The name comes from its link.
local function CursorItem()
	local kind, id, link = GetCursorInfo()
	if not Readable(kind, id, link) or kind ~= "item" or type(link) ~= "string" then
		return nil
	end
	local name = link:match("%[(.-)%]")
	if not name or name == "" or #name > LIMITS.target or not ns.TaskWire.IsCleanText(name) then
		return nil
	end
	return { id = id, name = name, count = CursorCount() }
end

-- An item dropped on a slot of the reward. The item stays in your bags.
function TaskForm.DropItem()
	local item = CursorItem()
	if not item then
		return
	end
	ClearCursor()
	local form = TaskForm.Draft()
	if not ns.TaskReward.AddItem(form.money, form.items, item) then
		Say("That's all the reward a quest can hold.")
	end
	Changed()
end

function TaskForm.RemoveItem(index)
	table.remove(TaskForm.Draft().items, index)
	Changed()
end

-- Who gets it --------------------------------------------------------------------------------

function TaskForm.Pick(name)
	TaskForm.Draft().doer = name
	Changed()
end

-- Save, send, and cancel ---------------------------------------------------------------------

-- The first thing that stops a save, or nil.
function TaskForm.CantSave()
	local form = TaskForm.Draft()
	if TaskForm.Clean(form.title) == "" then
		return "Add a title."
	end
	if TaskForm.TitleTooLong() then
		return "The title is too long."
	end
end

-- The first thing that stops a send, or nil.
function TaskForm.CantSend()
	local problem = TaskForm.CantSave()
	if problem then
		return problem
	end
	local form = TaskForm.Draft()
	if #form.steps == 0 then
		return "Add a step."
	end
	if not form.doer then
		return "Pick who gets it."
	end
end

local function Stored(form)
	return {
		id = form.id,
		title = TaskForm.Clean(form.title),
		text = form.text,
		money = form.money,
		items = CopyItems(form.items),
		steps = CopySteps(form.steps),
		savedAt = time(),
	}
end

local function CountSaved(drafts)
	local count = 0
	for _ in pairs(drafts) do
		count = count + 1
	end
	return count
end

function TaskForm.Save()
	local form = TaskForm.Draft()
	if TaskForm.CantSave() then
		return
	end
	local drafts = ns.TaskStore.Data().drafts
	if not form.id and CountSaved(drafts) >= MAX_SAVED then
		Say(string.format("You have %d saved quests. Delete one to save another.", MAX_SAVED))
		return
	end
	form.id = form.id or ns.TaskStore.NewId(time())
	drafts[form.id] = Stored(form)
	ns.JournalFrame.Select("draft:" .. form.id)
end

function TaskForm.Send()
	local form = TaskForm.Draft()
	if TaskForm.CantSend() then
		return
	end
	local task = Stored(form)
	task.reward = ns.TaskReward.Text(form.money, form.items)
	local id = ns.PlayerTasks.Give(task, form.doer)
	if not id then
		return
	end
	if form.id then
		ns.TaskStore.Data().drafts[form.id] = nil
	end
	draft = nil
	ns.JournalFrame.Select("gave:" .. id)
end

function TaskForm.Cancel()
	draft = nil
	ns.JournalFrame.Select(nil)
end

function TaskForm.Delete()
	local form = TaskForm.Draft()
	if form.id then
		ns.TaskStore.Data().drafts[form.id] = nil
	end
	TaskForm.Cancel()
end

-- The saved tasks, newest first: { key, task }.
function TaskForm.Saved()
	return ns.TaskStore.List(ns.TaskStore.Data().drafts)
end
