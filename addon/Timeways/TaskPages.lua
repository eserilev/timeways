-- The pages of player tasks in the Tasks section of the journal (GAMEPLAY.md 4.7): the
-- form to give a task, a task that you got, and a task that you gave with its turn-in.
-- The rows and lines have the shapes of Journal.Render.

local _, ns = ...

local TaskPages = {}
ns.TaskPages = TaskPages

local GIVE = "give"
local GOT_PREFIX, GAVE_PREFIX = "got:", "gave:"

local function Line(style, text, action)
	return { style = style, text = text, action = action }
end

local function Button(label, run, disabled)
	return { label = label, run = run, disabled = disabled }
end

local function Short(name)
	return ns.TaskPeople.Short(name)
end

local function Day(at)
	return date("%d %b %Y", at)
end

local function Clock(at)
	return date("%H:%M", at)
end

-- Steps ------------------------------------------------------------------------------------

local function Times(step)
	return step.count > 1 and string.format(" (%d times)", step.count) or ""
end

local STEP_TEXTS = {
	place = function(step)
		return "Go to " .. step.target .. "."
	end,
	npc = function(step)
		return "Talk to " .. step.target .. "."
	end,
	kill = function(step)
		return "Defeat " .. step.target .. Times(step) .. "."
	end,
	meet = function(step)
		return "Find " .. Short(step.target) .. "."
	end,
	item = function(step, giver)
		local count = step.count > 1 and (step.count .. " ") or ""
		return "Bring " .. count .. step.target .. " to " .. giver .. "."
	end,
}

function TaskPages.StepText(step, giver)
	return STEP_TEXTS[step.kind](step, giver)
end

-- The list ---------------------------------------------------------------------------------

local function Group(text)
	return { style = "group", text = text }
end

local function Item(key, text, detail, mark)
	return { style = "item", key = key, text = text, detail = detail, mark = mark }
end

local function CountClaims(task)
	local count = 0
	for index = 1, #task.steps do
		if task.claims[index] then
			count = count + 1
		end
	end
	return count
end

