-- The Stories page of the journal (GAMEPLAY.md 4.8): the stories that wait for your answer,
-- the ones that you accepted, and your drafts on the left, and the open one as a page of a
-- book on the right.

local _, ns = ...

local JournalStories = {}
ns.JournalStories = JournalStories

local Entries = ns.JournalRows.Entries
local Line = ns.JournalRows.Line
local Group = ns.JournalRows.Group
local Item = ns.JournalRows.Item
local Button = ns.JournalRows.Button
local OpenIndex = ns.JournalRows.OpenIndex
local Keys = ns.JournalRows.Keys

JournalStories.USAGE = "Stories other players told about you."
JournalStories.FULL = "Answer some stories to get new ones."

local function Short(name)
	return ns.TaskPeople.Short(name)
end

-- "Today", or the day of the month and the month: "3 Oct".
function JournalStories.Day(at)
	if type(at) ~= "number" then
		return "?"
	end
	if date("%Y-%m-%d", at) == date("%Y-%m-%d", time()) then
		return "Today"
	end
	return (date("%d %b", at):gsub("^0", ""))
end

local Day = JournalStories.Day

-- The reason why the last Accept of a story did not go, by its key, until another try.
local problems = {}

-- The keys of the rows, so a key never mixes two kinds.
local function WaitingKey(story)
	return "w:" .. story.author .. ":" .. story.id
end

local function AcceptedKey(story)
	return "a:" .. tostring(story.number)
end

local function DraftKey(draft)
	return "d:" .. draft.to
end

local function AuthorName(story)
	local author = ns.PlayerStories.AuthorOf(story.number)
	return author and Short(author) or "a friend"
end

local function WaitingTitle(story)
	if story.title ~= "" then
		return ns.Plain(story.title), false
	end
	return "A story from " .. Short(story.author), true
end

local function AcceptedTitle(story)
	if type(story.title) == "string" then
		return ns.PlayerStories.Shown(story.title), false
	end
	return "A story from " .. AuthorName(story), true
end

local function DraftTitle(draft)
	if draft.title ~= "" then
		return ns.Plain(draft.title), false
	end
	return "A story about " .. Short(draft.to), true
end

