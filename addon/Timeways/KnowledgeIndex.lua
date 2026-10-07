-- What the character knows of each place and person, joined from the lists of the journal
-- by name (GAMEPLAY.md 3.6): the people of a place, its quests, what you read there, the
-- deaths and the kills, its chapters, and its lore. Pure functions of a journal, so the
-- tests and the fuzzer read them with no frame.

local _, ns = ...

local KnowledgeIndex = {}
ns.KnowledgeIndex = KnowledgeIndex

local Entries = ns.JournalRows.Entries
local List = ns.JournalRows.List

local QUEST_DEEDS = { quest_done = true, game_quest_done = true, class_quest_done = true }
local READ_KINDS = { book = true, quest = true, gossip = true }

local function IsName(value)
	return type(value) == "string" and value ~= ""
end

local function IsTime(value)
	return type(value) == "number"
end

-- The index of the newest journal, built once for each journal.
local built, builtFor

local function Named(list)
	local by = {}
	for n, entry in ipairs(list) do
		if IsName(entry.name) and not by[entry.name] then
			by[entry.name] = { entry = entry, n = n }
		end
	end
	return by
end

local function ByField(list, field)
	local by = {}
	for _, entry in ipairs(list) do
		if IsName(entry[field]) then
			by[entry[field]] = entry
		end
	end
	return by
end

local function Build(journal)
	local index = {
		journal = journal,
		places = Entries(journal.places),
		people = Entries(journal.people),
		deeds = Entries(journal.deeds),
		learned = Entries(journal.learned),
		quests = Entries(journal.quests),
		chapters = Entries(journal.chapters),
	}
	index.readNumber = {}
	for n, entry in ipairs(index.learned) do
		index.readNumber[entry] = n
	end
	index.place = Named(index.places)
	index.person = Named(index.people)
	index.lore = ByField(Entries(journal.lore), "about")
	index.history = ByField(Entries(journal.histories), "zone")
	return index
end

function KnowledgeIndex.Of(journal)
	if builtFor ~= journal then
		built, builtFor = Build(journal), journal
	end
	return built
end

-- The zone of a place: the zone around a subzone, or the place itself.
function KnowledgeIndex.ZoneOf(index, name)
	local known = IsName(name) and index.place[name]
	if known and IsName(known.entry.within) then
		return known.entry.within
	end
	return name
end

function KnowledgeIndex.IsVisited(index, name)
	return IsName(name) and index.place[name] ~= nil
end

function KnowledgeIndex.IsZone(index, name)
	return KnowledgeIndex.IsVisited(index, name) and not IsName(index.place[name].entry.within)
end

-- True when `place` is the place itself or, for a zone, one of its subzones.
local function IsIn(index, place, name, isZone)
	if not IsName(place) then
		return false
	end
	if isZone then
		return KnowledgeIndex.ZoneOf(index, place) == name
	end
	return place == name
end

