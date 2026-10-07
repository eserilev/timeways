-- The Chronicle as a book (docs/plans/chapters.md 6): the title page, one page for each
-- chapter, and one page for each tale of a dungeon, a raid, or a battleground. A tale
-- follows the chapter that held its first step. The contents on the left group the chapters
-- by level band, newest first.

local _, ns = ...

local JournalChronicle = {}
ns.JournalChronicle = JournalChronicle

local List = ns.JournalRows.List
local Entries = ns.JournalRows.Entries
local Name = ns.JournalRows.Name
local Day = ns.JournalRows.Day
local Line = ns.JournalRows.Line
local Group = ns.JournalRows.Group
local Item = ns.JournalRows.Item
local Button = ns.JournalRows.Button
local Keys = ns.JournalRows.Keys

JournalChronicle.USAGE = "Your story so far, chapter by chapter."

-- The key of the title page: who you've become (docs/plans/hero-stories.md 4.6).
local TITLE_PAGE = "title"

-- The levels of one band of the contents: 1 to 9, 10 to 19, and so on.
local BAND = 10

local KINDS = { dungeon = "Dungeon", raid = "Raid", battleground = "Battleground" }

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

local function Number(chapter)
	return type(chapter.number) == "number" and chapter.number or "?"
end

-- The title from the desktop: the zone where the chapter opened, "Return to Westfall", or
-- "Westfall, continued".
local function Title(chapter)
	return type(chapter.title) == "string" and ns.Plain(chapter.title) or nil
end

-- The first zone where the chapter gained weight: the map of its page.
local function Place(chapter)
	local zones = List(chapter.zones)
	return type(zones[1]) == "string" and ns.Plain(zones[1]) or nil
end

local function IsOpen(chapter)
	return chapter.state == "open"
end

-- A session can run past midnight, so a chapter can span days.
local function Dates(entry)
	local began, ended = Day(entry.began), Day(entry.ended)
	if type(entry.ended) ~= "number" or began == ended then
		return began
	end
	return began .. " to " .. ended
end

local function Levels(chapter)
	local levels = List(chapter.levels)
	local from, to = levels[1], levels[2]
	if type(from) ~= "number" or type(to) ~= "number" then
		return nil
	end
	if from == to then
		return "Level " .. to
	end
	return string.format("Levels %d to %d", from, to)
end

-- "3 Oct 2026 · Levels 20 to 22"
local function When(chapter)
	local levels = Levels(chapter)
	return levels and (Dates(chapter) .. " · " .. levels) or Dates(chapter)
end

local function Runs(tale)
	local runs = type(tale.runs) == "number" and tale.runs or 0
	return runs == 1 and "1 run" or (runs .. " runs")
end

local function KindOf(tale)
	return KINDS[tale.kind] or "Dungeon"
end

-- Who killed you, and where: "Bleak Worg at The Dead Field".
local function Death(deed)
	local killer = type(deed.killer) == "string" and deed.killer or "Unknown"
	return type(deed.place) == "string" and (killer .. " at " .. deed.place) or killer
end

local QUESTS = { quest_done = true, game_quest_done = true, class_quest_done = true }

-- The deeds by kind: first kills, quests, deaths, and the rest as their own lines.
local function Sorted(deeds)
	local sorted = { defeated = {}, quests = {}, deaths = {}, others = {} }
	for _, deed in ipairs(deeds) do
		if deed.kind == "defeated" and deed.times == 1 then
			table.insert(sorted.defeated, deed.foe)
		elseif QUESTS[deed.kind] then
			table.insert(sorted.quests, deed.title)
		elseif deed.kind == "died" then
			table.insert(sorted.deaths, Death(deed))
		elseif deed.kind ~= "level" then
			table.insert(sorted.others, deed)
		end
	end
	return sorted
end

