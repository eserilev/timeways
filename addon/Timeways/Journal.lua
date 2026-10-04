-- The pages of the journal: the hero, the chronicle, deeds, what you learned, and your
-- side quests (GAMEPLAY.md 3.1.1, 3.4, 3.6, and 3.7).
-- The desktop sends them, because the world lives there and never in the saved variables
-- (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

Journal.SECTIONS = { "hero", "chapters", "deeds", "learned", "quests" }
Journal.TITLES = {
	hero = "Hero",
	chapters = "Chronicle",
	deeds = "Deeds",
	learned = "Knowledge",
	quests = "Quests",
}

-- The lists that come in pages, with the sheet and the entries of the hero.
-- Places and people have no page: the map and the tooltips read them.
local LISTS = { "chapters", "places", "people", "deeds", "learned", "quests", "stories" }

-- A reply holds at most 24 KB, so a long journal comes in pages. More than this many
-- pages means a broken reply, not a long journal.
local MAX_PAGES = 64

-- The whole journal, once every page came. It stays while a newer one comes in.
local pages
-- The pages so far of the journal that comes in, and the page that comes next.
local collecting

-- Values from the desktop are checked here, so a broken journal shows gaps, not errors.
local function List(value)
	return type(value) == "table" and value or {}
end

local function Entries(value)
	local entries = {}
	for _, entry in ipairs(List(value)) do
		if type(entry) == "table" then
			entries[#entries + 1] = entry
		end
	end
	return entries
end

-- The page draws again while it shows the count of a carry step.
function Journal.BagsChanged()
	if ns.QuestSteps.Carries() then
		ns.JournalFrame.Refresh()
	end
end

function Journal.Request(page)
	ns.Outbox.Add(ns.Inputs.JournalAsked(page))
	ns.Outbox.Flush()
end

local function Started()
	local journal = { chapters = {}, places = {}, people = {}, deeds = {}, learned = {}, quests = {}, stories = {} }
	journal.next = 0
	journal.hero = { sheet = {}, entries = {} }
	return journal
end

local function Append(journal, value)
	for _, section in ipairs(LISTS) do
		for _, entry in ipairs(Entries(value[section])) do
			table.insert(journal[section], entry)
		end
	end
	local hero = type(value.hero) == "table" and value.hero or {}
	for _, list in ipairs({ "sheet", "entries" }) do
		for _, entry in ipairs(Entries(hero[list])) do
			table.insert(journal.hero[list], entry)
		end
	end
end

-- The journal of the first page alone, for the self-test.
function Journal.FirstPage(value)
	local journal = Started()
	Append(journal, value)
	return journal
end

-- A page out of order belongs to an older request, and is dropped.
function Journal.Receive(value)
	local page, count = value.page, value.pages
	-- No pages: the desktop failed to read the journal, and the book keeps what it shows.
	if type(page) ~= "number" or type(count) ~= "number" or count < 1 or count > MAX_PAGES then
		return
	end
	if page == 0 then
		collecting = Started()
		ns.Hero.ShowRefused(value.hero_refused)
	end
	if not collecting or page ~= collecting.next then
		return
	end
	Append(collecting, value)
	collecting.next = page + 1
	if collecting.next < count then
		Journal.Request(collecting.next)
		return
	end
	pages, collecting = collecting, nil
	ns.Hero.JournalCame()
	ns.MspProfile.JournalCame(pages.hero.sheet)
	ns.Quest.JournalCame()
	ns.Trust.Update(pages.people)
	ns.QuestSteps.Read(pages.quests)
	ns.JournalFrame.Refresh()
	ns.Hero.AskOnce(pages.hero)
end

local function Name(value)
	return type(value) == "string" and ns.Plain(value) or "?"
end

local function Day(at)
	return type(at) == "number" and date("%d %b %Y", at) or "an unknown day"
end

-- `action` is an optional button of the line: { label = ..., run = function }.
local function Line(style, text, action)
	return { style = style, text = text, action = action }
end

local function Where(deed)
	return deed.place and (Name(deed.place) .. ", ") or ""
end

-- The first kill is the true kill. Each later kill is an echo after a reset (5.13).
local function DeedTitle(deed)
	if deed.kind == "level" and type(deed.to) == "number" then
		local what = deed.from and "Reached level %d" or "Started at level %d"
		return string.format(what, deed.to)
	elseif deed.kind == "defeated" and type(deed.times) == "number" then
		if deed.times == 1 then
			return "Defeated " .. Name(deed.foe)
		end
		return string.format("Defeated %s again (%d times)", Name(deed.foe), deed.times)
	elseif deed.kind == "titled" then
		return "Earned the title " .. Name(deed.title)
	elseif deed.kind == "quest_done" then
		return "Finished the quest " .. Name(deed.title)
	elseif deed.kind == "game_quest_done" then
		return "Finished the quest " .. Name(deed.title)
	elseif deed.kind == "class_quest_done" then
		return "Finished the class quest " .. Name(deed.title)
	elseif deed.kind == "quest_marked" then
		return Name(deed.mark) .. ", from " .. Name(deed.quest)
	elseif deed.kind == "died" then
		return deed.killer and ("Killed by " .. Name(deed.killer)) or "Died"
	end
end

local function Deeds(deeds)
	local lines = {}
	for _, deed in ipairs(deeds) do
		local title = DeedTitle(deed)
		if title then
			lines[#lines + 1] = Line("entry", title)
			lines[#lines + 1] = Line("text", Where(deed) .. Day(deed.at) .. ".")
		end
	end
	return lines
end

-- "A", "A and B", or "A, B and C".
local function Together(names)
	local shown = {}
	for _, name in ipairs(names) do
		shown[#shown + 1] = Name(name)
	end
	if #shown <= 1 then
		return shown[1] or ""
	end
	return table.concat(shown, ", ", 1, #shown - 1) .. " and " .. shown[#shown]
end

-- The first zone of a chapter names it, as a chapter of a book has a title.
local function ChapterPlace(chapter)
	local zones = List(chapter.zones)
	return type(zones[1]) == "string" and ns.Plain(zones[1]) or nil
end

local function ChapterNumber(chapter)
	return type(chapter.number) == "number" and chapter.number or "?"
end

local function ChapterTitle(chapter)
	local place = ChapterPlace(chapter)
	local number = "Chapter " .. ChapterNumber(chapter)
	return place and (number .. ": " .. place) or number
end

-- A session can run past midnight, so a chapter can span days.
local function Dates(chapter)
	local began, ended = Day(chapter.began), Day(chapter.ended)
	if type(chapter.ended) ~= "number" or began == ended then
		return began
	end
	return began .. " to " .. ended
end

-- A chapter is what was new in one session (GAMEPLAY.md 3.3). Once the narrator wrote it,
-- its saga comes first.
local function ChapterLines(chapter)
	local lines = {}
	local place = ChapterPlace(chapter)
	if place then
		lines[#lines + 1] = Line("note", "Chapter " .. ChapterNumber(chapter))
	end
	lines[#lines + 1] = Line("heading", place or ("Chapter " .. ChapterNumber(chapter)))
	lines[#lines + 1] = Line("text", Dates(chapter) .. ".")
	if type(chapter.prose) == "string" then
		lines[#lines + 1] = Line("prose", ns.Plain(chapter.prose))
	end
	for _, footnote in ipairs(List(chapter.footnotes)) do
		if type(footnote) == "string" then
			lines[#lines + 1] = Line("note", "* " .. ns.Plain(footnote))
		end
	end
	if #List(chapter.zones) > 0 then
		lines[#lines + 1] = Line("section", "Places you visited")
		lines[#lines + 1] = Line("text", Together(List(chapter.zones)) .. ".")
	end
	if #List(chapter.people) > 0 then
		lines[#lines + 1] = Line("section", "People you met")
		lines[#lines + 1] = Line("text", Together(List(chapter.people)) .. ".")
	end
	local deeds = {}
	for _, deed in ipairs(Entries(chapter.deeds)) do
		local title = DeedTitle(deed)
		if title then
			deeds[#deeds + 1] = Line("entry", title .. ".")
		end
	end
	if #deeds > 0 then
		lines[#lines + 1] = Line("section", "What you did")
	end
	for _, line in ipairs(deeds) do
		lines[#lines + 1] = line
	end
	if type(chapter.left_out) == "number" and chapter.left_out > 0 then
		lines[#lines + 1] = Line("text", string.format("And %d more.", chapter.left_out))
	end
	return lines
end

-- The desktop keeps `$N` in place of the name of the character, so no model sees it (5.11).
local function WithName(text)
	local name = UnitName("player")
	local shown = type(name) == "string" and not issecretvalue(name) and name:gsub("%%", "%%%%") or "you"
	return (ns.Plain(text):gsub("%$N", shown))
end

local LEARNED_TITLES = {
	book = function(entry)
		return "Read " .. Name(entry.title)
	end,
	quest = function(entry)
		return "The quest " .. Name(entry.title)
	end,
	gossip = function(entry)
		return "Heard from " .. Name(entry.npc)
	end,
	rumor = function(entry)
		return "A rumor from " .. Name(entry.npc)
	end,
}

-- A rumor is the word of a model, never canon (3.1.1), and the page says so.
local function Learned(entries)
	local lines = {}
	for _, entry in ipairs(entries) do
		local title = LEARNED_TITLES[entry.kind]
		if title then
			lines[#lines + 1] = Line("entry", title(entry))
			if type(entry.excerpt) == "string" then
				lines[#lines + 1] = Line("prose", WithName(entry.excerpt))
			end
			local place = type(entry.place) == "string" and (Name(entry.place) .. ", ") or ""
			local rumor = entry.kind == "rumor" and "Only a rumor. " or ""
			lines[#lines + 1] = Line("text", rumor .. place .. Day(entry.at) .. ".")
		end
	end
	return lines
end

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

local function StepText(step)
	if step.goal == "visit" then
		return "Visit " .. Name(step.place) .. "."
	end
	if step.goal == "visit_at" and TIMES[step.time] then
		return "Visit " .. Name(step.place) .. " " .. TIMES[step.time] .. "."
	end
	if step.goal == "meet" then
		return "Speak with " .. Name(step.npc) .. "."
	end
	if step.goal == "talk" and type(step.about) == "string" then
		return "Ask " .. Name(step.npc) .. " about " .. ns.Plain(step.about) .. " (/talk)."
	end
	if step.goal == "talk" then
		return "Talk to " .. Name(step.npc) .. " (/talk)."
	end
	if step.goal == "emote" and type(step.emote) == "string" then
		return EmoteText(step)
	end
	if step.goal == "level" and type(step.level) == "number" then
		return string.format("Reach level %d.", step.level)
	end
	if step.goal == "enter" then
		return "Enter " .. Name(step.dungeon) .. "."
	end
	if step.goal == "defeat" then
		return string.format("%s slain: %d/1", Name(step.boss), step.state == "done" and 1 or 0)
	end
	if step.goal == "game_quest" then
		return 'Complete "' .. Name(step.title) .. '".'
	end
	if step.goal == "slap" then
		return "Use /slap on " .. Name(step.npc) .. "."
	end
	if step.goal == "carry" and type(step.count) == "number" then
		return CarryText(step)
	end
	if step.goal == "wait" and type(step.days) == "number" then
		return WaitText(step)
	end
	if step.goal == "kill" and type(step.count) == "number" then
		return string.format("%s slain: %d/%d", Name(step.creature), StepKills(step), step.count)
	end
	return "?"
end

-- The open item of each page with a list, by its key. A key that is gone opens the default.
local selected = {}

function Journal.Select(section, key)
	selected[section] = key
end

function Journal.Selected(section)
	return selected[section]
end

-- A row of the list on the left: a group title, a line of help, or an item that opens.
local function Group(text)
	return { style = "group", text = text }
end

local function Item(key, text, detail, mark)
	return { style = "item", key = key, text = text, detail = detail, mark = mark }
end

local function Button(label, run, disabled)
	return { label = label, run = run, disabled = disabled }
end

-- The place in `keys` of the selected key, or of the default when the selected key is gone.
local function OpenIndex(section, keys, default)
	for n, key in ipairs(keys) do
		if key == selected[section] then
			return n
		end
	end
	return default
end

-- The buttons that open the item before and after the open one.
local function Steps(keys, index, before, after)
	local function Open(n)
		return function()
			ns.JournalFrame.Select(keys[n])
		end
	end
	return {
		Button(before, Open(index - 1), index <= 1),
		Button(after, Open(index + 1), index >= #keys),
	}
end

-- A number from the desktop that is no whole number goes out as no number: the newest offer.
local function QuestNumber(quest)
	local number = quest.number
	return type(number) == "number" and number >= 1 and number % 1 == 0 and number or nil
end

local SAVING = "Saving..."

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

-- The rewards are story, never loot (3.4): the giver trusts you more, and the deed goes into
-- your chronicle.
local function QuestLines(quest, saving)
	local lines = { Line("heading", Name(quest.title)), QuestStatus(quest, saving) }
	lines[#lines + 1] = Line("text", "From " .. Name(quest.giver) .. ", on " .. Day(quest.offered_at) .. ".")
	if type(quest.text) == "string" then
		lines[#lines + 1] = Line("prose", ns.Plain(quest.text))
	end
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

local function Keys(items)
	local keys = {}
	for n, item in ipairs(items) do
		keys[n] = item.key
	end
	return keys
end

-- Tasks from players have their own rows and pages (4.7).
local function Tasks(journal)
	local tasks = Grouped(OpenTasks(Entries(journal.quests)))
	local page = { list = TaskList(tasks), lines = {}, buttons = {}, side = "map" }
	for _, row in ipairs(ns.TaskPages.Rows()) do
		page.list[#page.list + 1] = row
	end
	if ns.TaskPages.Owns(selected.quests) then
		return ns.TaskPages.Fill(page, selected.quests)
	end
	local task = tasks[OpenIndex("quests", Keys(tasks), 1)]
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

-- The book opens on the newest chapter, the one still being written.
local function Chronicle(journal)
	local chapters = Entries(journal.chapters)
	local page = { list = {}, lines = {}, buttons = {}, side = "map" }
	for n, chapter in ipairs(chapters) do
		local key = type(chapter.number) == "number" and chapter.number or ("#" .. n)
		page.list[n] = Item(key, ChapterTitle(chapter), Dates(chapter))
	end
	local keys = Keys(page.list)
	local index = OpenIndex("chapters", keys, #keys)
	local chapter = chapters[index]
	if not chapter then
		return page
	end
	page.selected = keys[index]
	page.lines = ChapterLines(chapter)
	page.buttons = Steps(keys, index, "Previous chapter", "Next chapter")
	page.footer = string.format("Chapter %d of %d", index, #chapters)
	page.crumb = "Chapter " .. ChapterNumber(chapter)
	page.zone = ChapterPlace(chapter)
	return page
end

-- The texts of the sheet by field, with the edits that the desktop did not confirm yet.
local function SheetTexts(hero, unsaved)
	local texts = {}
	for _, field in ipairs(Entries(hero and hero.sheet)) do
		if type(field.field) == "string" then
			texts[field.field] = field.text
		end
	end
	for field, text in pairs(unsaved.fields) do
		texts[field] = text ~= "" and text or nil
	end
	return texts
end

local function Answer(text)
	return type(text) == "string" and ns.Plain(text) or "Not answered yet."
end

-- The key of the Roleplay Profile in the list (3.7.1). Its fields show under it while it
-- or one of them is open.
local PROFILE = "roleplay"

local function Joined(first, second)
	local keys = {}
	for _, list in ipairs({ first, second }) do
		for _, key in ipairs(list) do
			keys[#keys + 1] = key
		end
	end
	return keys
end

local function ProfileKeys()
	return Joined({ PROFILE }, ns.Hero.PROFILE)
end

local function IsIn(keys, key)
	for _, each in ipairs(keys) do
		if each == key then
			return true
		end
	end
	return false
end

-- An empty name shows the name in the game, which roleplay addons show in its place.
local function ProfileAnswer(field, text)
	if type(text) == "string" then
		return ns.Plain(text)
	end
	if field == "name" then
		return UnitName("player") or Answer(nil)
	end
	return Answer(nil)
end

local function SheetList(texts, profileOpen)
	local rows = {
		Group("About your hero"),
		{ style = "help", text = Journal.USAGE.hero },
	}
	for _, field in ipairs(ns.Hero.FIELDS) do
		rows[#rows + 1] = Item(field, ns.Hero.HINTS[field], Answer(texts[field]))
	end
	rows[#rows + 1] = Item(PROFILE, "Roleplay Profile", ns.MspProfile.State())
	if profileOpen then
		for _, field in ipairs(ns.Hero.PROFILE) do
			rows[#rows + 1] = Item(field, ns.Hero.LABELS[field], ProfileAnswer(field, texts[field]))
		end
	end
	return rows
end

local function EditButton(field, text)
	return {
		label = "Edit",
		run = function()
			ns.Hero.Edit(field, text)
		end,
	}
end

local function FieldLines(field, index, texts, unsaved)
	local text = texts[field]
	local lines = {
		Line("note", string.format("Question %d of %d", index, #ns.Hero.FIELDS)),
		Line("heading", ns.Hero.HINTS[field], EditButton(field, text)),
		Line(type(text) == "string" and "text" or "hint", Answer(text)),
	}
	if unsaved.fields[field] then
		lines[#lines + 1] = Line("hint", SAVING)
	end
	return lines
end

local SHARE_HELP = {
	on = "Players with roleplay addons like Total RP 3 can see this profile.",
	off = "Players with roleplay addons like Total RP 3 can see this profile if you share it.",
}
local SHARED_FIELDS = "They see these six fields, where you're from, and your background."
	.. " Your other answers and your notes stay private."

local function ShareButton()
	local sharing = ns.MspProfile.IsSharing()
	return {
		label = sharing and "Stop sharing" or "Share",
		run = function()
			ns.MspProfile.SetSharing(not sharing)
		end,
	}
end

-- The state of the profile and the switch, or the name of the addon that owns it.
local function ProfileLines()
	local lines = { Line("heading", "Roleplay Profile") }
	local owner = ns.MspProfile.Owner()
	if owner then
		-- The owner can be "your roleplay addon", which starts in lower case.
		local subject = owner:gsub("^%l", string.upper)
		lines[#lines + 1] = Line("help", subject .. " shares your profile. Change it there.")
		return lines
	end
	local help = ns.MspProfile.IsSharing() and SHARE_HELP.on or SHARE_HELP.off
	lines[#lines + 1] = Line("text", help, ShareButton())
	lines[#lines + 1] = Line("help", SHARED_FIELDS)
	return lines
end

-- A field of the profile. Another roleplay addon owns the text, so it has no Edit then.
local function ProfileFieldLines(field, texts, unsaved)
	local text = texts[field]
	local owner = ns.MspProfile.Owner()
	local edit = not owner and EditButton(field, text) or nil
	local lines = {
		Line("note", "Roleplay Profile"),
		Line("heading", ns.Hero.HINTS[field], edit),
		Line(type(text) == "string" and "text" or "hint", ProfileAnswer(field, text)),
	}
	if owner then
		lines[#lines + 1] = Line("hint", "From " .. owner)
	elseif field == "name" and type(text) ~= "string" then
		lines[#lines + 1] = Line("help", "Roleplay addons show your name in the game until you set one.")
	end
	if unsaved.fields[field] then
		lines[#lines + 1] = Line("hint", SAVING)
	end
	return lines
end

local function AnsweredCount(texts)
	local count = 0
	for _, field in ipairs(ns.Hero.FIELDS) do
		if type(texts[field]) == "string" then
			count = count + 1
		end
	end
	return count
end

local function SavedEntry(entry)
	local remove = {
		label = "Remove",
		run = function()
			ns.Hero.Remove(entry)
		end,
	}
	local about = type(entry.npc) == "string" and ("About " .. Name(entry.npc) .. ". ") or ""
	local place = type(entry.place) == "string" and (Name(entry.place) .. ", ") or ""
	return {
		Line("entry", ns.Plain(tostring(entry.text)), remove),
		Line("text", about .. place .. Day(entry.at) .. "."),
	}
end

-- The player's own entries, less the ones that wait for removal, then the new ones.
local function LoreEntries(hero, unsaved)
	local lines = {}
	for _, entry in ipairs(Entries(hero and hero.entries)) do
		if not unsaved.removed[entry.number] then
			for _, line in ipairs(SavedEntry(entry)) do
				lines[#lines + 1] = line
			end
		end
	end
	for _, entry in ipairs(unsaved.added) do
		lines[#lines + 1] = Line("entry", ns.Plain(entry.text))
		lines[#lines + 1] = Line("hint", SAVING)
	end
	return lines
end

local function Lore(hero, unsaved)
	local add = { label = "Add a note", run = ns.Hero.Write }
	local lines = { Line("heading", "Your Notes", add) }
	local entries = LoreEntries(hero, unsaved)
	if #entries == 0 then
		lines[#lines + 1] = Line("help", "Write anything the game can't see: a grudge, a promise, a secret.")
	end
	for _, line in ipairs(entries) do
		lines[#lines + 1] = line
	end
	return lines
end

-- A story that waits for your answer: Accept on its text, Decline on its author.
local function WaitingStory(index, story)
	local accept = {
		label = "Accept",
		run = function()
			ns.PlayerStories.Accept(index)
		end,
	}
	local decline = {
		label = "Decline",
		run = function()
			ns.PlayerStories.Decline(index)
		end,
	}
	return {
		Line("entry", ns.Plain(story.text), accept),
		Line("text", ns.TaskPeople.Short(story.author) .. " told this story about you.", decline),
		Line("help", ns.TaskPages.REPORT),
	}
end

-- An accepted story. One that the story already used stays, so it has no Remove.
local function AcceptedStory(story)
	local remove = not story.used
			and {
				label = "Remove",
				run = function()
					ns.PlayerStories.Remove(story)
				end,
			}
		or nil
	local author = ns.PlayerStories.AuthorOf(story.number)
	local by = author and ns.TaskPeople.Short(author) or "a friend"
	return {
		Line("entry", ns.PlayerStories.Shown(story.text), remove),
		Line("text", "Told by " .. by .. ", " .. Day(story.at) .. "."),
	}
end

-- The stories that players told about you (4.8): the ones that wait, then the accepted ones.
local function Stories(journal)
	local lines = { Line("heading", "Stories About You") }
	local waiting = ns.PlayerStories.Waiting()
	for index = #waiting, 1, -1 do
		for _, line in ipairs(WaitingStory(index, waiting[index])) do
			lines[#lines + 1] = line
		end
	end
	for _, story in ipairs(Entries(journal.stories)) do
		for _, line in ipairs(AcceptedStory(story)) do
			lines[#lines + 1] = line
		end
	end
	if #lines == 1 then
		lines[#lines + 1] = Line("help", "Target someone in your group and type /story to tell a story about them.")
	end
	return lines
end

-- The lines of the open item: a question, the Roleplay Profile, or one of its fields.
local function OpenLines(key, texts, unsaved)
	if key == PROFILE then
		return ProfileLines()
	end
	if IsIn(ns.Hero.PROFILE, key) then
		return ProfileFieldLines(key, texts, unsaved)
	end
	return FieldLines(key, OpenIndex("hero", ns.Hero.FIELDS, 1), texts, unsaved)
end

-- Who the hero is (3.7): the questions of the sheet and the Roleplay Profile on the left,
-- and the open one with the player's own lore on the right.
local function Hero(journal)
	local hero, unsaved = journal.hero, ns.Hero.Unsaved()
	local texts = SheetTexts(hero, unsaved)
	local keys = Joined(ns.Hero.FIELDS, ProfileKeys())
	local key = keys[OpenIndex("hero", keys, 1)]
	local profileOpen = IsIn(ProfileKeys(), key)
	local lines = OpenLines(key, texts, unsaved)
	for _, line in ipairs(Lore(hero, unsaved)) do
		lines[#lines + 1] = line
	end
	for _, line in ipairs(Stories(journal)) do
		lines[#lines + 1] = line
	end
	local steps = profileOpen and ProfileKeys() or ns.Hero.FIELDS
	return {
		list = SheetList(texts, profileOpen),
		selected = key,
		lines = lines,
		buttons = Steps(steps, OpenIndex("hero", steps, 1), "Previous", "Next"),
		footer = string.format("%d of %d answered", AnsweredCount(texts), #ns.Hero.FIELDS),
		side = "sheet",
	}
end

-- A page with no list: the lines alone, beside the map.
local function SinglePage(builder)
	return function(journal)
		return { lines = builder(journal), buttons = {}, side = "map" }
	end
end

-- A page that reads only its own list of the journal.
local function OwnList(builder, section)
	return SinglePage(function(journal)
		return builder(List(journal[section]))
	end)
end

local BUILDERS = {
	hero = Hero,
	chapters = Chronicle,
	deeds = OwnList(Deeds, "deeds"),
	learned = OwnList(Learned, "learned"),
	quests = Tasks,
}

local EMPTY = {
	chapters = "Your story hasn't started yet. Go make some trouble.",
	deeds = "No deeds yet.",
	learned = "You haven't learned a thing yet. Pick up a book, or listen at the inn.",
	quests = "No quests yet. Target someone and type /quest to ask for one.",
}

-- The line at the bottom of the book, on how to use the page.
Journal.USAGE = {
	hero = "Your character's backstory. It shapes your story.",
	chapters = "Your story so far, chapter by chapter.",
	deeds = "Your levels, big kills, deaths, and titles.",
	learned = "Everything you've read or heard.",
	quests = "Quests from the people you meet. Target someone and type /quest.",
}

-- A page is what the book shows for one section:
--   lines: the parchment on the right. Each line is { style, text, action }, and the style
--     is "heading", "section", "prose", "entry", "text", "note", "hint", or "help", or an
--     input of JournalInputs: "field", "money", or "slots".
--   list: the rows on the left, or nil. Each row is { style = "group" | "help" | "item",
--     text, detail, mark, key }. A click on an item selects its key.
--   selected: the key of the open item.
--   buttons: the buttons at the bottom, each { label, run, disabled }.
--   footer: the text at the bottom left.
--   crumb: the last step of the path at the top, or nil for the name of the map.
--   zone: the zone that the map shows, or nil for the zone of the player.
--   map: the id of the map to show, which wins over `zone` when the game has its art.
--   pins: the pins of the map, from `TaskPins.For`, or nil.
--   side: what the left half shows: "map", or "sheet" for the list alone on parchment.
function Journal.Render(journal, section)
	local page = BUILDERS[section](journal)
	page.footer = page.footer or Journal.USAGE[section]
	if #page.lines == 0 then
		page.lines = { Line("help", EMPTY[section]) }
	end
	return page
end

-- Tasks from players need no desktop, so their page shows while the journal loads.
function Journal.Page(section)
	if pages then
		return Journal.Render(pages, section)
	end
	local loading = Line("help", "Loading... If this stays empty, start Gnomish Relay on your computer.")
	if section ~= "quests" then
		return { lines = { loading }, buttons = {}, footer = Journal.USAGE[section], side = "map" }
	end
	local page = Journal.Render(Started(), section)
	if not ns.TaskPages.Owns(page.selected) then
		page.lines = { loading }
	end
	return page
end

-- The subzones of this zone that the player visited, in the order of the first visit.
function Journal.VisitedIn(zone)
	local names = {}
	for _, place in ipairs(pages and Entries(pages.places) or {}) do
		if place.within == zone and type(place.name) == "string" then
			names[#names + 1] = ns.Plain(place.name)
		end
	end
	return names
end

function Journal.Lines(section)
	return Journal.Page(section).lines
end
