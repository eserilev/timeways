-- The pages of the journal: the hero, the chronicle, the stories of players, deeds, what you
-- learned, and your side quests (GAMEPLAY.md 3.1.1, 3.4, 3.6, 3.7, and 4.8).
-- The desktop sends them, because the world lives there and never in the saved variables
-- (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

Journal.SECTIONS = { "hero", "chapters", "stories", "deeds", "learned", "quests" }
Journal.TITLES = {
	hero = "Hero",
	chapters = "Chronicle",
	stories = "Stories",
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

local List = ns.JournalRows.List
local Entries = ns.JournalRows.Entries
local Name = ns.JournalRows.Name
local Day = ns.JournalRows.Day
local Line = ns.JournalRows.Line
local Group = ns.JournalRows.Group
local Item = ns.JournalRows.Item
local Button = ns.JournalRows.Button
local Keys = ns.JournalRows.Keys

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

-- The whole journal that the book shows, or nil while the first one loads.
function Journal.Current()
	return pages
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
		collecting.talk_quest = value.talk_quest
		collecting.summary = value.summary
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
	ns.TalkWindow.JournalCame()
	ns.Hero.AskOnce(pages.hero)
end

-- The quest that the newest talk with work asked for (GAMEPLAY.md 3.5), or nil when the
-- journal has none or a broken one. Each is { npc, at, state, number or line }.
function Journal.TalkQuest()
	local quest = pages and pages.talk_quest
	if type(quest) ~= "table" or type(quest.npc) ~= "string" or type(quest.at) ~= "number" then
		return nil
	end
	local offered = quest.state == "offered" and type(quest.number) == "number"
	local refused = quest.state == "refused" and type(quest.line) == "string"
	if quest.state == "writing" or offered or refused then
		return quest
	end
	return nil
end

-- The side quest with this number, or nil.
function Journal.Quest(number)
	for _, quest in ipairs(pages and Entries(pages.quests) or {}) do
		if quest.number == number then
			return quest
		end
	end
	return nil
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
	elseif deed.kind == "mounted" then
		local first = deed.epic and "Rode your first epic mount, " or "Rode your first mount, "
		return first .. Name(deed.mount)
	elseif deed.kind == "epic_item" then
		return "Equipped your first epic item, " .. Name(deed.item)
	elseif deed.kind == "upgraded" then
		return "Equipped " .. Name(deed.item) .. ", a big upgrade"
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
		lines[#lines + 1] = Line("prose", ns.WithName(chapter.prose))
	end
	for _, footnote in ipairs(List(chapter.footnotes)) do
		if type(footnote) == "string" then
			lines[#lines + 1] = Line("note", "* " .. ns.WithName(footnote))
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
}

-- Only what you read. The words of a talk live in the talk window (3.5), so an older
-- desktop's "rumor" entry shows nothing.
local function Learned(entries)
	local lines = {}
	for _, entry in ipairs(entries) do
		local title = LEARNED_TITLES[entry.kind]
		if title then
			lines[#lines + 1] = Line("entry", title(entry))
			if type(entry.excerpt) == "string" then
				lines[#lines + 1] = Line("prose", ns.WithName(entry.excerpt))
			end
			local place = type(entry.place) == "string" and (Name(entry.place) .. ", ") or ""
			lines[#lines + 1] = Line("text", place .. Day(entry.at) .. ".")
		end
	end
	return lines
end

-- The open item of each page with a list, by its key. A key that is gone opens the default.
local selected = {}

local function OpenIndex(section, keys, default)
	return ns.JournalRows.OpenIndex(keys, selected[section], default)
end

function Journal.Select(section, key)
	selected[section] = key
end

function Journal.Selected(section)
	return selected[section]
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

-- The key of the title page of the Chronicle: who you've become
-- (docs/plans/hero-stories.md 4.6).
local TITLE_PAGE = "title"

-- "Level 12 Undead Paladin", from the game.
local function LevelLine()
	local race = UnitRace("player") or ""
	local class = UnitClass("player") or ""
	return string.format("Level %d %s %s", UnitLevel("player") or 0, race, class)
end

-- The summary that a model writes when a chapter ends. With no model, the page holds the
-- header alone.
local function TitleLines(journal, chapterEnded)
	local lines = {
		Line("heading", UnitName("player") or "?"),
		Line("text", LevelLine()),
	}
	if type(journal.summary) == "string" then
		lines[#lines + 1] = Line("prose", ns.WithName(journal.summary))
	elseif not chapterEnded then
		lines[#lines + 1] = Line("help", "Fills in when your first chapter ends.")
	end
	return lines
end

-- The title page comes first, and the book opens on the newest chapter, the one still
-- being written.
local function Chronicle(journal)
	local chapters = Entries(journal.chapters)
	local title = Item(TITLE_PAGE, UnitName("player") or "?", "Who you've become")
	local items, list = { title }, { title }
	if #chapters > 0 then
		list[2] = Group("Chapters")
	end
	for n, chapter in ipairs(chapters) do
		local key = type(chapter.number) == "number" and chapter.number or ("#" .. n)
		items[n + 1] = Item(key, ChapterTitle(chapter), Dates(chapter))
		list[#list + 1] = items[n + 1]
	end
	local keys = Keys(items)
	local index = OpenIndex("chapters", keys, #keys)
	local page = { list = list, buttons = Steps(keys, index, "Previous chapter", "Next chapter"), side = "map" }
	page.selected = keys[index]
	local chapter = chapters[index - 1]
	if not chapter then
		page.lines = TitleLines(journal, #chapters >= 2)
		return page
	end
	page.lines = ChapterLines(chapter)
	page.footer = string.format("Chapter %d of %d", index - 1, #chapters)
	page.crumb = "Chapter " .. ChapterNumber(chapter)
	page.zone = ChapterPlace(chapter)
	return page
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
	hero = ns.JournalHero.Page,
	chapters = Chronicle,
	stories = ns.JournalStories.Page,
	deeds = OwnList(Deeds, "deeds"),
	learned = OwnList(Learned, "learned"),
	quests = ns.JournalQuests.Page,
}

local EMPTY = {
	deeds = "No deeds yet.",
	learned = "You haven't learned a thing yet. Pick up a book, or listen at the inn.",
	quests = "No quests yet. Target someone and type /quest to ask for one.",
}

-- The line at the bottom of the book, on how to use the page.
Journal.USAGE = {
	hero = ns.JournalHero.USAGE,
	chapters = "Your story so far, chapter by chapter.",
	stories = ns.JournalStories.USAGE,
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
--   side: what the left half shows: "map", "sheet" for the list alone on parchment, or
--     "cards" for the cards of JournalCards.
--   cards: the sub-tabs and the cards of the left half, for the side "cards".
function Journal.Render(journal, section)
	local page = BUILDERS[section](journal)
	page.footer = page.footer or Journal.USAGE[section]
	if #page.lines == 0 then
		page.lines = { Line("help", EMPTY[section]) }
	end
	return page
end

-- The count on the tab of a section, or nil for none.
local BADGES = { stories = ns.JournalStories.Badge }

function Journal.Badge(section)
	return BADGES[section] and BADGES[section]()
end

-- Tasks and stories from players need no desktop, so their pages show while the journal
-- loads.
function Journal.Page(section)
	if pages then
		return Journal.Render(pages, section)
	end
	if section == "stories" then
		return ns.JournalStories.Page(nil)
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
