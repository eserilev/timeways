-- The side quests on the Quests page of the journal (GAMEPLAY.md 3.4): the list, the lines
-- of the open quest with its steps and rewards, and its buttons.

local _, ns = ...

local JournalQuests = {}
ns.JournalQuests = JournalQuests

local Entries = ns.JournalRows.Entries
local Name = ns.JournalRows.Name
local Day = ns.JournalRows.Day
local Line = ns.JournalRows.Line
local Group = ns.JournalRows.Group
local Item = ns.JournalRows.Item
local Button = ns.JournalRows.Button
local OpenIndex = ns.JournalRows.OpenIndex
local Keys = ns.JournalRows.Keys
local SAVING = ns.JournalRows.SAVING

local DAY_SECONDS, HOUR_SECONDS = 86400, 3600

local function Days(days)
	return days == 1 and "1 day" or string.format("%d days", days)
end

-- A person counts in days and hours, never in seconds.
local function TimeLeft(seconds)
	if seconds >= DAY_SECONDS then
		return Days(math.floor(seconds / DAY_SECONDS)) .. " left"
	end
	local hours = math.floor(seconds / HOUR_SECONDS)
	if hours == 1 then
		return "1 hour left"
	end
	if hours > 1 then
		return string.format("%d hours left", hours)
	end
	return "less than an hour left"
end

-- The desktop records the end of a wait with its next line, so the page shows it at once.
local function WaitIsOver(step)
	return step.goal == "wait" and step.state == "open" and type(step.ready_at) == "number" and time() >= step.ready_at
end

local function WaitText(step)
	local text = "Wait " .. Days(math.floor(step.days))
	if step.state == "open" and type(step.ready_at) == "number" and not WaitIsOver(step) then
		return text .. ": " .. TimeLeft(step.ready_at - time()) .. "."
	end
	return text .. "."
end

-- The hours of each time of day, as the clock of the game shows them in English.
local TIMES = {
	dawn = "at dawn (5 AM to 8 AM)",
	noon = "at noon (11 AM to 2 PM)",
	dusk = "at dusk (6 PM to 9 PM)",
	night = "at night (9 PM to 5 AM)",
}

-- The command that the player types, as WoW writes an emote objective.
local function EmoteText(step)
	local command = "Use /" .. ns.Plain(step.emote)
	if type(step.npc) == "string" then
		return command .. " on " .. Name(step.npc) .. "."
	end
	return command .. " in " .. Name(step.place) .. "."
end

-- The count in your bags now, as the game shows a collect objective. A done step shows
-- its whole count: you had the items when you met the NPC.
local function CarryText(step)
	local held = step.state == "done" and step.count or (ns.Carry.InBags(Name(step.item)) or 0)
	local text = string.format("Bring %s to %s", Name(step.item), Name(step.npc))
	return string.format("%s: %d/%d", text, math.min(held, step.count), step.count)
end

-- The kills so far of a kill step, as the game shows a quest objective.
local function StepKills(step)
	return type(step.kills) == "number" and step.kills or 0
end

local STEP_TEXTS = {
	visit = function(step)
		return "Visit " .. Name(step.place) .. "."
	end,
	visit_at = function(step)
		if TIMES[step.time] then
			return "Visit " .. Name(step.place) .. " " .. TIMES[step.time] .. "."
		end
	end,
	meet = function(step)
		return "Speak with " .. Name(step.npc) .. "."
	end,
	talk = function(step)
		if type(step.about) == "string" then
			return "Ask " .. Name(step.npc) .. " about " .. ns.Plain(step.about) .. " (/talk)."
		end
		return "Talk to " .. Name(step.npc) .. " (/talk)."
	end,
	emote = function(step)
		if type(step.emote) == "string" then
			return EmoteText(step)
		end
	end,
	level = function(step)
		if type(step.level) == "number" then
			return string.format("Reach level %d.", step.level)
		end
	end,
	enter = function(step)
		return "Enter " .. Name(step.dungeon) .. "."
	end,
	defeat = function(step)
		return string.format("%s slain: %d/1", Name(step.boss), step.state == "done" and 1 or 0)
	end,
	game_quest = function(step)
		return 'Complete "' .. Name(step.title) .. '".'
	end,
	slap = function(step)
		return "Use /slap on " .. Name(step.npc) .. "."
	end,
	carry = function(step)
		if type(step.count) == "number" then
			return CarryText(step)
		end
	end,
	wait = function(step)
		if type(step.days) == "number" then
			return WaitText(step)
		end
	end,
	kill = function(step)
		if type(step.count) == "number" then
			return string.format("%s slain: %d/%d", Name(step.creature), StepKills(step), step.count)
		end
	end,
}

