-- The Hero page of the journal (GAMEPLAY.md 3.7 and 3.7.1). Two sub-tabs: Your Story, the six
-- answers as cards with your notes beside them, and Roleplay Profile, the fields that roleplay
-- addons share with a preview of what other players see.

local _, ns = ...

local JournalHero = {}
ns.JournalHero = JournalHero

local Entries = ns.JournalRows.Entries
local Name = ns.JournalRows.Name
local Line = ns.JournalRows.Line
local SAVING = ns.JournalRows.SAVING

JournalHero.USAGE = "Shapes your chapters and what NPCs say to you."

-- The fields of the profile that a card shows on its own row: their texts run long.
local WIDE = { currently = true, appearance = true }

-- The MSP names of the two answers that the profile shares too.
local SHARED_AS = { origin = "Birthplace", background = "History" }

-- "03 Oct", as the notes show their day.
local function Day(at)
	return type(at) == "number" and date("%d %b", at) or "?"
end

local function OpenTab(key)
	return function()
		ns.Journal.Select("hero", key)
		ns.JournalFrame.Refresh()
	end
end

local function Tabs(open)
	return {
		tabs = {
			{ key = "story", label = "Your Story", run = OpenTab("story") },
			{ key = "profile", label = "Roleplay Profile", run = OpenTab("profile") },
		},
		tab = open,
		rows = {},
	}
end

-- The texts of the sheet by field, with the edits that the desktop did not confirm yet.
function JournalHero.Texts(hero, unsaved)
	local texts = {}
	for _, field in ipairs(Entries(hero and hero.sheet)) do
		if type(field.field) == "string" and type(field.text) == "string" then
			texts[field.field] = field.text
		end
	end
	for field, text in pairs(unsaved.fields) do
		texts[field] = text ~= "" and text or nil
	end
	return texts
end

local function AnsweredCount(texts)
	local count = 0
	for _, field in ipairs(ns.Hero.FIELDS) do
		if texts[field] then
			count = count + 1
		end
	end
	return count
end

-- The field whose card is open for writing, or nil.
local editing

local function Refresh()
	ns.JournalFrame.Refresh()
end

