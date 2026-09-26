-- The pages of the journal: the chronicle, places, people, and deeds (GAMEPLAY.md 3.6).
-- The desktop sends them, because the world lives there and never in the saved variables
-- (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

Journal.SECTIONS = { "chapters", "places", "people", "deeds" }
Journal.TITLES = { chapters = "Chronicle", places = "Places", people = "People", deeds = "Deeds" }

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
		collecting = { chapters = {}, places = {}, people = {}, deeds = {}, next = 0 }
	end
	if not collecting or page ~= collecting.next then
		return
	end
	for _, section in ipairs(Journal.SECTIONS) do
		for _, entry in ipairs(Entries(value[section])) do
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
	local subzones = {}
	for _, place in ipairs(places) do
		if place.within ~= nil then
			subzones[place.within] = subzones[place.within] or {}
			table.insert(subzones[place.within], place)
		end
	end
	local lines = {}
	for _, zone in ipairs(places) do
		if zone.within == nil then
			lines[#lines + 1] = Line("heading", Name(zone.name))
			lines[#lines + 1] = Line("text", "First visited on " .. Day(zone.first_visit) .. ".")
			for _, subzone in ipairs(subzones[zone.name] or {}) do
				lines[#lines + 1] = Line("entry", Name(subzone.name))
			end
		end
	end
	return lines
end

local function TrustWords(trust)
	if trust >= 50 then
		return "Trusts you."
	elseif trust >= 10 then
		return "Likes you."
	elseif trust > -10 then
		return "Thinks little of you."
	elseif trust > -50 then
		return "Wary of you."
	end
	return "Distrusts you."
end

-- "Slapped 2 times. Wary of you." It stays empty until something changed the trust.
local function Standing(person)
	local parts = {}
	if type(person.slapped) == "number" then
		local times = person.slapped == 1 and "time" or "times"
		parts[#parts + 1] = string.format("Slapped %d %s.", person.slapped, times)
	end
	if type(person.trust) == "number" then
		parts[#parts + 1] = TrustWords(person.trust)
	end
	return table.concat(parts, " ")
end

local function People(people)
	local lines = {}
	for _, person in ipairs(people) do
		lines[#lines + 1] = Line("entry", Name(person.name))
		local where = person.place and ("in " .. Name(person.place) .. ", ") or ""
		lines[#lines + 1] = Line("text", "Met " .. where .. "on " .. Day(person.first_met) .. ".")
		local standing = Standing(person)
		if standing ~= "" then
			lines[#lines + 1] = Line("text", standing)
		end
	end
	return lines
end

local function Where(deed)
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
	elseif deed.kind == "died" then
		return deed.killer and ("Fell to " .. Name(deed.killer)) or "Died"
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

-- A chapter is the list of what was new in one session (GAMEPLAY.md 3.3). Once the bard
-- wrote it, its saga comes first.
local function Chapters(chapters)
	local lines = {}
	for _, chapter in ipairs(chapters) do
		local number = type(chapter.number) == "number" and chapter.number or "?"
		lines[#lines + 1] = Line("heading", "Chapter " .. number)
		lines[#lines + 1] = Line("text", Day(chapter.began) .. ".")
		if type(chapter.prose) == "string" then
			lines[#lines + 1] = Line("prose", ns.Plain(chapter.prose))
		end
		if #List(chapter.zones) > 0 then
			lines[#lines + 1] = Line("entry", "Traveled to " .. Together(List(chapter.zones)) .. ".")
		end
		if #List(chapter.people) > 0 then
			lines[#lines + 1] = Line("entry", "Met " .. Together(List(chapter.people)) .. ".")
		end
		for _, deed in ipairs(Entries(chapter.deeds)) do
			local title = DeedTitle(deed)
			if title then
				lines[#lines + 1] = Line("entry", title .. ".")
			end
		end
		if type(chapter.left_out) == "number" and chapter.left_out > 0 then
			lines[#lines + 1] = Line("text", string.format("And %d more.", chapter.left_out))
		end
	end
	return lines
end

local BUILDERS = { chapters = Chapters, places = Places, people = People, deeds = Deeds }

local EMPTY = {
	chapters = "No chapter is written yet.",
	places = "You have not traveled yet.",
	people = "You have met no one yet.",
	deeds = "Your deeds are not written yet.",
}

-- Each line is { style = "heading" | "prose" | "entry" | "text" | "note", text = ... }.
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