local function StepsMark(task)
	return string.format("%d of %d", CountClaims(task), #task.steps)
end

local GOT_MARKS = { offered = "New", done = "Done" }

local function GotMark(task)
	if task.turnInAt then
		return "Turn in"
	end
	return GOT_MARKS[task.status] or StepsMark(task)
end

local GAVE_MARKS = { offered = "Waiting", declined = "Declined", cancelled = "Canceled", done = "Done" }

local function GaveMark(task)
	if task.status == "accepted" and task.turnInAt then
		return "Turn in"
	end
	return GAVE_MARKS[task.status] or StepsMark(task)
end

function TaskPages.RewardState(task)
	if task.reward == "" then
		return "No reward promised"
	end
	if ns.TaskProof.Paid(task, ns.TaskStore.Data()) then
		return "Reward: paid in trade"
	end
	if task.status == "done" then
		return "Reward: not paid"
	end
	return "Reward: promised"
end

-- A task that you declined, or that the giver canceled, leaves your list.
local function GotRows(rows)
	local shown = {}
	for _, entry in ipairs(ns.PlayerTasks.Received()) do
		local task = entry.task
		if task.status ~= "declined" and task.status ~= "cancelled" then
			shown[#shown + 1] = Item(GOT_PREFIX .. entry.key, task.title, "From " .. Short(task.giver), GotMark(task))
		end
	end
	if #shown > 0 then
		rows[#rows + 1] = Group("From players")
	end
	for _, row in ipairs(shown) do
		rows[#rows + 1] = row
	end
end

local function GaveRows(rows)
	rows[#rows + 1] = Group("Tasks I gave")
	rows[#rows + 1] = Item(GIVE, "Give a task", "Write one for a friend")
	for _, entry in ipairs(ns.PlayerTasks.Given()) do
		local task = entry.task
		local detail = "To " .. Short(task.doer) .. ". " .. TaskPages.RewardState(task)
		rows[#rows + 1] = Item(GAVE_PREFIX .. entry.key, task.title, detail, GaveMark(task))
	end
end

function TaskPages.Rows()
	local rows = {}
	GotRows(rows)
	GaveRows(rows)
	return rows
end

-- The key of the first task that you got, for a book with no other task to open.
function TaskPages.FirstKey()
	for _, row in ipairs(TaskPages.Rows()) do
		if row.style == "item" and row.key ~= GIVE then
			return row.key
		end
	end
end

-- A task that you got ----------------------------------------------------------------------

local RELATIONS = { party = "your party", guild = "your guild", friend = "your friend" }

local function From(task)
	local relation = RELATIONS[ns.TaskPeople.Relation(task.giver)]
	local from = "From " .. Short(task.giver)
	return relation and (from .. ", " .. relation .. ".") or (from .. ".")
end

local function GotStatus(task)
	if task.status == "offered" then
		return "New task."
	end
	if task.status == "done" then
		return "Done on " .. Day(task.closedAt) .. "."
	end
	if task.turnInAt then
		return "Waiting for " .. Short(task.giver) .. " to check it. Stand next to them."
	end
	return "In progress."
end

local function GotProgress(task, index)
	local step = task.steps[index]
	if task.claims[index] then
		return "(done) "
	end
	local progress = (task.progress or {})[index]
	if step.count > 1 and progress then
		return string.format("(%d of %d) ", progress, step.count)
	end
	return ""
end

-- The doer's own words, never a model's: the Chronicle of the desktop never gets it (5.11).
function TaskPages.ChronicleLine(task)
	local line = string.format("%s finished %s for %s.", Short(task.doer), task.title, Short(task.giver))
	if task.place and task.place ~= "" then
		line = line .. " They met face to face in " .. task.place .. " to turn it in."
	end
	return line
end

local function GotLines(task)
	local giver = Short(task.giver)
	local lines = { Line("heading", task.title), Line("note", GotStatus(task)), Line("text", From(task)) }
	lines[#lines + 1] = Line("prose", task.text)
	for index, step in ipairs(task.steps) do
		lines[#lines + 1] = Line("entry", GotProgress(task, index) .. TaskPages.StepText(step, giver))
	end
	local done = task.status == "done" and "(done) " or ""
	lines[#lines + 1] = Line("entry", done .. "Turn in to " .. giver .. ", face to face.")
	if task.status == "done" then
		lines[#lines + 1] = Line("section", "For your Chronicle")
		lines[#lines + 1] = Line("prose", TaskPages.ChronicleLine(task))
	end
	lines[#lines + 1] = Line("section", "Rewards")
	lines[#lines + 1] = Line("text", "This goes into your Chronicle, with " .. giver .. "'s name.")
	if task.reward ~= "" then
		lines[#lines + 1] = Line("text", task.reward .. ". Promised by " .. giver .. ", paid by trade.")
	end
	return lines
end

local function GotButtons(key, task)
	local actions = ns.PlayerTasks
	if task.status == "offered" then
		return {
			Button("Block player", function()
				actions.Block(key)
			end),
			Button("Decline", function()
				actions.Decline(key)
			end),
			Button("Accept", function()
				actions.Accept(key)
			end),
		}
	end
	if task.status ~= "accepted" then
		return {}
	end
	local ready = CountClaims(task) == #task.steps and not task.turnInAt
	return {
		Button("Give up", function()
			actions.GiveUp(key)
		end),
		Button("Turn in", function()
			actions.AskTurnIn(key)
		end, not ready),
	}
end

-- A task that you gave, and its turn-in ----------------------------------------------------

local PROOF_TEXTS = {
	witnessed = "Witnessed: your addon saw it too.",
	seen = "Seen: only their addon recorded it.",
	unconfirmed = "Not confirmed: you were in the same zone and party, and your addon saw nothing.",
	missing = "Not done yet.",
}

local function ProofLine(step, task)
	local claim = task.claims[step.index]
	local level = ns.TaskProof.Level(step.step, claim, task, ns.TaskStore.Data())
	local text = PROOF_TEXTS[level]
	if level == "unconfirmed" and step.step.kind == "item" then
		text = "Not confirmed: your addon saw no trade with these items."
	end
	if claim then
		text = text .. " " .. Clock(claim.at) .. "."
	end
	return Line("hint", text)
end

local GAVE_STATUS = {
	offered = "Waiting for an answer.",
	accepted = "Accepted.",
	declined = "Declined.",
	cancelled = "You canceled this task.",
	done = "Done.",
}

local function TurnInLines(task)
	local doer = Short(task.doer)
	if not ns.TaskPeople.IsNear(task.doer) then
		return {
			Line("entry", "Turn in to you, face to face."),
			Line("hint", doer .. " isn't next to you. Target them and stand close."),
		}
	end
	return { Line("entry", "Turn in to you, face to face."), Line("hint", "Face to face now.") }
end

local function GaveLines(task)
	local doer = Short(task.doer)
	local lines = { Line("heading", task.title) }
	lines[#lines + 1] =
		Line("note", "Given to " .. doer .. " on " .. Day(task.sentAt) .. ". " .. GAVE_STATUS[task.status])
	if task.turnInAt and task.status == "accepted" then
		lines[#lines + 1] = Line("text", doer .. " wants to turn this in.")
	end
	lines[#lines + 1] = Line("section", "What they did")
	for index, step in ipairs(task.steps) do
		lines[#lines + 1] = Line("entry", TaskPages.StepText(step, "you"))
		lines[#lines + 1] = ProofLine({ index = index, step = step }, task)
	end
	for _, line in ipairs(TurnInLines(task)) do
		lines[#lines + 1] = line
	end
	lines[#lines + 1] = Line("help", "Timeways checks what it can. It can't catch everything.")
	lines[#lines + 1] = Line("section", "Reward")
	local promise = task.reward ~= "" and (" " .. task.reward .. ".") or ""
	lines[#lines + 1] = Line("text", TaskPages.RewardState(task) .. "." .. promise)
	if task.reward ~= "" and task.status == "accepted" then
		lines[#lines + 1] = Line("help", "Meet " .. doer .. " and stand next to each other. Pay in the trade window.")
	end
	return lines
end

local function GaveButtons(id, task)
	if task.status ~= "accepted" and task.status ~= "offered" then
		return {}
	end
	local buttons = {
		Button("Cancel task", function()
			ns.PlayerTasks.Cancel(id)
		end),
	}
	if task.status == "accepted" and task.turnInAt then
		buttons = {
			Button("Not yet", function()
				ns.PlayerTasks.NotYet(id)
			end),
			Button("Complete task", function()
				ns.PlayerTasks.Complete(id)
			end, not ns.TaskPeople.IsNear(task.doer)),
		}
	end
	return buttons
end

-- The form to give a task ------------------------------------------------------------------

local function Edit(field)
	return {
		label = "Edit",
		run = function()
			ns.TaskForm.Edit(field)
		end,
	}
end

local function FieldLines(lines, heading, field, empty, style)
	local form = ns.TaskForm.Draft()
	lines[#lines + 1] = Line("section", heading, Edit(field))
	if form[field] == "" then
		lines[#lines + 1] = Line("hint", empty)
	else
		lines[#lines + 1] = Line(style, form[field])
	end
end

local function StepLines(lines)
	lines[#lines + 1] = Line("section", "Steps")
	lines[#lines + 1] = Line("help", "Only steps the game can check. Add the same foe again for more kills.")
	for index, step in ipairs(ns.TaskForm.Draft().steps) do
		local remove = {
			label = "Remove",
			run = function()
				ns.TaskForm.RemoveStep(index)
			end,
		}
		lines[#lines + 1] = Line("entry", TaskPages.StepText(step, "you"), remove)
	end
	lines[#lines + 1] = Line("entry", "Turn in to you, face to face. Always last.")
end

local function Add(run)
	return { label = "Add", run = run }
end

local function ChoiceLines(lines)
	lines[#lines + 1] = Line("section", "Add a step")
	local choices = ns.TaskForm.Choices()
	for _, choice in ipairs(choices) do
		lines[#lines + 1] = Line(
			"text",
			choice.label,
			Add(function()
				ns.TaskForm.Choose(choice)
			end)
		)
	end
	lines[#lines + 1] = Line("text", "Bring an item to you.", Add(ns.TaskForm.AddItem))
	if not UnitExists("target") then
		lines[#lines + 1] = Line("help", "Target someone to add a talk, defeat, or find step.")
	end
end

local function RecipientLines(lines)
	lines[#lines + 1] = Line("section", "Who gets it?")
	local recipients = ns.PlayerTasks.Recipients()
	local picked = ns.TaskForm.Draft().doer
	for _, recipient in ipairs(recipients) do
		local name = Short(recipient.name) .. " (" .. RELATIONS[recipient.relation] .. ")"
		local pick = {
			label = "Pick",
			run = function()
				ns.TaskForm.Pick(recipient.name)
			end,
		}
		if recipient.name == picked then
			lines[#lines + 1] = Line("entry", name .. ". Picked.")
		else
			lines[#lines + 1] = Line("text", name, pick)
		end
	end
	if #recipients == 0 then
		lines[#lines + 1] = Line("hint", "Nobody online in your party, guild, or friends has Timeways right now.")
	end
	lines[#lines + 1] = Line("help", "Party, guild, and friends who use Timeways.")
end

local function SuggestionLines(lines)
	local suggestion = ns.TaskDraftHelp.Suggestion()
	if not suggestion then
		return
	end
	lines[#lines + 1] = Line("section", "Suggestion", { label = "Use this", run = ns.TaskDraftHelp.Use })
	lines[#lines + 1] = Line("text", suggestion.title)
	lines[#lines + 1] = Line("prose", suggestion.text)
	for _, step in ipairs(suggestion.steps) do
		lines[#lines + 1] = Line("entry", TaskPages.StepText(step, "you"))
	end
	local keep = { label = "Keep mine", run = ns.TaskDraftHelp.Keep }
	lines[#lines + 1] = Line("help", "Nothing changes until you pick. You can change every word after.", keep)
end

local function FormLines()
	local lines = { Line("heading", "Give a task") }
	SuggestionLines(lines)
	FieldLines(lines, "Name your task", "title", "No name yet.", "text")
	FieldLines(lines, "What should they do?", "text", "Tell the story: who, what, and why.", "prose")
	FieldLines(lines, "You promise", "reward", "Nothing. That's fine too.", "text")
	lines[#lines + 1] = Line("text", "Story reward: the task goes into their Chronicle, with your name.")
	lines[#lines + 1] = Line("help", "Timeways can't hand over items. You trade the reward yourself when you meet.")
	StepLines(lines)
	ChoiceLines(lines)
	RecipientLines(lines)
	local missing = ns.TaskForm.Missing()
	if missing then
		lines[#lines + 1] = Line("hint", missing)
	end
	return lines
end

local function FormButtons()
	local buttons = {}
	if ns.TaskDraftHelp.Available() then
		buttons[#buttons + 1] = Button("Help me write this", ns.TaskDraftHelp.Open)
	end
	buttons[#buttons + 1] = Button("Cancel", ns.TaskForm.Cancel)
	buttons[#buttons + 1] = Button("Send", ns.TaskForm.Send, ns.TaskForm.Missing() ~= nil)
	return buttons
end

-- The page of a key -------------------------------------------------------------------------

function TaskPages.Owns(key)
	if key == GIVE then
		return true
	end
	return type(key) == "string" and (key:sub(1, #GOT_PREFIX) == GOT_PREFIX or key:sub(1, #GAVE_PREFIX) == GAVE_PREFIX)
end

local function Got(key)
	local task = ns.TaskStore.Data().received[key]
	return task and { lines = GotLines(task), buttons = GotButtons(key, task), crumb = task.title }
end

local function Gave(id)
	local task = ns.TaskStore.Data().given[id]
	return task and { lines = GaveLines(task), buttons = GaveButtons(id, task), crumb = task.title }
end

-- Fills `page` with the page of a key that this module owns. A key of a task that is gone
-- leaves the page empty.
function TaskPages.Fill(page, key)
	local filled
	if key == GIVE then
		filled = { lines = FormLines(), buttons = FormButtons(), crumb = "Give a task" }
	elseif key:sub(1, #GOT_PREFIX) == GOT_PREFIX then
		filled = Got(key:sub(#GOT_PREFIX + 1))
	else
		filled = Gave(key:sub(#GAVE_PREFIX + 1))
	end
	page.selected = key
	if filled then
		page.lines, page.buttons, page.crumb = filled.lines, filled.buttons, filled.crumb
	end
	return page
end