local function Row(lines, label, names)
	if #names > 0 then
		lines[#lines + 1] = Line("text", label .. ": " .. Together(names) .. ".")
	end
end

-- Each name that Knowledge knows opens its page there.
local function Linked(names, link)
	local shown = {}
	for _, name in ipairs(names) do
		shown[#shown + 1] = type(name) == "string" and link(name) or name
	end
	return shown
end

local function PlaceLink(index, name)
	local n = ns.KnowledgeIndex.PlaceNumber(index, name)
	return n and ns.JournalLinks.Text("place", n, Name(name)) or name
end

local function PersonLink(index, name)
	local n = ns.KnowledgeIndex.PersonNumber(index, name)
	return n and ns.JournalLinks.Text("person", n, Name(name)) or name
end

-- The NPCs that you talked to while the chapter was open (3.5), from the talks that this
-- computer keeps.
local function TalksIn(chapter)
	local names = {}
	for _, npc in ipairs(ns.TalkHistory.Npcs()) do
		for _, exchange in ipairs(ns.TalkHistory.Of(npc)) do
			if ns.KnowledgeIndex.InChapter(chapter, exchange.at) then
				names[#names + 1] = npc
				break
			end
		end
	end
	return names
end

-- The stories that you accepted while the chapter was open (4.8).
local function StoriesIn(chapter, stories)
	local titles = {}
	for _, story in ipairs(stories) do
		if ns.KnowledgeIndex.InChapter(chapter, story.at) and type(story.title) == "string" then
			titles[#titles + 1] = ns.WithName(story.title)
		end
	end
	return titles
end

-- The tally lines of repeats, the deeds of no kind, and what the lists left out.
local function Rest(lines, entry, others)
	for _, deed in ipairs(others) do
		local title = ns.JournalRows.DeedTitle(deed)
		if title then
			lines[#lines + 1] = Line("entry", title .. ".")
		end
	end
	for _, again in ipairs(List(entry.again)) do
		if type(again) == "string" then
			lines[#lines + 1] = Line("text", again)
		end
	end
	if type(entry.left_out) == "number" and entry.left_out > 0 then
		lines[#lines + 1] = Line("text", string.format("And %d more.", entry.left_out))
	end
end

-- The story of the narrator, then the player's own paragraphs, then the notes. The desktop
-- leaves out the narrator's story when the player's text stands in its place.
local function Story(lines, prose, footnotes, edit)
	if type(prose) == "string" then
		lines[#lines + 1] = Line("prose", ns.WithName(prose))
	end
	for _, paragraph in ipairs(ns.JournalEdits.Paragraphs(edit)) do
		lines[#lines + 1] = Line("prose", paragraph)
	end
	for _, footnote in ipairs(List(footnotes)) do
		if type(footnote) == "string" then
			lines[#lines + 1] = Line("note", "* " .. ns.WithName(footnote))
		end
	end
end

-- While an edit of the page is on its way.
local function Saving(lines, entry)
	if ns.JournalEdits.IsSaving(entry) then
		lines[#lines + 1] = Line("help", ns.JournalRows.SAVING)
	end
end

-- "Chapter 8", the title, the dates and levels, the story, the notes, and "In this
-- chapter": one line for each kind, in the order places, people, talks, defeated, quests,
-- deaths, stories. Each place and person opens its page of Knowledge.
function JournalChronicle.ChapterLines(journal, chapter, edit)
	local index = ns.KnowledgeIndex.Of(journal)
	local lines = {}
	local title = ns.JournalEdits.Title(edit) or Title(chapter) or Place(chapter)
	if title then
		lines[#lines + 1] = Line("note", "Chapter " .. Number(chapter))
	end
	lines[#lines + 1] = Line("heading", title or ("Chapter " .. Number(chapter)))
	lines[#lines + 1] = Line("text", When(chapter) .. ".")
	if IsOpen(chapter) then
		lines[#lines + 1] = Line("help", "This chapter isn't over yet. Its story comes when the next one starts.")
	end
	Saving(lines, ns.JournalEdits.Chapter(chapter.first))
	Story(lines, chapter.prose, chapter.footnotes, edit)
	local sorted = Sorted(Entries(chapter.deeds))
	local list = {}
	Row(
		list,
		"Places",
		Linked(List(chapter.zones), function(name)
			return PlaceLink(index, name)
		end)
	)
	Row(
		list,
		"People",
		Linked(List(chapter.people), function(name)
			return PersonLink(index, name)
		end)
	)
	Row(
		list,
		"Talked to",
		Linked(TalksIn(chapter), function(name)
			return PersonLink(index, name)
		end)
	)
	Row(list, "Defeated", sorted.defeated)
	Row(list, "Quests", sorted.quests)
	Row(list, "Deaths", sorted.deaths)
	Row(list, "Stories", StoriesIn(chapter, Entries(journal.stories)))
	Rest(list, chapter, sorted.others)
	if #list > 0 then
		lines[#lines + 1] = Line("section", "In this chapter")
	end
	for _, line in ipairs(list) do
		lines[#lines + 1] = line
	end
	return lines
end

-- "Dungeon", the instance, the day of the first run and the count of runs, the story, and
-- what happened inside.
function JournalChronicle.TaleLines(tale, edit)
	local kind = KindOf(tale)
	local lines = {
		Line("note", kind),
		Line("heading", ns.JournalEdits.Title(edit) or Name(tale.instance)),
		Line("text", Day(tale.began) .. " · " .. Runs(tale) .. "."),
	}
	Saving(lines, ns.JournalEdits.Tale(tale.first))
	Story(lines, tale.text, nil, edit)
	local sorted = Sorted(Entries(tale.deeds))
	local list = {}
	Row(list, "Defeated", sorted.defeated)
	Row(list, "Quests", sorted.quests)
	Row(list, "Deaths", sorted.deaths)
	Rest(list, tale, sorted.others)
	if #list > 0 then
		lines[#lines + 1] = Line("section", "In this " .. kind:lower())
	end
	for _, line in ipairs(list) do
		lines[#lines + 1] = line
	end
	return lines
end

-- "Level 12 Undead Paladin", from the game.
local function LevelLine()
	local race = UnitRace("player") or ""
	local class = UnitClass("player") or ""
	return string.format("Level %d %s %s", UnitLevel("player") or 0, race, class)
end

-- The summary that a model writes when a chapter ends. With no model, the page holds the
-- header alone.
local function TitleLines(journal, chapterEnded, edit)
	local lines = {
		Line("heading", UnitName("player") or "?"),
		Line("text", LevelLine()),
	}
	Saving(lines, ns.JournalEdits.SUMMARY)
	if type(journal.summary) ~= "string" and not edit and not chapterEnded then
		lines[#lines + 1] = Line("help", "Fills in when your first chapter ends.")
	end
	Story(lines, journal.summary, nil, edit)
	return lines
end

-- The one mark of a row of the contents: what opened the chapter, or what happened in it.
local MARKS = {
	level = "Level",
	capital = "Capital",
	["return"] = "Return",
}

local function Mark(chapter, tales)
	if #tales > 0 then
		return KindOf(tales[1])
	end
	if chapter.opened_by == "level" then
		local levels = List(chapter.levels)
		return type(levels[2]) == "number" and ("Level " .. levels[2]) or MARKS.level
	end
	local sorted = Sorted(Entries(chapter.deeds))
	if #sorted.defeated > 0 then
		return "Kill"
	end
	if #sorted.deaths > 0 then
		return "Death"
	end
	return MARKS[chapter.opened_by]
end

local function ChapterKey(chapter, n)
	return type(chapter.number) == "number" and chapter.number or ("#" .. n)
end

local function TaleKey(tale)
	return "tale:" .. tostring(tale.first)
end

-- The tales that follow each chapter, by the first event of the chapter.
local function TalesByChapter(tales)
	local by = {}
	for _, tale in ipairs(tales) do
		local chapter = tale.chapter
		by[chapter] = by[chapter] or {}
		table.insert(by[chapter], tale)
	end
	return by
end

local function Band(chapter)
	local levels = List(chapter.levels)
	local level = levels[2]
	if type(level) ~= "number" then
		return nil
	end
	local low = math.floor(level / BAND) * BAND
	return math.max(low, 1), low + BAND - 1
end

local function BandTitle(chapter)
	local low, high = Band(chapter)
	return low and string.format("Levels %d to %d", low, high) or "Chapters"
end

-- The pages in the order of the book: the title page, then each chapter and its tales.
-- Each page is { key, item, chapter or tale }.
-- A small "Edited" after the detail of a row that the player changed.
local function Edited(detail, edit)
	return edit and (detail .. " · Edited") or detail
end

local function TalePage(journal, tale, number)
	local key = TaleKey(tale)
	local edit = ns.JournalEdits.Of(journal, ns.JournalEdits.Tale(tale.first))
	local title = ns.JournalEdits.Title(edit) or Name(tale.instance)
	local item = Item(key, title, Edited(KindOf(tale) .. " · " .. Runs(tale), edit))
	return { key = key, item = item, tale = tale, number = number, edit = edit }
end

local function ChapterPage(journal, chapter, n, own)
	local edit = ns.JournalEdits.Of(journal, ns.JournalEdits.Chapter(chapter.first))
	local title = ns.JournalEdits.Title(edit) or Title(chapter) or Place(chapter)
	local text = "Chapter " .. Number(chapter) .. (title and (": " .. title) or "")
	local detail = Edited(IsOpen(chapter) and (Dates(chapter) .. " · now") or Dates(chapter), edit)
	local key = ChapterKey(chapter, n)
	return { key = key, item = Item(key, text, detail, Mark(chapter, own)), chapter = chapter, edit = edit }
end

local function BookPages(journal)
	local chapters = Entries(journal.chapters)
	local tales = TalesByChapter(Entries(journal.tales))
	local summaryEdit = ns.JournalEdits.Of(journal, ns.JournalEdits.SUMMARY)
	local title = Item(TITLE_PAGE, UnitName("player") or "?", Edited("Who you've become", summaryEdit))
	local pages = { { key = TITLE_PAGE, item = title, edit = summaryEdit } }
	for n, chapter in ipairs(chapters) do
		local own = tales[chapter.first] or {}
		pages[#pages + 1] = ChapterPage(journal, chapter, n, own)
		for _, tale in ipairs(own) do
			pages[#pages + 1] = TalePage(journal, tale, Number(chapter))
		end
	end
	return pages
end

-- The contents: the title page, then the chapters grouped by level band, newest first. A
-- tale stays under its chapter.
local function Contents(pages)
	local list = { pages[1].item }
	local band
	for n = #pages, 2, -1 do
		local page = pages[n]
		if page.chapter then
			local title = BandTitle(page.chapter)
			if title ~= band then
				band = title
				list[#list + 1] = Group(title)
			end
			list[#list + 1] = page.item
			-- The tales of the chapter come after it, in their order.
			for m = n + 1, #pages do
				if pages[m].chapter then
					break
				end
				list[#list + 1] = pages[m].item
			end
		end
	end
	return list
end

-- The buttons that open the page before and after the open one.
local function Steps(keys, index)
	local function Open(n)
		return function()
			ns.JournalFrame.Select(keys[n])
		end
	end
	return {
		Button("Previous chapter", Open(index - 1), index <= 1),
		Button("Next chapter", Open(index + 1), index >= #keys),
	}
end

local function ChapterCount(journal)
	return #Entries(journal.chapters)
end

local function AnyClosed(journal)
	for _, chapter in ipairs(Entries(journal.chapters)) do
		if chapter.state == "closed" then
			return true
		end
	end
	return false
end

-- The book opens on the newest chapter, the one still being written.
local function NewestChapter(pages)
	for n = #pages, 1, -1 do
		if pages[n].chapter then
			return n
		end
	end
	return 1
end

-- The Edit button of a page opens the box with what the page shows.
local function EditOf(journal, page)
	return function()
		if page.chapter then
			local chapter = page.chapter
			local prose = type(chapter.prose) == "string" and ns.WithName(chapter.prose) or nil
			local entry = ns.JournalEdits.Chapter(chapter.first)
			local label = "Chapter " .. Number(chapter)
			ns.JournalEdits.Open(entry, label, prose, page.edit, Title(chapter) or Place(chapter))
		elseif page.tale then
			local tale = page.tale
			local prose = type(tale.text) == "string" and ns.WithName(tale.text) or nil
			local entry = ns.JournalEdits.Tale(tale.first)
			ns.JournalEdits.Open(entry, Name(tale.instance), prose, page.edit, Name(tale.instance))
		else
			local prose = type(journal.summary) == "string" and ns.WithName(journal.summary) or nil
			ns.JournalEdits.Open(ns.JournalEdits.SUMMARY, "Who you've become", prose, page.edit, nil)
		end
	end
end

function JournalChronicle.Page(journal)
	local pages = BookPages(journal)
	local items = {}
	for n, page in ipairs(pages) do
		items[n] = page.item
	end
	local keys = Keys(items)
	local index = ns.JournalRows.OpenIndex(keys, ns.Journal.Selected("chapters"), NewestChapter(pages))
	local open = pages[index]
	local result = { list = Contents(pages), side = "map", selected = keys[index] }
	local count = ChapterCount(journal)
	if open.chapter then
		result.lines = JournalChronicle.ChapterLines(journal, open.chapter, open.edit)
		result.pins = ns.AtlasPins.ForChapter(ns.KnowledgeIndex.Of(journal), open.chapter)
		result.footer = string.format("Chapter %s of %d", tostring(Number(open.chapter)), count)
		result.crumb = "Chapter " .. Number(open.chapter)
		result.zone = Place(open.chapter)
	elseif open.tale then
		result.lines = JournalChronicle.TaleLines(open.tale, open.edit)
		result.footer = "After chapter " .. open.number
		result.crumb = Name(open.tale.instance)
		result.zone = Name(open.tale.instance)
	else
		result.lines = TitleLines(journal, AnyClosed(journal), open.edit)
	end
	result.edit = EditOf(journal, open)
	result.buttons = ns.JournalEdits.Buttons(result.edit, open.edit)
	ns.Ratings.AddButtons(result.buttons, open, journal)
	for _, step in ipairs(Steps(keys, index)) do
		result.buttons[#result.buttons + 1] = step
	end
	return result
end

-- A chapter that a page of Knowledge names opens here.
ns.JournalLinks.OPENERS.chapter = function(number)
	ns.JournalFrame.Open("chapters")
	ns.JournalFrame.Select(number)
end
