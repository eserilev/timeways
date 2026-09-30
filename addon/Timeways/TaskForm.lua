-- The task that you write for another player, before you send it (GAMEPLAY.md 4.7). Each
-- step comes from the game: where you stand, or the unit that you target. So each name in
-- a step is the name that the game uses, and the doer's addon can match it.

local _, ns = ...

local TaskForm = {}
ns.TaskForm = TaskForm

local LIMITS = ns.TaskWire.LIMITS

local draft

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Fresh()
	return { title = "", text = "", reward = "", steps = {}, doer = nil }
end

function TaskForm.Draft()
	draft = draft or Fresh()
	return draft
end

-- A `|` starts a WoW escape, and the wire refuses one, so a typed text loses it here.
function TaskForm.Clean(text)
	return (tostring(text):gsub("[%c|]", " "):gsub("^%s+", ""):gsub("%s+$", ""))
end

local function Changed()
	ns.JournalFrame.Refresh()
end

-- Opens the form, and asks who can get a task.
function TaskForm.Open()
	TaskForm.Draft()
	ns.PlayerTasks.Call()
	ns.JournalFrame.Select("give")
end

function TaskForm.IsOpen()
	return ns.JournalFrame.IsShown()
		and ns.JournalFrame.Section() == "quests"
		and ns.Journal.Selected("quests") == "give"
end

-- The "Add" lines follow your target.
function TaskForm.TargetChanged()
	if TaskForm.IsOpen() then
		Changed()
	end
end

local FIELDS = {
	title = { title = "Name your task", hint = "A short name, like a quest title.", limit = LIMITS.title },
	text = { title = "What should they do?", hint = "Tell the story: who, what, and why.", limit = LIMITS.text },
	reward = {
		title = "What do you promise?",
		hint = "Leave it empty for no reward. You hand it over yourself, in a trade.",
		limit = LIMITS.reward,
	},
}

function TaskForm.Edit(field)
	local spec = FIELDS[field]
	ns.JournalFrame.Edit({
		title = spec.title,
		hint = spec.hint,
		text = TaskForm.Draft()[field],
		limit = spec.limit,
		save = function(text)
			TaskForm.Draft()[field] = TaskForm.Clean(text):sub(1, spec.limit)
		end,
	})
end

local function Full()
	if #TaskForm.Draft().steps >= ns.TaskWire.MAX_STEPS then
		Say(string.format("A task has at most %d steps, and the turn-in.", ns.TaskWire.MAX_STEPS))
		return true
	end
	return false
end

-- The same step again raises its count, as "defeat 3 zombies".
local function AddStep(kind, target, count)
	for _, step in ipairs(TaskForm.Draft().steps) do
		if step.kind == kind and step.target == target then
			step.count = math.min(step.count + count, ns.TaskWire.MAX_COUNT)
			Changed()
			return
		end
	end
	if not Full() then
		table.insert(TaskForm.Draft().steps, { kind = kind, target = target, count = count })
		Changed()
	end
end

function TaskForm.RemoveStep(index)
	table.remove(TaskForm.Draft().steps, index)
	Changed()
end

-- The steps that the game offers now: { kind, target, label }.
function TaskForm.Choices()
	local choices = {}
	local place = GetSubZoneText()
	place = place ~= "" and place or GetRealZoneText()
	if place ~= "" and #place <= LIMITS.target then
		choices[#choices + 1] = { kind = "place", target = place, label = "Go to " .. place .. ", where you stand." }
	end
	local friendly = ns.Units.FriendlyNpcName("target")
	if friendly and #friendly <= LIMITS.target then
		choices[#choices + 1] = { kind = "npc", target = friendly, label = "Talk to " .. friendly .. ", your target." }
	end
	local foe = UnitCanAttack("player", "target") and UnitName("target")
	if foe and not issecretvalue(foe) and #foe <= LIMITS.target then
		choices[#choices + 1] = { kind = "kill", target = foe, label = "Defeat " .. foe .. ", your target." }
	end
	local player = ns.TaskPeople.OfUnit("target")
	if player and player ~= ns.TaskPeople.Me() then
		local label = "Find " .. ns.TaskPeople.Short(player) .. ", your target."
		choices[#choices + 1] = { kind = "meet", target = player, label = label }
	end
	return choices
end

function TaskForm.Choose(choice)
	AddStep(choice.kind, choice.target, 1)
end

-- "10 Linen Cloth", or "Linen Cloth" for one.
function TaskForm.ParseItem(text)
	text = TaskForm.Clean(text)
	local count, name = text:match("^(%d+)%s+(.+)$")
	count, name = tonumber(count) or 1, name or text
	if name == "" or #name > LIMITS.target or count < 1 or count > ns.TaskWire.MAX_COUNT then
		return nil
	end
	return name, count
end

function TaskForm.AddItem()
	ns.JournalFrame.Edit({
		title = "Bring an item",
		hint = "Which item, and how many? For example: 10 Linen Cloth. Spell it as the game does.",
		text = "",
		limit = LIMITS.target + 4,
		save = function(text)
			local name, count = TaskForm.ParseItem(text)
			if not name then
				Say("Write the number, then the item, like: 10 Linen Cloth.")
				return
			end
			AddStep("item", name, count)
		end,
	})
end

function TaskForm.Pick(name)
	TaskForm.Draft().doer = name
	Changed()
end

-- The reason the task can't go yet, or nil.
function TaskForm.Missing()
	local form = TaskForm.Draft()
	if form.title == "" then
		return "Name your task first."
	end
	if form.text == "" then
		return "Say what they should do."
	end
	if #form.steps == 0 then
		return "Add at least one step."
	end
	if not form.doer then
		return "Pick who gets it."
	end
end

function TaskForm.Send()
	local form = TaskForm.Draft()
	if TaskForm.Missing() then
		return
	end
	local id = ns.PlayerTasks.Give(form, form.doer)
	if id then
		draft = nil
		ns.JournalFrame.Select("gave:" .. id)
	end
end

function TaskForm.Cancel()
	draft = nil
	ns.JournalFrame.Select(nil)
end