-- A goal that the addon does not know, or a step that misses a field, shows "?".
local function StepText(step)
	local text = STEP_TEXTS[step.goal]
	return text and text(step) or "?"
end

-- A number from the desktop that is no whole number goes out as no number: the newest offer.
local function QuestNumber(quest)
	local number = quest.number
	return type(number) == "number" and number >= 1 and number % 1 == 0 and number or nil
end

local function QuestStatus(quest, saving)
	if saving then
		return Line("hint", SAVING)
	end
	if quest.status == "offered" then
		return Line("note", "New offer.")
	end
	if quest.status == "done" then
		return Line("note", "Done on " .. Day(quest.done_at) .. ".")
	end
	return Line("note", "In progress.")
end

local function StepsDone(quest)
	local done = 0
	for _, step in ipairs(Entries(quest.steps)) do
		done = done + (step.state == "done" and 1 or 0)
	end
	return done
end

-- In a quest in progress, a step that waits for another step shows faded. An offer shows
-- its steps plainly, as the game shows the goals of a quest that you read.
local function StepLine(quest, step)
	if step.state == "done" or WaitIsOver(step) then
		return Line("entry", StepText(step) .. " (Complete)")
	end
	local waits = quest.status == "accepted" and step.state == "later"
	return Line(waits and "later" or "entry", StepText(step))
end

-- A mystery keeps its later steps from the book. Only their count comes.
local function HiddenSteps(quest)
	return type(quest.hidden_steps) == "number" and quest.hidden_steps > 0
end

-- The place in the list of the first step of the any-order set, or nil. The desktop counts
-- from 0.
local function AnyOrderStart(quest)
	local span = quest.any_order
	if type(span) == "table" and type(span.first) == "number" then
		return span.first + 1
	end
end

-- The NPC of the slap step. A step that the book does not show yet names no one.
local function SlappedName(quest)
	for _, step in ipairs(Entries(quest.steps)) do
		if step.goal == "slap" and type(step.npc) == "string" then
			return Name(step.npc)
		end
	end
	return "Someone"
end