-- The first question after this one with no answer, and else the first one before it.
function JournalHero.NextEmpty(field)
	local journal = ns.Journal.Current()
	local texts = JournalHero.Texts(journal and journal.hero, ns.Hero.Unsaved())
	local fields, at = ns.Hero.FIELDS, 0
	for n, each in ipairs(fields) do
		at = each == field and n or at
	end
	for step = 1, #fields - 1 do
		local next = fields[(at + step - 1) % #fields + 1]
		if not texts[next] then
			return next
		end
	end
end

local function IsQuestion(field)
	for _, each in ipairs(ns.Hero.FIELDS) do
		if each == field then
			return true
		end
	end
	return false
end

-- After Save, the next question with no answer opens for its answer.
local function Saved(field)
	editing = IsQuestion(field) and JournalHero.NextEmpty(field) or nil
	Refresh()
end

-- The writing part of an open card: the box, its limits, Save, and Cancel.
local function Editing(field, text)
	local limit = ns.Hero.LIMITS[field]
	return {
		key = field,
		text = text or "",
		limit = limit.letters,
		bytes = limit.bytes,
		problem = function(typed)
			return ns.Hero.Problem(field, typed)
		end,
		save = function(typed)
			ns.Hero.Save(field, text, typed)
			Saved(field)
		end,
		cancel = function()
			editing = nil
			Refresh()
		end,
	}
end

-- A card of a field: its answer and Edit, or its question and the empty label, such as
-- Answer. Another roleplay addon owns the profile, so its fields have no button then.
local function Card(field, texts, unsaved, empty)
	local text = texts[field]
	local card = {
		kind = "card",
		key = field,
		label = ns.Hero.LABELS[field],
		text = text and ns.Plain(text) or ns.Hero.HINTS[field],
		empty = text == nil,
		wide = WIDE[field],
		note = unsaved.fields[field] and SAVING or nil,
		writable = true,
	}
	card.action = {
		label = text and "Edit" or empty,
		run = function()
			editing = field
			Refresh()
		end,
	}
	if editing == field then
		card.text, card.empty, card.wide = ns.Hero.HINTS[field], true, true
		card.editing = Editing(field, text)
	end
	return card
end

-- Your Story -----------------------------------------------------------------------------------

local function SavedEntry(entry)
	local remove = {
		label = "Remove",
		run = function()
			ns.Hero.Remove(entry)
		end,
	}
	local parts = {}
	if type(entry.npc) == "string" then
		parts[#parts + 1] = "About " .. Name(entry.npc)
	end
	if type(entry.place) == "string" then
		parts[#parts + 1] = Name(entry.place)
	end
	parts[#parts + 1] = Day(entry.at)
	return {
		Line("entry", ns.Plain(tostring(entry.text)), remove),
		Line("text", table.concat(parts, " · ")),
	}
end

-- The player's own notes, less the ones that wait for removal, then the new ones.
local function NoteLines(hero, unsaved)
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

local function Notes(hero, unsaved)
	local add = { label = "Add a note", run = ns.Hero.Write }
	local lines = { Line("heading", "Your Notes", add) }
	local notes = NoteLines(hero, unsaved)
	if #notes == 0 then
		lines[#lines + 1] = Line("help", "A grudge, a promise, a secret.")
	end
	for _, line in ipairs(notes) do
		lines[#lines + 1] = line
	end
	return lines
end

local function StoryPage(hero, texts, unsaved)
	local cards = Tabs("story")
	cards.rows[1] = { kind = "line", text = JournalHero.USAGE, ink = "faded" }
	local shared = ns.MspProfile.IsSharing()
	local summary = ns.JournalEdits.SharedSummary()
	for _, field in ipairs(ns.Hero.FIELDS) do
		local card = Card(field, texts, unsaved, "Answer")
		local history = field == "background" and summary
		card.tag = shared and SHARED_AS[field] and not history and "Shared" or nil
		cards.rows[#cards.rows + 1] = card
	end
	return {
		cards = cards,
		lines = Notes(hero, unsaved),
		buttons = {},
		footer = string.format("%d of %d answered", AnsweredCount(texts), #ns.Hero.FIELDS),
		side = "cards",
	}
end

-- Roleplay Profile -----------------------------------------------------------------------------

local SHARE_LINES = {
	on = "Players with roleplay addons like Total RP 3 see this.",
	off = "Only you see this.",
}

local function ShareRow()
	local owner = ns.MspProfile.Owner()
	if owner then
		-- The owner can be "your roleplay addon", which starts in lower case.
		local subject = owner:gsub("^%l", string.upper)
		return { kind = "line", text = subject .. " shares your profile. Change it there." }
	end
	local sharing = ns.MspProfile.IsSharing()
	return {
		kind = "line",
		text = sharing and SHARE_LINES.on or SHARE_LINES.off,
		action = {
			label = sharing and "Stop sharing" or "Share",
			run = function()
				ns.MspProfile.SetSharing(not sharing)
			end,
		},
	}
end

-- Origin and Background go out too, under the names that roleplay addons use.
local function AlsoShared(texts)
	local rows = {
		{
			kind = "heading",
			text = "Also shared, from Your Story",
			action = { label = "Edit in Your Story", run = OpenTab("story") },
		},
	}
	local summary = ns.JournalEdits.SharedSummary()
	for _, field in ipairs({ "origin", "background" }) do
		local text = texts[field]
		local row = {
			kind = "linked",
			label = ns.Hero.LABELS[field],
			text = text and ns.Plain(text) or "Not answered yet",
			empty = text == nil,
			note = "as " .. SHARED_AS[field],
		}
		-- Your own summary of the Chronicle goes out as the History in place of Background.
		if field == "background" and summary then
			row.label, row.text, row.empty = "Your summary", summary, false
		end
		rows[#rows + 1] = row
	end
	return rows
end

local function Append(list, items)
	for _, item in ipairs(items) do
		list[#list + 1] = item
	end
end

-- "Level 9 Undead Paladin (Player)", as the tooltip of the game shows it.
local function TooltipLevel()
	local race = UnitRace("player") or ""
	local class = UnitClass("player") or ""
	return string.format("Level %d %s %s (Player)", UnitLevel("player") or 0, race, class)
end

local function Shown(texts, field)
	if field == "background" and ns.JournalEdits.SharedSummary() then
		return ns.JournalEdits.SharedSummary()
	end
	return texts[field] and ns.Plain(texts[field])
end

-- What other players see: the tooltip and the profile of a roleplay addon. It reads the
-- sheet and the unsaved edits, so it shows before you share.
local function Preview(texts)
	local name = UnitName("player") or "?"
	local title = Shown(texts, "title")
	local lines = {
		Line("heading", "What Others See"),
		Line("section", "Tooltip"),
		Line("entry", title and (name .. ", " .. title) or name),
		Line("text", TooltipLevel()),
	}
	if texts.currently then
		lines[#lines + 1] = Line("text", "Currently: " .. Shown(texts, "currently"))
	end
	lines[#lines + 1] = Line("section", "Profile")
	lines[#lines + 1] = Line("entry", name)
	if title then
		lines[#lines + 1] = Line("text", title)
	end
	for _, part in ipairs({ { "age", "Age" }, { "origin", "Birthplace" }, { "motto", "Motto" } }) do
		if texts[part[1]] then
			lines[#lines + 1] = Line("text", part[2] .. ": " .. Shown(texts, part[1]))
		end
	end
	for _, part in ipairs({ { "appearance", "Description" }, { "background", "History" } }) do
		if Shown(texts, part[1]) then
			lines[#lines + 1] = Line("section", part[2])
			local cut = Line("prose", Shown(texts, part[1]))
			cut.maxLines = 3
			lines[#lines + 1] = cut
		end
	end
	return lines
end

local function ProfilePage(texts, unsaved)
	local cards = Tabs("profile")
	local owner = ns.MspProfile.Owner()
	cards.rows[1] = ShareRow()
	for _, field in ipairs(ns.Hero.PROFILE) do
		local card = Card(field, texts, unsaved, "Add")
		card.action = not owner and card.action or nil
		cards.rows[#cards.rows + 1] = card
	end
	local lines = { Line("help", "Other players see the profile of " .. tostring(owner) .. ".") }
	if not owner then
		Append(cards.rows, AlsoShared(texts))
		lines = Preview(texts)
	end
	return { cards = cards, lines = lines, buttons = {}, footer = "", side = "cards" }
end

-- The open sub-tab is the selected key of the Hero page.
function JournalHero.Page(journal)
	local hero, unsaved = journal.hero, ns.Hero.Unsaved()
	local texts = JournalHero.Texts(hero, unsaved)
	if ns.Journal.Selected("hero") == "profile" then
		return ProfilePage(texts, unsaved)
	end
	return StoryPage(hero, texts, unsaved)
end
