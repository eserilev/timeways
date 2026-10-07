-- Knowledge as an atlas (GAMEPLAY.md 3.6): the map is the index, and the page opens on the
-- zone where you stand. A click on a zone, a place pin, a person pin, or a name opens what
-- your character knows of it: its lore, its people, its quests, what you read and heard,
-- the deaths and the kills, and its chapters. Counts, never totals.

local _, ns = ...

local JournalKnowledge = {}
ns.JournalKnowledge = JournalKnowledge

local Index = ns.KnowledgeIndex
local Link = ns.JournalLinks.Text
local Line = ns.JournalRows.Line
local Name = ns.JournalRows.Name
local Day = ns.JournalRows.Day
local Button = ns.JournalRows.Button

JournalKnowledge.USAGE = "What you know, place by place."

-- The rows of a list before "Show all".
local SHOWN = 3
-- The passages of lore that a zone page shows: its own, then those of its subzones.
local PASSAGES = 3

JournalKnowledge.ICONS = {
	place = "Interface\\Icons\\INV_Misc_Map_01",
	person = "Interface\\GossipFrame\\GossipGossipIcon",
	quest = "Interface\\GossipFrame\\ActiveQuestIcon",
	read = "Interface\\Icons\\INV_Misc_Book_09",
	death = "Interface\\TargetingFrame\\UI-TargetingFrame-Skull",
	kill = "Interface\\Icons\\INV_Sword_04",
}
local ICONS = JournalKnowledge.ICONS

-- The pages that the player opened, newest last. Each is { kind, name }. Empty is the
-- zone where the player stands.
local trail = {}
-- The lists that show every row, by the page and the list.
local expanded = {}

-- On a loading screen the game names no zone, and the world page shows.
local function Here()
	local zone = GetRealZoneText()
	if type(zone) ~= "string" or zone == "" then
		return { kind = "world", name = 0 }
	end
	return { kind = "place", name = zone }
end

