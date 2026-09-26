-- The pages of the journal: places, people, and deeds (GAMEPLAY.md 3.3). The desktop
-- sends them, because the world lives there and never in the saved variables (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

Journal.SECTIONS = { "places", "people", "deeds" }
Journal.TITLES = { places = "Places", people = "People", deeds = "Deeds" }

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

function Journal.Request(page)
	ns.Outbox.Add(ns.Inputs.JournalAsked(page))
	ns.Outbox.Flush()
end

-- A page out of order belongs to an older request, and is dropped.
function Journal.Receive(value)
	local page, count = value.page, value.pages
	if type(page) ~= "number" or type(count) ~= "number" or count > MAX_PAGES then
		return
	end
	if page == 0 then
		collecting = { places = {}, people = {}, deeds = {}, next = 0 }
	end
	if not collecting or page ~= collecting.next then
		return
	end
	for _, section in ipairs(Journal.SECTIONS) do
		for _, entry in ipairs(List(value[section])) do
			table.insert(collecting[section], entry)
		end
	end
	collecting.next = page + 1
	if collecting.next < count then
		Journal.Request(collecting.next)
		return
	end
	pages, collecting = collecting, nil
	ns.JournalFrame.Refresh()
end

local function Name(value)
	return type(value) == "string" and ns.Plain(value) or "?"
end

local function Day(at)
	return type(at) == "number" and date("%d %b %Y", at) or "an unknown day"
end

local function Line(style, text)
	return { style = style, text = text }
end

local function Places(places)
	local lines = {}
	for _, zone in ipairs(places) do
		if zone.within == nil then
			lines[#lines + 1] = Line("heading", Name(zone.name))
			lines[#lines + 1] = Line("text", "First visited on " .. Day(zone.first_visit) .. ".")
			for _, place in ipairs(places) do
				if place.within ~= nil and place.within == zone.name then
					lines[#lines + 1] = Line("entry", Name(place.name))
				end
			end
		end
	end
	return lines
end

local function People(people)
	local lines = {}
	for _, person in ipairs(people) do
		lines[#lines + 1] = Line("entry", Name(person.name))
		local where = person.place and ("in " .. Name(person.place) .. ", ") or ""
		lines[#lines + 1] = Line("text", "Met " .. where .. "on " .. Day(person.first_met) .. ".")
	end
	return lines
end

local function Place(deed)
	return deed.place and (Name(deed.place) .. ", ") or ""
end

-- The first kill is the true kill. Each later kill is an echo after a reset (5.13).
local function DeedTitle(deed)
	if deed.kind == "level" and type(deed.to) == "number" then
		local what = deed.from and "Reached level %d" or "Began this journal at level %d"
		return string.format(what, deed.to)
	elseif deed.kind == "defeated" and type(deed.times) == "number" then
		if deed.times == 1 then
			return "Defeated " .. Name(deed.foe)
		end
		return string.format("Defeated %s again (%d times)", Name(deed.foe), deed.times)
	end
end

local function Deeds(deeds)
	local lines = {}
	for _, deed in ipairs(deeds) do
		local title = DeedTitle(deed)
		if title then
			lines[#lines + 1] = Line("entry", title)
			lines[#lines + 1] = Line("text", Place(deed) .. Day(deed.at) .. ".")
		end
	end
	return lines
end

local BUILDERS = { places = Places, people = People, deeds = Deeds }

local EMPTY = {
	places = "You have not traveled yet.",
	people = "You have met no one yet.",
	deeds = "Your deeds are not written yet.",
}

-- Each line is { style = "heading" | "entry" | "text" | "note", text = ... }.
function Journal.Lines(section)
	if not pages then
		return { Line("note", "The pages fill with ink...") }
	end
	local lines = BUILDERS[section](List(pages[section]))
	if #lines == 0 then
		return { Line("note", EMPTY[section]) }
	end
	return lines
end