-- Each kind of story: its rows, newest first, and how its page reads.
local function WaitingRows()
	local rows, waiting = {}, ns.PlayerStories.Waiting()
	for index = #waiting, 1, -1 do
		local story = waiting[index]
		local title, faded = WaitingTitle(story)
		local row = Item(WaitingKey(story), title, Short(story.author), Day(story.at))
		row.faded, row.story, row.kind = faded, story, "waiting"
		rows[#rows + 1] = row
	end
	return rows
end

local function AcceptedRows(journal)
	local rows, stories = {}, Entries(journal.stories)
	for index = #stories, 1, -1 do
		local story = stories[index]
		if type(story.number) == "number" then
			local title, faded = AcceptedTitle(story)
			local row = Item(AcceptedKey(story), title, AuthorName(story), Day(story.at))
			row.faded, row.story, row.kind = faded, story, "accepted"
			rows[#rows + 1] = row
		end
	end
	return rows
end

local function DraftRows()
	local rows = {}
	for _, draft in ipairs(ns.StoryDrafts.All()) do
		local title, faded = DraftTitle(draft)
		local row = Item(DraftKey(draft), title, "To " .. Short(draft.to), Day(draft.at))
		row.faded, row.story, row.kind = faded, draft, "draft"
		rows[#rows + 1] = row
	end
	return rows
end

local function Append(list, rows)
	for _, row in ipairs(rows) do
		list[#list + 1] = row
	end
end

-- `journal` is nil while the journal loads: the stories that wait and the drafts live here,
-- so they show anyway.
local function List(journal)
	local waiting, accepted, drafts = WaitingRows(), journal and AcceptedRows(journal) or {}, DraftRows()
	local list = {}
	if #waiting > 0 then
		list[#list + 1] = Group("Waiting " .. #waiting)
		Append(list, waiting)
		if #waiting >= ns.StorySaved.MAX_WAITING then
			list[#list + 1] = { style = "help", text = JournalStories.FULL }
		end
	end
	if not journal then
		list[#list + 1] = { style = "help", text = "Loading..." }
	elseif #accepted > 0 then
		list[#list + 1] = Group("Accepted " .. #accepted)
		Append(list, accepted)
	end
	if #drafts > 0 then
		list[#list + 1] = Group("Drafts " .. #drafts)
		Append(list, drafts)
	end
	return list
end

local function Items(list)
	local items = {}
	for _, row in ipairs(list) do
		if row.style == "item" then
			items[#items + 1] = row
		end
	end
	return items
end

local function Paragraphs(lines, paragraphs, shown)
	for _, paragraph in ipairs(paragraphs) do
		lines[#lines + 1] = Line("prose", shown(paragraph))
	end
end

local function WaitingPage(row)
	local story = row.story
	local lines = {
		Line("note", "New"),
		Line("heading", row.text),
		Line("note", "By " .. Short(story.author) .. " · " .. Day(story.at)),
	}
	Paragraphs(lines, ns.StoryText.Paragraphs(story.text), ns.Plain)
	lines[#lines + 1] = Line("help", ns.TaskPages.REPORT)
	if problems[row.key] then
		lines[#lines + 1] = Line("hint", problems[row.key])
	end
	local buttons = {
		Button("Block player", function()
			ns.PlayerStories.AskBlock(story.author)
		end),
		Button("Decline", function()
			ns.PlayerStories.Decline(JournalStories.WaitingIndex(story))
			ns.Journal.Select("stories", nil)
		end),
		Button("Accept", function()
			problems[row.key] = ns.PlayerStories.Accept(JournalStories.WaitingIndex(story))
			if not problems[row.key] then
				ns.Journal.Select("stories", nil)
			end
			ns.JournalFrame.Refresh()
		end),
	}
	return lines, buttons
end

-- The paragraphs that the desktop sent, less any that is no text.
local function AcceptedParagraphs(story)
	local paragraphs = {}
	for _, paragraph in ipairs(type(story.paragraphs) == "table" and story.paragraphs or {}) do
		if type(paragraph) == "string" then
			paragraphs[#paragraphs + 1] = paragraph
		end
	end
	return paragraphs
end

local function AcceptedPage(row)
	local story = row.story
	local lines = {
		Line("heading", row.text),
		Line("note", "By " .. AuthorName(story) .. " · Accepted " .. Day(story.at)),
	}
	Paragraphs(lines, AcceptedParagraphs(story), ns.PlayerStories.Shown)
	if story.used then
		lines[#lines + 1] = Line("hint", "Your story uses this one, so it stays.")
		return lines, {}
	end
	return lines, {
		Button("Remove", function()
			ns.PlayerStories.Remove(story)
		end),
	}
end

local function DraftPage(row)
	local draft = row.story
	local lines = {
		Line("heading", row.text),
		Line("note", "To " .. Short(draft.to) .. " · " .. Day(draft.at)),
	}
	Paragraphs(lines, ns.StoryText.Paragraphs(ns.StoryText.Body(draft.text)), ns.Plain)
	return lines, {
		Button("Delete", function()
			ns.StoryDrafts.AskDelete(draft.to)
		end),
	}
end

local PAGES = { waiting = WaitingPage, accepted = AcceptedPage, draft = DraftPage }

-- The place of a story among the ones that wait, as Accept and Decline take it.
function JournalStories.WaitingIndex(story)
	for index, each in ipairs(ns.PlayerStories.Waiting()) do
		if each == story then
			return index
		end
	end
end

local function Footer()
	local count = #ns.PlayerStories.Waiting()
	return count > 0 and string.format("%d waiting", count) or JournalStories.USAGE
end

-- The badge of the tab: the count of the stories that wait, or nil for none.
function JournalStories.Badge()
	local count = #ns.PlayerStories.Waiting()
	return count > 0 and count or nil
end

local EMPTY = {
	Line("help", "No stories yet."),
	Line("help", "To tell one, target someone in your group and type /story."),
}

-- Waiting stories come first, so the page opens on the newest one.
function JournalStories.Page(journal)
	local list = List(journal)
	local page = { list = list, lines = {}, buttons = {}, side = "sheet", footer = Footer() }
	local items = Items(list)
	if #items == 0 then
		page.list = nil
		page.lines = EMPTY
		return page
	end
	local keys = Keys(items)
	local row = items[OpenIndex(keys, ns.Journal.Selected("stories"), 1)]
	page.selected = row.key
	page.lines, page.buttons = PAGES[row.kind](row)
	return page
end
