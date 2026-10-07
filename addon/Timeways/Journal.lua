-- The pages of the journal: the hero, the chronicle, the stories of players, what you know
-- place by place, and your side quests (GAMEPLAY.md 3.1.1, 3.4, 3.6, 3.7, and 4.8).
-- The desktop sends them, because the world lives there and never in the saved variables
-- (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

-- The deeds have no tab: each one shows in its chapter and on its place in Knowledge.
Journal.SECTIONS = { "hero", "chapters", "stories", "knowledge", "quests" }
Journal.TITLES = {
	hero = "Hero",
	chapters = "Chronicle",
	stories = "Stories",
	knowledge = "Knowledge",
	quests = "Quests",
}

-- The lists that come in pages, with the sheet and the entries of the hero.
local LISTS = {
	"chapters",
	"tales",
	"edits",
	"histories",
	"places",
	"people",
	"lore",
	"deeds",
	"learned",
	"quests",
	"stories",
}

-- A reply holds at most 24 KB, so a long journal comes in pages. More than this many
-- pages means a broken reply, not a long journal.
local MAX_PAGES = 64

-- The whole journal, once every page came. It stays while a newer one comes in.
local pages
-- The pages so far of the journal that comes in, and the page that comes next.
local collecting

local Entries = ns.JournalRows.Entries
local Line = ns.JournalRows.Line

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
	local journal = {}
	for _, list in ipairs(LISTS) do
		journal[list] = {}
	end
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
		ns.Dev.DesktopSays(value.dev)
		collecting = Started()
		collecting.talk_quest = value.talk_quest
		collecting.summary = value.summary
		ns.Hero.ShowRefused(value.hero_refused)
		ns.Hero.ShowRefused(value.edit_refused)
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
	ns.JournalEdits.JournalCame()
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

-- The open item of each page with a list, by its key. A key that is gone opens the default.
local selected = {}

function Journal.Select(section, key)
	selected[section] = key
end

function Journal.Selected(section)
	return selected[section]
end

local BUILDERS = {
	hero = ns.JournalHero.Page,
	chapters = ns.JournalChronicle.Page,
	stories = ns.JournalStories.Page,
	knowledge = ns.JournalKnowledge.Page,
	quests = ns.JournalQuests.Page,
}

local EMPTY = {
	quests = "No quests yet. Target someone and type /quest to ask for one.",
}

-- The line at the bottom of the book, on how to use the page.
Journal.USAGE = {
	hero = ns.JournalHero.USAGE,
	chapters = ns.JournalChronicle.USAGE,
	stories = ns.JournalStories.USAGE,
	knowledge = ns.JournalKnowledge.USAGE,
	quests = "Quests from the people you meet. Target someone and type /quest.",
}

-- A tab that starts fresh each time it opens: Knowledge opens on the zone where you stand.
local OPENED = { knowledge = ns.JournalKnowledge.Home }

function Journal.Opened(section)
	if OPENED[section] then
		OPENED[section]()
	end
end

-- A page is what the book shows for one section:
--   lines: the parchment on the right. Each line is { style, text, action }, and the style
--     is "heading", "section", "prose", "entry", "text", "note", "hint", or "help", or an
--     input of JournalInputs: "field", "money", or "slots".
--   list: the rows on the left, or nil. Each row is { style = "group" | "help" | "item",
--     text, detail, mark, key }. A click on an item selects its key.
--   selected: the key of the open item.
--   buttons: the buttons at the bottom, each { label, run, disabled }.
--   edit: the run of the Edit button when the player can edit the text of the page, or nil.
--     A press anywhere on the text runs it too.
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