local function Current()
	return trail[#trail] or Here()
end

local function PageKey(at)
	return at.kind .. ":" .. tostring(at.name)
end

-- The tab opens on the zone where you stand.
function JournalKnowledge.Home()
	trail, expanded = {}, {}
end

function JournalKnowledge.Back()
	trail[#trail] = nil
	ns.JournalFrame.Turn()
end

-- Opens a page of Knowledge, from any tab.
function JournalKnowledge.Show(kind, name)
	if ns.JournalFrame.Section() ~= "knowledge" then
		ns.JournalFrame.Open("knowledge")
	end
	local at = Current()
	if at.kind ~= kind or at.name ~= name then
		trail[#trail + 1] = { kind = kind, name = name }
	end
	ns.JournalFrame.Turn()
end

function JournalKnowledge.Where()
	local at = Current()
	return at.kind, at.name
end

local function Section(lines, label)
	lines[#lines + 1] = Line("section", label)
end

local function Note(lines, text)
	lines[#lines + 1] = Line("note", text)
end

-- One row of a list: an icon, the text with its link, and a faded word on the right.
local function Row(icon, text, detail)
	local line = Line("link", text)
	line.icon, line.detail = icon, detail
	return line
end

local function PlaceLink(index, name)
	local n = Index.PlaceNumber(index, name)
	return n and Link("place", n, Name(name)) or Name(name)
end

local function PersonLink(index, name)
	local n = Index.PersonNumber(index, name)
	return n and Link("person", n, Name(name)) or Name(name)
end

local function ChapterLink(chapter)
	local number = type(chapter.number) == "number" and chapter.number or nil
	local label = "Chapter " .. tostring(number or "?")
	return number and Link("chapter", number, label) or label
end

-- "Ch. 4", for the right of a row.
local function ChapterMark(index, at)
	local chapter = Index.ChapterAt(index, at)
	return chapter and type(chapter.number) == "number" and ("Ch. " .. chapter.number) or nil
end

local function Pills(lines, counts)
	if #counts > 0 then
		local line = Line("pills", table.concat(counts, " · "))
		line.pills = counts
		lines[#lines + 1] = line
	end
end

-- The rows of a list, with "Show all 9" while it is folded.
local function Rows(lines, at, list, label, rows)
	if #rows == 0 then
		return
	end
	Section(lines, label)
	local key = PageKey(at) .. ":" .. list
	local all = expanded[key] or #rows <= SHOWN
	for n, row in ipairs(rows) do
		if all or n <= SHOWN then
			lines[#lines + 1] = row
		end
	end
	if not all then
		lines[#lines + 1] = Line("text", Link("all", list, "Show all " .. #rows))
	end
end

-- The standing word of the tooltip of the NPC: one name for one feeling everywhere.
local function Standing(person)
	return ns.Trust.Word(person.trust)
end

local function PeopleRows(index, people)
	local rows = {}
	for _, person in ipairs(people) do
		rows[#rows + 1] = Row(ICONS.person, PersonLink(index, person.name), Standing(person))
	end
	return rows
end

-- A side quest opens on the Quests tab, and a quest of the game opens the text you read.
local function QuestLink(index, deed)
	local title = Name(deed.title)
	if deed.kind == "quest_done" then
		for _, quest in ipairs(index.quests) do
			if quest.title == deed.title and type(quest.number) == "number" then
				return Link("quest", quest.number, title)
			end
		end
		return title
	end
	local text = Index.QuestText(index, deed.title)
	return text and Link("text", text, title) or title
end

local function QuestRows(index, deeds)
	local rows = {}
	for _, deed in ipairs(deeds) do
		rows[#rows + 1] = Row(ICONS.quest, QuestLink(index, deed), ChapterMark(index, deed.at))
	end
	return rows
end

local READ_TITLES = {
	book = function(entry)
		return Name(entry.title)
	end,
	quest = function(entry)
		return Name(entry.title)
	end,
	gossip = function(entry)
		return "Heard from " .. Name(entry.npc)
	end,
}

local function ReadRows(index, entries)
	local rows = {}
	for _, entry in ipairs(entries) do
		local n = Index.ReadNumber(index, entry)
		local title = READ_TITLES[entry.kind](entry)
		local where = type(entry.npc) == "string" and entry.kind ~= "gossip" and Name(entry.npc) or nil
		rows[#rows + 1] = Row(ICONS.read, n and Link("text", n, title) or title, where)
	end
	return rows
end

local function DeathRows(index, known)
	local rows = {}
	for _, deed in ipairs(known.deaths) do
		local what = type(deed.killer) == "string" and ("Killed by " .. Name(deed.killer)) or "Died"
		local where = deed.place ~= known.name and type(deed.place) == "string" and Name(deed.place) or nil
		rows[#rows + 1] = Row(ICONS.death, what, where or ChapterMark(index, deed.at))
	end
	for _, deed in ipairs(known.kills) do
		rows[#rows + 1] = Row(ICONS.kill, "Defeated " .. Name(deed.foe), ChapterMark(index, deed.at))
	end
	return rows
end

local function Chapters(lines, chapters)
	if #chapters == 0 then
		return
	end
	Section(lines, "Chapters")
	local links = {}
	for _, chapter in ipairs(chapters) do
		links[#links + 1] = ChapterLink(chapter)
	end
	lines[#lines + 1] = Line("text", table.concat(links, ", "))
end

-- "First visit 28 Sep 2026, in Chapter 4"
local function FirstVisit(index, place)
	local chapter = Index.ChapterAt(index, place.first_visit)
	local when = "First visit " .. Day(place.first_visit)
	return chapter and (when .. ", in " .. ChapterLink(chapter)) or when
end

local function LoreTexts(index, names)
	local texts = {}
	for _, name in ipairs(names) do
		local lore = Index.Lore(index, name)
		if lore and type(lore.text) == "string" and #texts < PASSAGES then
			texts[#texts + 1] = lore.text
		end
	end
	return texts
end

local function WhatYouKnow(lines, texts)
	if #texts == 0 then
		return
	end
	Section(lines, "What you know")
	for _, text in ipairs(texts) do
		lines[#lines + 1] = Line("prose", ns.WithName(text))
	end
end

-- "There's more to learn here." says only that more exists, never what.
local function MoreToLearn(index, names)
	for _, name in ipairs(names) do
		local lore = Index.Lore(index, name)
		if lore and lore.more == true then
			return true
		end
	end
	return false
end

local EMPTY = "Nothing yet. People you meet, books you read, and quests you finish show up here."

local function ZoneLines(index, at, zone)
	local known = Index.Place(index, zone)
	local names = { zone }
	for _, subzone in ipairs(Index.Subzones(index, zone)) do
		names[#names + 1] = subzone.name
	end
	local lines = { Line("heading", Name(zone)), Line("note", FirstVisit(index, known.place)) }
	Pills(lines, Index.Counts(known))
	local lore = LoreTexts(index, names)
	WhatYouKnow(lines, lore)
	local history = Index.History(index, zone)
	if history then
		Section(lines, "Your history here")
		lines[#lines + 1] = Line("prose", ns.WithName(history))
	end
	Rows(lines, at, "people", "People", PeopleRows(index, known.people))
	Rows(lines, at, "quests", "Quests", QuestRows(index, known.quests))
	Rows(lines, at, "read", "Read and heard", ReadRows(index, known.read))
	Rows(lines, at, "deaths", "Deaths and kills", DeathRows(index, known))
	Chapters(lines, Index.ChaptersOf(index, zone))
	if Index.IsEmpty(known) and #lore == 0 and not history then
		lines[#lines + 1] = Line("help", EMPTY)
	end
	if MoreToLearn(index, names) then
		lines[#lines + 1] = Line("help", "There's more to learn here.")
	end
	return lines
end

local function SubzoneLines(index, at, name)
	local known = Index.Place(index, name)
	local within = known.place.within
	local where = "In " .. PlaceLink(index, within) .. " · first visit " .. Day(known.place.first_visit)
	local lines = { Line("heading", Name(name)), Line("note", where) }
	Pills(lines, Index.Counts(known))
	WhatYouKnow(lines, LoreTexts(index, { name }))
	Rows(lines, at, "people", "People", PeopleRows(index, known.people))
	Rows(lines, at, "quests", "Quests", QuestRows(index, known.quests))
	Rows(lines, at, "deaths", "Deaths and kills", DeathRows(index, known))
	local chapter = Index.ChapterAt(index, known.place.first_visit)
	Chapters(lines, chapter and { chapter } or {})
	return lines
end

local function PlacePage(index, at)
	local name = at.name
	if not Index.IsVisited(index, name) then
		local lines = { Line("heading", Name(name)) }
		local first = #index.places == 0
		lines[#lines + 1] = Line("help", first and EMPTY or "You haven't been here yet.")
		return { lines = lines, zone = name, crumb = Name(name) }
	end
	local zone = Index.ZoneOf(index, name)
	if zone == name then
		return { lines = ZoneLines(index, at, name), zone = name, crumb = Name(name) }
	end
	local crumb = Name(zone) .. "  >  " .. Name(name)
	return { lines = SubzoneLines(index, at, name), zone = zone, crumb = crumb }
end

local QUEST_STATES = { offered = "Offered", accepted = "In progress", done = "Finished" }

local function Times(count)
	if count == 1 then
		return "once"
	end
	return count == 2 and "twice" or (count .. " times")
end

-- What is between you and a person: the quests that they gave you, your talks, and slaps.
local function BetweenYou(index, person)
	local rows = {}
	for _, quest in ipairs(Index.QuestsFrom(index, person.name)) do
		local title = type(quest.number) == "number" and Link("quest", quest.number, Name(quest.title))
		rows[#rows + 1] = Row(ICONS.quest, title or Name(quest.title), QUEST_STATES[quest.status])
	end
	local talks = ns.TalkHistory.Of(person.name)
	if #talks > 0 then
		rows[#rows + 1] = Row(ICONS.person, "Talked " .. Times(#talks), Day(talks[#talks].at))
	end
	if type(person.slapped) == "number" and person.slapped > 0 then
		rows[#rows + 1] = Row(ICONS.person, "Slapped " .. Times(person.slapped), nil)
	end
	return rows
end

local SAID_FROM = {
	quest = function(entry)
		return "From the quest " .. Name(entry.title)
	end,
	gossip = function(entry)
		return type(entry.place) == "string" and ("In " .. Name(entry.place)) or "In passing"
	end,
	book = function(entry)
		return "From " .. Name(entry.title)
	end,
}

local function SaidToYou(lines, index, name)
	local said = Index.SaidBy(index, name)
	if #said == 0 then
		return
	end
	Section(lines, "Said to you")
	for n = #said, math.max(1, #said - 1), -1 do
		local entry = said[n].entry
		lines[#lines + 1] = Line("prose", '"' .. ns.WithName(entry.excerpt) .. '"')
		Note(lines, Link("text", said[n].n, SAID_FROM[entry.kind](entry)))
	end
end

local function PersonPage(index, at)
	local person = Index.Person(index, at.name)
	if not person then
		return { lines = { Line("heading", Name(at.name)) }, crumb = Name(at.name) }
	end
	local lines = { Line("heading", Name(person.name)) }
	local zone = Index.ZoneOf(index, person.place)
	if type(person.place) == "string" then
		local where = PlaceLink(index, person.place)
		Note(lines, zone ~= person.place and (where .. ", " .. PlaceLink(index, zone)) or where)
	end
	local why = ns.Trust.Why(person)
	lines[#lines + 1] = Line("text", Standing(person) .. (why and (" · " .. why) or ""))
	local chapter = Index.ChapterAt(index, person.first_met)
	local met = "Met " .. Day(person.first_met)
	Note(lines, chapter and (met .. ", in " .. ChapterLink(chapter)) or met)
	SaidToYou(lines, index, person.name)
	local between = BetweenYou(index, person)
	if #between > 0 then
		Section(lines, "Between you")
		for _, row in ipairs(between) do
			lines[#lines + 1] = row
		end
	end
	WhatYouKnow(lines, LoreTexts(index, { person.name }))
	local crumb = type(person.place) == "string" and (Name(person.place) .. "  >  " .. Name(person.name))
	return { lines = lines, zone = zone, crumb = crumb or Name(person.name) }
end

local TEXT_TITLES = {
	book = function(entry)
		return Name(entry.title), "Read in "
	end,
	quest = function(entry)
		return Name(entry.title), "From "
	end,
	gossip = function(entry)
		return "Heard from " .. Name(entry.npc), "In "
	end,
}

local function TextPage(index, at)
	local entry = Index.Read(index, at.name)
	if not entry or not TEXT_TITLES[entry.kind] then
		return { lines = { Line("help", EMPTY) } }
	end
	local title, from = TEXT_TITLES[entry.kind](entry)
	local lines = { Line("heading", title) }
	local by = entry.kind == "quest" and type(entry.npc) == "string" and PersonLink(index, entry.npc)
	local place = type(entry.place) == "string" and PlaceLink(index, entry.place)
	local where = by or place
	Note(lines, where and (from .. where .. " · " .. Day(entry.at)) or Day(entry.at))
	if type(entry.excerpt) == "string" then
		lines[#lines + 1] = Line("prose", ns.WithName(entry.excerpt))
	end
	local zone = type(entry.place) == "string" and entry.place or nil
	return { lines = lines, zone = zone, crumb = title }
end

-- The two biggest counts of a zone, for its row on the world page.
local function Brief(index, zone)
	local counts = Index.Counts(Index.Place(index, zone))
	return table.concat({ counts[1], counts[2] }, " · ")
end

-- The zones that you visited, by the continent of their map. The world view with zones
-- to click on the art of the continent is not built: the list on the parchment comes
-- first (the mockup of the atlas, "Hard").
local function WorldPage(index)
	local here = ns.MapPane.ContinentOf(Index.ZoneOf(index, GetRealZoneText()))
	local groups, order = {}, {}
	for _, zone in ipairs(Index.Zones(index)) do
		local continent = ns.MapPane.ContinentOf(zone.name)
		local key = continent and continent.name or "Elsewhere"
		if not groups[key] then
			groups[key] = {}
			order[#order + 1] = key
		end
		table.insert(groups[key], zone)
	end
	local first = here and groups[here.name] and here.name or order[1]
	if not first then
		return { lines = { Line("heading", "Your world"), Line("help", EMPTY) }, crumb = "Your world" }
	end
	local lines = { Line("heading", first) }
	Note(
		lines,
		string.format("You've been to %d %s here.", #groups[first], #groups[first] == 1 and "place" or "places")
	)
	local function Zones(key)
		for _, zone in ipairs(groups[key]) do
			lines[#lines + 1] = Row(ICONS.place, PlaceLink(index, zone.name), Brief(index, zone.name))
		end
	end
	Zones(first)
	for _, key in ipairs(order) do
		if key ~= first then
			Section(lines, key)
			Zones(key)
		end
	end
	local map = here and here.name == first and here.map or nil
	return { lines = lines, map = map, zone = nil, crumb = first, world = true }
end

local PAGES = { place = PlacePage, person = PersonPage, text = TextPage, world = WorldPage }

-- The button on the map that goes up to the world page, from a page of a place.
local function UpButton(page)
	if page.world then
		return nil
	end
	local continent = ns.MapPane.ContinentOf(page.zone)
	local label = continent and continent.name or "Your world"
	return Button("< " .. label, function()
		JournalKnowledge.Show("world", 0)
	end)
end

function JournalKnowledge.Page(journal)
	local index = Index.Of(journal)
	local at = Current()
	local page = (PAGES[at.kind] or PlacePage)(index, at)
	page.side = "map"
	page.footer = JournalKnowledge.USAGE
	page.mapButton = UpButton(page)
	page.pins = ns.AtlasPins.For(index, at.kind, at.name)
	page.buttons = {}
	if #trail > 0 then
		page.buttons[1] = Button("Back", JournalKnowledge.Back)
	end
	return page
end

-- The index of the journal that the book shows, or nil while it loads.
local function ShownIndex()
	local journal = ns.Journal.Current()
	return journal and Index.Of(journal)
end

-- A link names an entry by its place in its list, so the name of the page comes from it.
local function OpenNumbered(kind, list)
	return function(n)
		local index = ShownIndex()
		local entry = index and index[list][n]
		if entry then
			JournalKnowledge.Show(kind, entry.name)
		end
	end
end

ns.JournalLinks.OPENERS.place = OpenNumbered("place", "places")
ns.JournalLinks.OPENERS.person = OpenNumbered("person", "people")
ns.JournalLinks.OPENERS.text = function(n)
	JournalKnowledge.Show("text", n)
end
ns.JournalLinks.OPENERS.world = function()
	JournalKnowledge.Show("world", 0)
end
ns.JournalLinks.OPENERS.all = function(list)
	expanded[PageKey(Current()) .. ":" .. tostring(list)] = true
	ns.JournalFrame.Refresh()
end
