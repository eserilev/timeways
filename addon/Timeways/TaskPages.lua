-- The pages of player tasks in the Tasks section of the journal (GAMEPLAY.md 4.7): the
-- form to give a task, a task that you got, and a task that you gave with its turn-in.
-- The rows and lines have the shapes of Journal.Render.

local _, ns = ...

local TaskPages = {}
ns.TaskPages = TaskPages

local GIVE, BLOCKED = "give", "blocked"
local GOT_PREFIX, GAVE_PREFIX, DRAFT_PREFIX = "got:", "gave:", "draft:"

local function Line(style, text, action)
	return { style = style, text = text, action = action }
end

local function Button(label, run)
	return { label = label, run = run }
end

-- A button that shows what comes next, and does nothing yet.
local function Disabled(label)
	return { label = label, run = function() end, disabled = true }
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

-- A text of a player as a sentence: "5 gold" becomes "5 gold.", and "5 gold!" stays.
local function Sentence(text)
	return text:find("[%.!?]$") and text or (text .. ".")
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
	other = function(step)
		return Sentence(step.target)
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
	if GOT_MARKS[task.status] then
		return GOT_MARKS[task.status]
	end
	return task.turnInAt and "Turn in" or StepsMark(task)
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
	if #ns.PlayerTasks.Blocked() > 0 then
		shown[#shown + 1] = Item(BLOCKED, "Blocked players", "They can't send you quests")
	end
	if #shown > 0 then
		rows[#rows + 1] = Group("From players")
	end
	for _, row in ipairs(shown) do
		rows[#rows + 1] = row
	end
end

local function GaveRows(rows)
	rows[#rows + 1] = Group("Quests you wrote")
	rows[#rows + 1] = Item(GIVE, "New quest")
	for _, entry in ipairs(ns.TaskForm.Saved()) do
		rows[#rows + 1] = Item(DRAFT_PREFIX .. entry.key, entry.task.title, "Not sent yet")
	end
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

-- The key of the first player task of the list, for a book with no other task to open.
function TaskPages.FirstKey()
	for _, row in ipairs(TaskPages.Rows()) do
		local task = row.style == "item"
			and (row.key:sub(1, #GOT_PREFIX) == GOT_PREFIX or row.key:sub(1, #GAVE_PREFIX) == GAVE_PREFIX)
		if task then
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
		return "New quest."
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

-- The doer's own words, never a model's. The line names two players, so it stays in the
-- addon, and the Chronicle of the desktop never gets it (5.11).
function TaskPages.ChronicleLine(task)
	local line = string.format("%s finished %s for %s.", Short(task.doer), task.title, Short(task.giver))
	if task.place and task.place ~= "" then
		line = line .. " They met face to face in " .. task.place .. " to turn it in."
	end
	return line
end

-- The game can't see an "other" step, so the doer says when it is done.
local function MarkDone(key, task, index)
	if task.status ~= "accepted" or task.steps[index].kind ~= "other" or task.claims[index] then
		return nil
	end
	return Button("Done", function()
		ns.PlayerTasks.MarkDone(key, index)
	end)
end

local function GotLines(key, task)
	local giver = Short(task.giver)
	local lines = { Line("heading", task.title), Line("note", GotStatus(task)), Line("text", From(task)) }
	if task.text ~= "" then
		lines[#lines + 1] = Line("prose", task.text)
	end
	for index, step in ipairs(task.steps) do
		local text = GotProgress(task, index) .. TaskPages.StepText(step, giver)
		lines[#lines + 1] = Line("entry", text, MarkDone(key, task, index))
	end
	local done = task.status == "done" and "(done) " or ""
	lines[#lines + 1] = Line("entry", done .. "Turn in to " .. giver .. ", face to face.")
	if task.status == "done" then
		lines[#lines + 1] = Line("section", "Your story")
		lines[#lines + 1] = Line("prose", TaskPages.ChronicleLine(task))
	end
	lines[#lines + 1] = Line("section", "Rewards")
	lines[#lines + 1] = Line("text", "A line about it in your journal, with " .. giver .. "'s name.")
	if task.reward ~= "" then
		lines[#lines + 1] = Line("text", Sentence(task.reward) .. " " .. giver .. " pays it in a trade.")
	end
	return lines
end

local function GotButtons(key, task)
	local actions = ns.PlayerTasks
	if task.status == "offered" then
		return {
			Button("Block player", function()
				StaticPopup_Show("TIMEWAYS_BLOCK_PLAYER", Short(task.giver), nil, { key = key })
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
	-- Asking again is harmless, and the answer to the first ask can get lost.
	local ready = CountClaims(task) == #task.steps
	local turnIn = ready and Button("Turn in", function()
		actions.AskTurnIn(key)
	end) or Disabled("Turn in")
	return {
		Button("Give up", function()
			actions.GiveUp(key)
		end),
		turnIn,
	}
end

StaticPopupDialogs.TIMEWAYS_BLOCK_PLAYER = {
	text = "Block %s? You won't get quests from them anymore.",
	button1 = "Block",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		ns.PlayerTasks.Block(data.key)
	end,
}

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
	elseif claim and step.step.kind == "other" then
		text = "They say it's done. You decide."
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
	cancelled = "You canceled this quest.",
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
	if task.status == "accepted" then
		for _, line in ipairs(TurnInLines(task)) do
			lines[#lines + 1] = line
		end
	end
	lines[#lines + 1] = Line("help", "Timeways checks what it can. It can't catch everything.")
	lines[#lines + 1] = Line("section", "Reward")
	local promise = task.reward ~= "" and (" " .. Sentence(task.reward)) or ""
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
		Button("Cancel quest", function()
			ns.PlayerTasks.Cancel(id)
		end),
	}
	if task.status == "accepted" and task.turnInAt then
		local complete = ns.TaskPeople.IsNear(task.doer)
				and Button("Complete quest", function()
					ns.PlayerTasks.Complete(id)
				end)
			or Disabled("Complete quest")
		buttons = {
			Button("Not yet", function()
				ns.PlayerTasks.NotYet(id)
			end),
			complete,
		}
	end
	return buttons
end

-- The form to write a task ------------------------------------------------------------------

local LIMITS = ns.TaskWire.LIMITS

local function TitleLines(lines, form)
	lines[#lines + 1] = {
		style = "field",
		text = "Title",
		field = { value = form.title, letters = LIMITS.title, change = ns.TaskForm.SetTitle },
	}
	if ns.TaskForm.TitleTooLong() then
		lines[#lines + 1] = Line("hint", "Too long. Try a shorter title.")
	end
	if form.text == "" then
		lines[#lines + 1] = Line("help", "Description (optional)", Button("Add", ns.TaskForm.EditText))
	else
		lines[#lines + 1] = Line("prose", form.text, Button("Edit", ns.TaskForm.EditText))
	end
end

local function Remove(index)
	return Button("Remove", function()
		ns.TaskForm.RemoveStep(index)
	end)
end

local function StepInputLines(lines, form)
	if ns.TaskForm.Full() then
		return
	end
	local busy = ns.TaskDraftHelp.Busy()
	lines[#lines + 1] = {
		style = "field",
		text = "Add a step, like: kill 5 bats",
		field = {
			value = form.stepText,
			letters = LIMITS.target,
			change = ns.TaskForm.SetStepText,
			submit = ns.TaskForm.AddStep,
		},
		action = busy and Disabled("Add") or Button("Add", ns.TaskForm.AddStep),
	}
	if ns.TaskForm.StepTooLong() then
		lines[#lines + 1] = Line("hint", "Too long. Try a shorter step.")
	end
end

local function StepLines(lines, form)
	lines[#lines + 1] = Line("section", "Steps")
	for index, step in ipairs(form.steps) do
		lines[#lines + 1] = Line("entry", TaskPages.StepText(step, "you"), Remove(index))
		if step.kind == "other" then
			lines[#lines + 1] = Line("hint", "You check this one at turn-in.")
		end
	end
	local checking = ns.TaskDraftHelp.Checking()
	if checking then
		lines[#lines + 1] = Line("entry", Sentence(checking))
		lines[#lines + 1] = Line("hint", "Checking...")
	end
	lines[#lines + 1] = Line("entry", "Turn in to you, face to face.")
	StepInputLines(lines, form)
end

local function RewardLines(lines, form)
	lines[#lines + 1] = Line("section", "Reward")
	local coins = { ns.TaskReward.Coins(form.money) }
	lines[#lines + 1] = { style = "money", money = { coins = coins, change = ns.TaskForm.SetMoney } }
	lines[#lines + 1] = {
		style = "slots",
		slots = {
			items = form.items,
			size = ns.TaskReward.MAX_ITEMS,
			drop = ns.TaskForm.DropItem,
			remove = ns.TaskForm.RemoveItem,
		},
	}
	lines[#lines + 1] = Line("help", "You trade it to them at turn-in.")
end

local function RecipientLines(lines, form)
	lines[#lines + 1] = Line("section", "Send to")
	local recipients = ns.PlayerTasks.Recipients()
	for _, recipient in ipairs(recipients) do
		local name = Short(recipient.name) .. " (" .. recipient.relation .. ")"
		if recipient.name == form.doer then
			lines[#lines + 1] = Line("entry", name)
		else
			lines[#lines + 1] = Line(
				"text",
				name,
				Button("Pick", function()
					ns.TaskForm.Pick(recipient.name)
				end)
			)
		end
	end
	if #recipients == 0 then
		lines[#lines + 1] = Line("help", "Party, guild, and friends with Timeways show up here.")
	end
end

local HELP_STATES = {
	asking = "Writing...",
	failed = "The desktop app didn't answer. Try again later.",
	empty = "Couldn't write that one. Try other words.",
}

local function SuggestionLines(lines)
	local state = HELP_STATES[ns.TaskDraftHelp.State()]
	if state then
		lines[#lines + 1] = Line("hint", state)
	end
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
	lines[#lines + 1] = Line("help", "Nothing changes until you pick.", keep)
end

local function FormLines(form)
	local lines = {}
	SuggestionLines(lines)
	TitleLines(lines, form)
	StepLines(lines, form)
	RewardLines(lines, form)
	RecipientLines(lines, form)
	return lines
end

local function FormButtons(form)
	local buttons = {}
	local busy = ns.TaskDraftHelp.Busy()
	buttons[#buttons + 1] = busy and Disabled("Help me write") or Button("Help me write", ns.TaskDraftHelp.Open)
	if form.id then
		buttons[#buttons + 1] = Button("Delete", function()
			StaticPopup_Show("TIMEWAYS_DELETE_TASK")
		end)
	end
	buttons[#buttons + 1] = Button("Cancel", ns.TaskForm.Cancel)
	buttons[#buttons + 1] = ns.TaskForm.CantSave() and Disabled("Save") or Button("Save", ns.TaskForm.Save)
	buttons[#buttons + 1] = ns.TaskForm.CantSend() and Disabled("Send") or Button("Send", ns.TaskForm.Send)
	return buttons
end

StaticPopupDialogs.TIMEWAYS_DELETE_TASK = {
	text = "Delete this quest?",
	button1 = "Delete",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function()
		ns.TaskForm.Delete()
	end,
}

local function Form(id)
	local form = ns.TaskForm.For(id)
	local crumb = id and form.title or "New quest"
	return { lines = FormLines(form), buttons = FormButtons(form), crumb = crumb, footer = ns.TaskForm.CantSend() or "" }
end

-- The players that you blocked ------------------------------------------------------------

local function BlockedLines()
	local lines = { Line("heading", "Blocked players") }
	for _, name in ipairs(ns.PlayerTasks.Blocked()) do
		local unblock = {
			label = "Unblock",
			run = function()
				ns.PlayerTasks.Unblock(name)
			end,
		}
		lines[#lines + 1] = Line("text", Short(name), unblock)
	end
	if #lines == 1 then
		lines[#lines + 1] = Line("hint", "Nobody is blocked.")
	end
	lines[#lines + 1] = Line("help", "A blocked player can't send you quests.")
	return lines
end

-- The page of a key -------------------------------------------------------------------------

local PREFIXES = { GOT_PREFIX, GAVE_PREFIX, DRAFT_PREFIX }

function TaskPages.Owns(key)
	if key == GIVE or key == BLOCKED then
		return true
	end
	if type(key) ~= "string" then
		return false
	end
	for _, prefix in ipairs(PREFIXES) do
		if key:sub(1, #prefix) == prefix then
			return true
		end
	end
	return false
end

local function Got(key)
	local task = ns.TaskStore.Data().received[key]
	return task and { lines = GotLines(key, task), buttons = GotButtons(key, task), crumb = task.title }
end

-- A saved task that is gone, such as one sent from another form, leaves the page empty.
local function Saved(id)
	return ns.TaskStore.Data().drafts[id] and Form(id)
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
		filled = Form(nil)
	elseif key:sub(1, #DRAFT_PREFIX) == DRAFT_PREFIX then
		filled = Saved(key:sub(#DRAFT_PREFIX + 1))
	elseif key == BLOCKED then
		filled = { lines = BlockedLines(), buttons = {}, crumb = "Blocked players" }
	elseif key:sub(1, #GOT_PREFIX) == GOT_PREFIX then
		filled = Got(key:sub(#GOT_PREFIX + 1))
	else
		filled = Gave(key:sub(#GAVE_PREFIX + 1))
	end
	page.selected = key
	if filled then
		page.lines, page.buttons, page.crumb = filled.lines, filled.buttons, filled.crumb
		page.footer = filled.footer
	end
	return page
end