-- The zones that you visited, in the order of the first visit.
function KnowledgeIndex.Zones(index)
	local zones = {}
	for _, place in ipairs(index.places) do
		if IsName(place.name) and not IsName(place.within) then
			zones[#zones + 1] = place
		end
	end
	return zones
end

function KnowledgeIndex.Subzones(index, zone)
	local found = {}
	for _, place in ipairs(index.places) do
		if place.within == zone and IsName(place.name) then
			found[#found + 1] = place
		end
	end
	return found
end

-- The moment `at` is in the chapter: after its start, and before its end unless it is
-- still open.
function KnowledgeIndex.InChapter(chapter, at)
	local open = chapter.state == "open" or not IsTime(chapter.ended)
	local after = IsTime(at) and IsTime(chapter.began) and at >= chapter.began
	return after and (open or at <= chapter.ended)
end

-- The chapter that held the moment `at`: the newest one that began at or before it.
function KnowledgeIndex.ChapterAt(index, at)
	if not IsTime(at) then
		return nil
	end
	local found
	for _, chapter in ipairs(index.chapters) do
		if IsTime(chapter.began) and chapter.began <= at then
			found = chapter
		end
	end
	return found
end

-- What you know of one place, a zone with its subzones or a subzone alone. Each list
-- holds entries of the journal, oldest first.
function KnowledgeIndex.Place(index, name)
	local isZone = KnowledgeIndex.IsZone(index, name)
	local known = {
		name = name,
		place = KnowledgeIndex.PlaceEntry(index, name),
		people = {},
		quests = {},
		read = {},
		deaths = {},
		kills = {},
	}
	for _, person in ipairs(index.people) do
		if IsName(person.name) and IsIn(index, person.place, name, isZone) then
			known.people[#known.people + 1] = person
		end
	end
	for _, deed in ipairs(index.deeds) do
		if IsIn(index, deed.place, name, isZone) then
			if QUEST_DEEDS[deed.kind] and IsName(deed.title) then
				known.quests[#known.quests + 1] = deed
			elseif deed.kind == "died" then
				known.deaths[#known.deaths + 1] = deed
			elseif deed.kind == "defeated" and deed.times == 1 and IsName(deed.foe) then
				known.kills[#known.kills + 1] = deed
			end
		end
	end
	-- The game names the zone of a text, never its subzone.
	for _, entry in ipairs(index.learned) do
		if isZone and READ_KINDS[entry.kind] and entry.place == name then
			known.read[#known.read + 1] = entry
		end
	end
	return known
end

-- The chapters whose places hold this one, oldest first.
function KnowledgeIndex.ChaptersOf(index, name)
	local zone = KnowledgeIndex.ZoneOf(index, name)
	local found = {}
	for _, chapter in ipairs(index.chapters) do
		for _, place in ipairs(List(chapter.zones)) do
			if place == zone then
				found[#found + 1] = chapter
				break
			end
		end
	end
	return found
end

-- The lore of a place or a person from the desktop: { text, more }, or nil.
function KnowledgeIndex.Lore(index, name)
	return IsName(name) and index.lore[name] or nil
end

-- "Your history here" of a zone, or nil.
function KnowledgeIndex.History(index, zone)
	local history = IsName(zone) and index.history[zone]
	return history and IsName(history.text) and history.text or nil
end

-- The entry of a place in the journal, or nil.
function KnowledgeIndex.PlaceEntry(index, name)
	return IsName(name) and index.place[name] and index.place[name].entry or nil
end

function KnowledgeIndex.Person(index, name)
	return IsName(name) and index.person[name] and index.person[name].entry or nil
end

-- The place in its list of the journal, for a link.
function KnowledgeIndex.PlaceNumber(index, name)
	return IsName(name) and index.place[name] and index.place[name].n or nil
end

function KnowledgeIndex.PersonNumber(index, name)
	return IsName(name) and index.person[name] and index.person[name].n or nil
end

-- What a person said to you in the game: the quest texts and the gossip of their own.
function KnowledgeIndex.SaidBy(index, name)
	local said = {}
	for n, entry in ipairs(index.learned) do
		if entry.npc == name and READ_KINDS[entry.kind] and IsName(entry.excerpt) then
			said[#said + 1] = { entry = entry, n = n }
		end
	end
	return said
end

-- The side quests that a person gave you.
function KnowledgeIndex.QuestsFrom(index, name)
	local given = {}
	for _, quest in ipairs(index.quests) do
		if quest.giver == name and IsName(quest.title) then
			given[#given + 1] = quest
		end
	end
	return given
end

-- A read text by its place in the list of the journal.
function KnowledgeIndex.Read(index, n)
	return index.learned[n]
end

function KnowledgeIndex.ReadNumber(index, entry)
	return index.readNumber[entry]
end

-- The read text of the quest with this title, as a number of the list, or nil.
function KnowledgeIndex.QuestText(index, title)
	for n, entry in ipairs(index.learned) do
		if entry.kind == "quest" and entry.title == title then
			return n
		end
	end
	return nil
end

-- "9 people", "11 quests", "2 read", "2 deaths": only the counts that are not zero. A
-- count, never a total: nobody knows how many there are to find.
function KnowledgeIndex.Counts(known)
	local counts = {}
	local function Count(n, one, many)
		if n > 0 then
			counts[#counts + 1] = n == 1 and ("1 " .. one) or (n .. " " .. many)
		end
	end
	Count(#known.people, "person", "people")
	Count(#known.quests, "quest", "quests")
	Count(#known.read, "read", "read")
	Count(#known.deaths, "death", "deaths")
	return counts
end

function KnowledgeIndex.IsEmpty(known)
	return #KnowledgeIndex.Counts(known) == 0 and #known.kills == 0
end