-- The lines of the steps, with the mark of an any-order set and of hidden steps. The talk
-- window shows them too.
function JournalQuests.StepLines(quest)
	local lines = {}
	local setStart = AnyOrderStart(quest)
	for n, step in ipairs(Entries(quest.steps)) do
		if n == setStart then
			lines[#lines + 1] = Line("text", "In any order:")
		end
		lines[#lines + 1] = StepLine(quest, step)
	end
	if HiddenSteps(quest) then
		lines[#lines + 1] = Line("later", "More to come.")
	end
	return lines
end

-- The rewards are story, never loot (3.4): the giver trusts you more, and the deed goes into
-- your chronicle.
local function QuestLines(quest, saving)
	local lines = { Line("heading", Name(quest.title)), QuestStatus(quest, saving) }
	lines[#lines + 1] = Line("text", "From " .. Name(quest.giver) .. ", on " .. Day(quest.offered_at) .. ".")
	if type(quest.text) == "string" then
		lines[#lines + 1] = Line("prose", ns.Plain(quest.text))
	end
	for _, line in ipairs(JournalQuests.StepLines(quest)) do
		lines[#lines + 1] = line
	end
	lines[#lines + 1] = Line("section", "Rewards")
	lines[#lines + 1] = Line("text", Name(quest.giver) .. " trusts you more.")
	if quest.has_slap == true then
		lines[#lines + 1] = Line("text", SlappedName(quest) .. " will like you less.")
	end
	lines[#lines + 1] = Line("text", "An entry in your chronicle.")
	return lines
end

-- An offer has its buttons here too, because the chat line of the offer scrolls away.
local function QuestButtons(quest, saving)
	local number = QuestNumber(quest)
	if saving then
		return {}
	end
	if quest.status == "offered" then
		return {
			Button("Accept", function()
				ns.Quest.Accept(number)
			end),
			Button("Decline", function()
				ns.Quest.Decline(number)
			end),
		}
	end
	if quest.status == "done" or not number then
		return {}
	end
	return {
		Button("Abandon", function()
			ns.Quest.Abandon(number, quest.title)
		end),
	}
end

-- The groups of the list, in order. A status that the addon does not know shows as in progress.
local QUEST_GROUPS = { "offered", "accepted", "done" }
local QUEST_GROUP_TITLES = { offered = "Offered", accepted = "In progress", done = "Done" }

local function QuestGroup(quest)
	if quest.status == "offered" or quest.status == "done" then
		return quest.status
	end
	return "accepted"
end

local function QuestMark(quest, saving)
	if saving then
		return SAVING
	end
	if quest.status == "offered" then
		return "New"
	end
	if quest.status == "done" then
		return "Done"
	end
	if HiddenSteps(quest) then
		return string.format("%d done", StepsDone(quest))
	end
	return string.format("%d of %d", StepsDone(quest), #Entries(quest.steps))
end

-- A declined or abandoned task leaves the page at once. An accepted one shows as saving
-- until the journal confirms it.
local function OpenTasks(quests)
	local open = {}
	for n, quest in ipairs(quests) do
		local answer = ns.Quest.Answered(QuestNumber(quest))
		if answer ~= "declined" and answer ~= "abandoned" then
			local key = QuestNumber(quest) or ("#" .. n)
			open[#open + 1] = { quest = quest, key = key, saving = answer == "accepted" }
		end
	end
	return open
end

local function Grouped(tasks)
	local grouped = {}
	for _, group in ipairs(QUEST_GROUPS) do
		for _, task in ipairs(tasks) do
			if QuestGroup(task.quest) == group then
				grouped[#grouped + 1] = task
			end
		end
	end
	return grouped
end

-- `tasks` come in the order of their groups, so each group title comes once.
local function TaskList(tasks)
	local rows, group = {}, nil
	for _, task in ipairs(tasks) do
		if QuestGroup(task.quest) ~= group then
			group = QuestGroup(task.quest)
			rows[#rows + 1] = Group(QUEST_GROUP_TITLES[group])
		end
		local quest = task.quest
		rows[#rows + 1] = Item(task.key, Name(quest.title), Name(quest.giver), QuestMark(quest, task.saving))
	end
	return rows
end

-- The zone of a place: a subzone lies within its zone, and a zone is its own.
local function ZoneOf(journal, place)
	for _, known in ipairs(Entries(journal.places)) do
		if known.name == place and type(known.within) == "string" then
			return known.within
		end
	end
	return place
end

-- A task belongs to the zone of the person who gave it.
local function GiverZone(journal, giver)
	for _, person in ipairs(Entries(journal.people)) do
		if person.name == giver and type(person.place) == "string" then
			return ZoneOf(journal, person.place)
		end
	end
	return nil
end

-- The side quests, then the quests from players, which have their own rows and pages (4.7).
function JournalQuests.Page(journal)
	local tasks = Grouped(OpenTasks(Entries(journal.quests)))
	local page = { list = TaskList(tasks), lines = {}, buttons = {}, side = "map" }
	for _, row in ipairs(ns.TaskPages.Rows()) do
		page.list[#page.list + 1] = row
	end
	local selected = ns.Journal.Selected("quests")
	if ns.TaskPages.Owns(selected) then
		return ns.TaskPages.Fill(page, selected)
	end
	local task = tasks[OpenIndex(Keys(tasks), selected, 1)]
	if not task then
		local first = ns.TaskPages.FirstKey()
		return first and ns.TaskPages.Fill(page, first) or page
	end
	page.selected = task.key
	page.lines = QuestLines(task.quest, task.saving)
	page.buttons = QuestButtons(task.quest, task.saving)
	page.zone = GiverZone(journal, task.quest.giver)
	page.pins = ns.TaskPins.For(journal, task.quest)
	page.map = ns.TaskPins.MapOf(page.pins)
	return page
end

-- A quest that a page of Knowledge names opens here.
ns.JournalLinks.OPENERS.quest = function(number)
	ns.JournalFrame.Open("quests")
	ns.JournalFrame.Select(number)
end
