-- The player's own words for a chapter, a tale, or the summary (docs/plans/chapters.md 11).
-- One box holds the shown text. On Save, a box that still starts with the narrator's text
-- adds the paragraphs after it ("keep"). Any other box stands in its place ("replace").
-- Restore brings the narrator's text back. The events of an entry never change.

local _, ns = ...

local JournalEdits = {}
ns.JournalEdits = JournalEdits

local List = ns.JournalRows.List
local Entries = ns.JournalRows.Entries

JournalEdits.TOO_LONG = "Too long to save. Shorten it a little."
JournalEdits.EMPTY = "Write something first."
JournalEdits.BAR = "Take out the || sign and try again."
JournalEdits.ODD_SIGNS = "Some of these characters can't be sent. Take them out and try again."
JournalEdits.ONE_PARAGRAPH = "Your summary is one paragraph. Join them and try again."
JournalEdits.RESTORE = "Restore the narrator's version? Yours is removed."

-- The entries whose edit went out, until the next journal comes.
local saving = {}

local function KeyOf(entry)
	return tostring(entry.kind) .. ":" .. tostring(entry.first)
end

function JournalEdits.Chapter(first)
	return { kind = "chapter", first = first }
end

function JournalEdits.Tale(first)
	return { kind = "tale", first = first }
end

JournalEdits.SUMMARY = { kind = "summary" }

-- The standing edit of an entry in the journal, or nil.
function JournalEdits.Of(journal, entry)
	for _, edit in ipairs(Entries(journal and journal.edits)) do
		local key = type(edit.entry) == "table" and edit.entry or {}
		if key.kind == entry.kind and key.first == entry.first then
			return edit
		end
	end
	return nil
end

function JournalEdits.IsSaving(entry)
	return saving[KeyOf(entry)] == true
end

-- A new journal holds every edit that went out.
function JournalEdits.JournalCame()
	saving = {}
end

