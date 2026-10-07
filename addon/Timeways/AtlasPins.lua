-- The pins of Knowledge on the journal map (GAMEPLAY.md 3.6): the places of the zone, the
-- deaths, and the people of the open place, or what one chapter found. A click on a pin
-- opens its page in Knowledge, and its tooltip gives the counts.

local _, ns = ...

local AtlasPins = {}
ns.AtlasPins = AtlasPins

local Index = ns.KnowledgeIndex
local Of = ns.JournalLinks.Of
local Name = ns.JournalRows.Name
local Day = ns.JournalRows.Day
local Entries = ns.JournalRows.Entries

-- A position from the desktop as a point from 0 to 1, or nil.
local function Point(entry)
	local spot = ns.TaskPins.Spot(entry)
	if not spot then
		return nil
	end
	return { map = spot.map, x = spot.x / 1000, y = spot.y / 1000 }
end

local function Pin(point, fields)
	fields.map, fields.x, fields.y = point.map, point.x, point.y
	return fields
end

-- `open` is the place that the page shows, or nil for none: its pin stands out, and the
-- others fade.
local function PlacePin(index, place, open)
	local point = Point(place)
	if not point then
		return nil
	end
	local counts = Index.Counts(Index.Place(index, place.name))
	return Pin(point, {
		kind = "place",
		link = Of("place", Index.PlaceNumber(index, place.name)),
		title = Name(place.name),
		text = table.concat(counts, " · "),
		selected = place.name == open,
		dim = open ~= nil and place.name ~= open,
	})
end

local function PersonPin(index, person, open)
	local point = Point(person)
	if not point then
		return nil
	end
	return Pin(point, {
		kind = "person",
		link = Of("person", Index.PersonNumber(index, person.name)),
		title = Name(person.name),
		text = ns.Trust.Word(person.trust),
		selected = person.name == open,
	})
end

-- A death has no position of its own yet, so it stands at the spot of its place.
local function DeathPin(index, deed)
	local place = deed.kind == "died" and Index.PlaceEntry(index, deed.place)
	local point = place and Point(place)
	if not point then
		return nil
	end
	local what = type(deed.killer) == "string" and ("Killed by " .. Name(deed.killer)) or "Died"
	return Pin(point, { kind = "death", title = what, text = Day(deed.at) })
end

local function Add(pins, pin)
	if pin then
		pins[#pins + 1] = pin
	end
end

local function PlacesOf(pins, index, zone, open)
	for _, place in ipairs(Index.Subzones(index, zone)) do
		Add(pins, PlacePin(index, place, open))
	end
end

local function PeopleOf(pins, index, place, open)
	for _, person in ipairs(Index.Place(index, place).people) do
		Add(pins, PersonPin(index, person, open))
	end
end

-- Each pin is { kind = "place", "person", or "death", map, x, y, title, text, link,
-- selected, dim }, with x and y from 0 to 1.
function AtlasPins.For(index, kind, name)
	local pins = {}
	if kind == "place" and Index.IsVisited(index, name) then
		local zone = Index.ZoneOf(index, name)
		local open = zone ~= name and name or nil
		PlacesOf(pins, index, zone, open)
		if open then
			PeopleOf(pins, index, name, nil)
			return pins
		end
		for _, deed in ipairs(Index.Place(index, zone).deaths) do
			Add(pins, DeathPin(index, deed))
		end
		return pins
	end
	local person = kind == "person" and Index.Person(index, name)
	if person and type(person.place) == "string" then
		PlacesOf(pins, index, Index.ZoneOf(index, person.place), person.place)
		PeopleOf(pins, index, person.place, name)
	end
	return pins
end

-- The places inside a zone and the people that a chapter found, and its deaths: the map
-- of a chapter in the Chronicle.
function AtlasPins.ForChapter(index, chapter)
	local pins = {}
	for _, place in ipairs(index.places) do
		if type(place.within) == "string" and Index.InChapter(chapter, place.first_visit) then
			Add(pins, PlacePin(index, place, nil))
		end
	end
	for _, person in ipairs(index.people) do
		if Index.InChapter(chapter, person.first_met) then
			Add(pins, PersonPin(index, person, nil))
		end
	end
	for _, deed in ipairs(Entries(chapter.deeds)) do
		Add(pins, DeathPin(index, deed))
	end
	return pins
end
