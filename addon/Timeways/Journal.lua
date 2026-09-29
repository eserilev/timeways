-- The pages of the journal: the hero, the chronicle, places, people, deeds, what you
-- learned, and your side quests (GAMEPLAY.md 3.1.1, 3.4, 3.6, and 3.7).
-- The desktop sends them, because the world lives there and never in the saved variables
-- (5.10).

local _, ns = ...

local Journal = {}
ns.Journal = Journal

Journal.SECTIONS = { "hero", "chapters", "places", "people", "deeds", "learned", "quests" }
Journal.TITLES = {
	hero = "Hero",
	chapters = "Chronicle",
	places = "Places",
	people = "People",
	deeds = "Deeds",
	learned = "Learned",
	quests = "Quests",
}

-- The lists that come in pages. The sheet of the hero comes on the first page only.
local LISTS = { "chapters", "places", "people", "deeds", "learned", "quests" }

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

local function Started(value)
	local hero = type(value.hero) == "table" and value.hero or {}
	local journal = { chapters = {}, places = {}, people = {}, deeds = {}, learned = {}, quests = {} }
	journal.next = 0
	journal.hero = { sheet = Entries(hero.sheet), entries = {} }
	return journal
end

local function Append(journal, value)
	for _, section in ipairs(LISTS) do
		for _, entry in ipairs(Entries(value[section])) do
			table.insert(journal[section], entry)
		end
	end
	local hero = type(value.hero) == "table" and value.hero or {}
	for _, entry in ipairs(Entries(hero.entries)) do
		table.insert(journal.hero.entries, entry)
	end
end

-- The journal of the first page alone, for the self-test.
function Journal.FirstPage(value)
	local journal = Started(value)
	Append(journal, value)
	return journal
end

-- A page out of order belongs to an older request, and is dropped.
function Journal.Receive(value)
	local page, count = value.page, value.pages
	if type(page) ~= "number" or type(count) ~= "number" or count > MAX_PAGES then
		return
	end
	if page == 0 then
		collecting = Started(value)
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
	elseif deed.kind == "titled" then
		return "Earned the title " .. Name(deed.title)
	elseif deed.kind == "quest_done" then
		return "Finished the task " .. Name(deed.title)
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
		for _, footnote in ipairs(List(chapter.footnotes)) do
			if type(footnote) == "string" then
				lines[#lines + 1] = Line("note", "* " .. ns.Plain(footnote))
			end
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

local function StepText(step)
	if step.goal == "visit" then
		return "Visit " .. Name(step.place) .. "."
	end
	if step.goal == "meet" then
		return "Speak with " .. Name(step.npc) .. "."
	end
	return "?"
end

-- An offer carries its two buttons, because the chat line of the offer scrolls away.
local function QuestStatus(quest)
	if quest.status == "offered" then
		return Line("note", "An offer. Do you take it?", { label = "Decline", run = ns.Quest.Decline })
	end
	if quest.status == "done" then
		return Line("note", "Done on " .. Day(quest.done_at) .. ".")
	end
	return Line("note", "In progress.")
end

local function Quests(quests)
	local lines = {}
	for _, quest in ipairs(quests) do
		local accept = quest.status == "offered" and { label = "Accept", run = ns.Quest.Accept } or nil
		lines[#lines + 1] = Line("heading", Name(quest.title), accept)
		lines[#lines + 1] = Line("text", "From " .. Name(quest.giver) .. ", on " .. Day(quest.offered_at) .. ".")
		if type(quest.text) == "string" then
			lines[#lines + 1] = Line("prose", ns.Plain(quest.text))
		end
		local done = type(quest.steps_done) == "number" and quest.steps_done or 0
		for n, step in ipairs(Entries(quest.steps)) do
			local mark = n <= done and "(done) " or ""
			lines[#lines + 1] = Line("entry", mark .. StepText(step))
		end
		lines[#lines + 1] = QuestStatus(quest)
	end
	return lines
end

-- Who the hero is (3.7): each field of the sheet with its button, then the player's own lore.
local function Hero(hero)
	local lines = { Line("heading", "Who you are") }
	local texts = {}
	for _, field in ipairs(Entries(hero and hero.sheet)) do
		texts[field.field] = field.text
	end
	for _, field in ipairs(ns.Hero.FIELDS) do
		local text = texts[field]
		local edit = {
			label = "Edit",
			run = function()
				ns.Hero.Edit(field, text)
			end,
		}
		lines[#lines + 1] = Line("entry", ns.Hero.LABELS[field], edit)
		if type(text) == "string" then
			lines[#lines + 1] = Line("text", ns.Plain(text))
		else
			lines[#lines + 1] = Line("note", ns.Hero.HINTS[field])
		end
	end
	local add = { label = "Add", run = ns.Hero.Write }
	lines[#lines + 1] = Line("heading", "Your own lore", add)
	local entries = Entries(hero and hero.entries)
	if #entries == 0 then
		lines[#lines + 1] = Line("note", "Nothing yet. Add a memory, a rumor, or a vow.")
	end
	for _, entry in ipairs(entries) do
		local remove = {
			label = "Remove",
			run = function()
				ns.Hero.Remove(entry)
			end,
		}
		lines[#lines + 1] = Line("entry", ns.Plain(tostring(entry.text)), remove)
		local about = type(entry.npc) == "string" and ("About " .. Name(entry.npc) .. ". ") or ""
		local place = type(entry.place) == "string" and (Name(entry.place) .. ", ") or ""
		lines[#lines + 1] = Line("text", about .. place .. Day(entry.at) .. ".")
	end
	return lines
end

local BUILDERS = {
	hero = Hero,
	chapters = Chapters,
	places = Places,
	people = People,
	deeds = Deeds,
	learned = Learned,
	quests = Quests,
}

local EMPTY = {
	chapters = "No chapter is written yet.",
	places = "You have not traveled yet.",
	people = "You have met no one yet.",
	deeds = "Your deeds are not written yet.",
	learned = "You have learned nothing yet. Read a book, or listen to the people you meet.",
	quests = "No task yet. Target someone, and type /quest.",
}

-- Each line is { style = "heading" | "prose" | "entry" | "text" | "note", text = ... }.
function Journal.Render(journal, section)
	local builder = BUILDERS[section]
	local lines = section == "hero" and builder(journal.hero) or builder(List(journal[section]))
	if #lines == 0 then
		return { Line("note", EMPTY[section]) }
	end
	return lines
end

function Journal.Lines(section)
	if not pages then
		return { Line("note", "The pages fill with ink...") }
	end
	return Journal.Render(pages, section)
end