-- The paragraphs of the player that show, as the game shows them.
function JournalEdits.Paragraphs(edit)
	local paragraphs = {}
	for _, paragraph in ipairs(List(edit and edit.paragraphs)) do
		if type(paragraph) == "string" then
			paragraphs[#paragraphs + 1] = ns.WithName(paragraph)
		end
	end
	return paragraphs
end

-- The title of the player, or nil for the title of the book.
function JournalEdits.Title(edit)
	return edit and type(edit.title) == "string" and ns.WithName(edit.title) or nil
end

-- What the box shows: the narrator's text, then the player's paragraphs, one a line.
function JournalEdits.BoxText(narrator, edit)
	local lines = {}
	if narrator then
		lines[1] = narrator
	end
	for _, paragraph in ipairs(JournalEdits.Paragraphs(edit)) do
		lines[#lines + 1] = paragraph
	end
	return table.concat(lines, "\n")
end

local function Trimmed(text)
	return (tostring(text or ""):match("^%s*(.-)%s*$"))
end

-- The kind of text and the paragraphs of the player: "keep" while the box starts with the
-- narrator's text, else "replace". A box with only the narrator's text keeps it alone.
local function Split(narrator, paragraphs)
	if not narrator then
		return "keep", paragraphs
	end
	if paragraphs[1] ~= narrator then
		return "replace", paragraphs
	end
	local rest = {}
	for n = 2, #paragraphs do
		rest[#rest + 1] = paragraphs[n]
	end
	return #rest > 0 and "keep" or "narrator", rest
end

-- The line of an edit, or nil and the reason that it can't go. `narrator` is the narrator's
-- text as the box showed it, `bookTitle` the title of the book.
function JournalEdits.Input(entry, typed, heading, narrator, bookTitle)
	local body = ns.StoryText.Body(typed)
	local paragraphs = body == "" and {} or ns.StoryText.Paragraphs(body)
	local text, own = Split(narrator, paragraphs)
	if entry.kind == "summary" then
		text = "replace"
		own = paragraphs
	end
	if #own == 0 and text ~= "narrator" then
		return nil, JournalEdits.EMPTY
	end
	if entry.kind == "summary" and #own > 1 then
		return nil, JournalEdits.ONE_PARAGRAPH
	end
	local title = Trimmed(heading)
	if title == "" or title == bookTitle then
		title = nil
	end
	local joined = (title or "") .. "\n" .. table.concat(own, "\n")
	if joined:find("|", 1, true) then
		return nil, JournalEdits.BAR
	end
	if ns.StoryText.BadSign(joined) then
		return nil, JournalEdits.ODD_SIGNS
	end
	if
		(title and not ns.StoryText.IsTitle(title)) or (#own > 0 and not ns.StoryText.IsBody(table.concat(own, "\n")))
	then
		return nil, JournalEdits.TOO_LONG
	end
	local players = {}
	local function Marked(words)
		local marked, named = ns.TaskNames.Marked(words)
		for _, player in ipairs(named) do
			players[#players + 1] = player
		end
		return marked
	end
	local marked = {}
	for n, paragraph in ipairs(own) do
		marked[n] = Marked(paragraph)
	end
	local markedTitle = title and Marked(title) or nil
	local input = ns.Inputs.EntryEdited(time(), entry, markedTitle, text, marked)
	if not ns.Outbox.Fits(input) then
		return nil, JournalEdits.TOO_LONG
	end
	return input, nil, players
end

-- The line goes out with the players that it names, and a request for the journal. The page
-- says "Saving..." until the journal comes.
local function Send(entry, input, players)
	for _, line in ipairs(ns.TaskNames.Described(players or {})) do
		ns.Outbox.Add(line)
	end
	ns.Outbox.Add(input)
	saving[KeyOf(entry)] = true
	ns.Journal.Request(0)
end

-- Opens the box of an entry. `narrator` is the narrator's text that shows, or nil.
function JournalEdits.Open(entry, label, narrator, edit, bookTitle)
	local heading = nil
	if entry.kind ~= "summary" then
		heading = { text = JournalEdits.Title(edit) or bookTitle or "", letters = ns.StoryText.TITLE_LETTERS }
	end
	ns.JournalFrame.Edit({
		title = label,
		hint = entry.kind == "summary" and "Write who your character has become, in one paragraph." or nil,
		heading = heading,
		text = JournalEdits.BoxText(narrator, edit),
		limit = ns.StoryText.BODY_LETTERS + (narrator and #narrator + 1 or 0),
		problem = function(typed, title)
			return select(2, JournalEdits.Input(entry, typed, title, narrator, bookTitle))
		end,
		save = function(typed, title)
			local input, _, players = JournalEdits.Input(entry, typed, title, narrator, bookTitle)
			if input then
				Send(entry, input, players)
			end
		end,
	})
end

StaticPopupDialogs.TIMEWAYS_ENTRY_RESTORE = {
	text = JournalEdits.RESTORE,
	button1 = "Restore",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, entry)
		Send(entry, ns.Inputs.EntryEdited(time(), entry, nil, "narrator", {}))
		ns.JournalFrame.Refresh()
	end,
}

-- Asks first, because the player's words go away.
function JournalEdits.Restore(entry)
	StaticPopup_Show("TIMEWAYS_ENTRY_RESTORE", nil, nil, entry)
end

-- Your own summary, as the History of your roleplay profile, with your name in place of
-- the mark of the hero. Nil when the narrator's summary stands: that one stays private.
function JournalEdits.SharedSummary()
	local edit = JournalEdits.Of(ns.Journal.Current(), JournalEdits.SUMMARY)
	if not edit or edit.text ~= "replace" then
		return nil
	end
	return JournalEdits.Paragraphs(edit)[1]
end

-- The buttons of a page that the player can edit: Edit, and Restore when edited.
function JournalEdits.Buttons(open, edit)
	local buttons = { ns.JournalRows.Button("Edit", open) }
	if edit then
		buttons[2] = ns.JournalRows.Button("Restore", function()
			JournalEdits.Restore(edit.entry)
		end)
	end
	return buttons
end
