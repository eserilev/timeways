-- The saved variables of player stories, `TimewaysStories` (GAMEPLAY.md 4.8). Any addon can
-- write saved variables, so each entry is checked when the table is first read, and a
-- broken one goes.
--
--   waiting: { id, author, title, text, at }, oldest first. One for each author, 20 in all.
--   told: { [id] = { to, title, at, status } }, the newest 100. No text.
--   authors: { [number] = author } of the accepted stories, the newest 500 numbers.
--   drafts: { to, title, text, at }, one for each player, 10 in all.
--   nextNumber: the number of the next accepted story.
--
-- While dev mode is on, a table in memory takes its place (Dev.SavedTable).

local _, ns = ...

local StorySaved = {}
ns.StorySaved = StorySaved

local GLOBAL = "TimewaysStories"

StorySaved.MAX_WAITING = 20
StorySaved.MAX_TOLD = 100
StorySaved.MAX_AUTHORS = 500
StorySaved.MAX_DRAFTS = 10

-- `sent` waits for an answer. The others end it: the reader's answer, the room of a box that
-- did not take it, or `lost` when the reader's box no longer holds it.
StorySaved.TOLD_STATUSES = {
	sent = true,
	accepted = true,
	declined = true,
	lost = true,
	full = true,
	waiting = true,
	blocked = true,
}

local function Table(value)
	return type(value) == "table" and value or {}
end

local function IsName(value)
	return type(value) == "string" and ns.TaskPeople.Full(value) == value
end

local function IsTime(value)
	return type(value) == "number" and value >= 0 and value % 1 == 0
end

local function IsWaiting(story)
	return type(story) == "table"
		and ns.TaskWire.IsId(story.id)
		and IsName(story.author)
		and ns.StoryText.IsTitle(story.title)
		and ns.StoryText.IsBody(story.text)
		and IsTime(story.at)
end

local function IsTold(id, story)
	return ns.TaskWire.IsId(id)
		and type(story) == "table"
		and IsName(story.to)
		and ns.StoryText.IsTitle(story.title)
		and IsTime(story.at)
		and StorySaved.TOLD_STATUSES[story.status] == true
end

-- A draft is the text as the box shows it: a typed "|" is "||", and blank lines stay. Its
-- only control sign is "\n".
local function IsDraftText(text, letters, bytes)
	return type(text) == "string"
		and #text <= 2 * bytes
		and ns.StoryText.Letters(text) <= 2 * letters
		and not text:gsub("\n", " "):find("%c")
		and ns.Utf8.IsValid(text)
end

local function IsDraft(draft)
	return type(draft) == "table"
		and IsName(draft.to)
		and IsDraftText(draft.title, ns.StoryText.TITLE_LETTERS, ns.StoryText.TITLE_BYTES)
		and IsDraftText(draft.text, ns.StoryText.BODY_LETTERS, ns.StoryText.BODY_BYTES)
		and IsTime(draft.at)
end

-- The oldest story of each author stays, up to the bound.
local function CleanWaiting(list)
	local waiting, authors = {}, {}
	for _, story in ipairs(Table(list)) do
		if IsWaiting(story) and not authors[story.author] and #waiting < StorySaved.MAX_WAITING then
			authors[story.author] = true
			waiting[#waiting + 1] = story
		end
	end
	return waiting
end

local function CleanTold(map)
	local told = {}
	for id, story in pairs(Table(map)) do
		if IsTold(id, story) then
			told[id] = story
		end
	end
	StorySaved.TrimTold(told)
	return told
end

local function CleanAuthors(map)
	local authors = {}
	for number, author in pairs(Table(map)) do
		if IsTime(number) and IsName(author) then
			authors[number] = author
		end
	end
	StorySaved.TrimAuthors(authors)
	return authors
end

local function Newer(a, b)
	return a.at > b.at
end

-- The newest draft of each player stays, up to the bound.
local function CleanDrafts(list)
	local candidates = {}
	for _, draft in ipairs(Table(list)) do
		if IsDraft(draft) then
			candidates[#candidates + 1] = draft
		end
	end
	table.sort(candidates, Newer)
	local drafts, players = {}, {}
	for _, draft in ipairs(candidates) do
		if not players[draft.to] and #drafts < StorySaved.MAX_DRAFTS then
			players[draft.to] = true
			drafts[#drafts + 1] = draft
		end
	end
	return drafts
end

local function Clean(data)
	data.waiting = CleanWaiting(data.waiting)
	data.told = CleanTold(data.told)
	data.authors = CleanAuthors(data.authors)
	data.drafts = CleanDrafts(data.drafts)
	data.nextNumber = IsTime(data.nextNumber) and data.nextNumber or 1
end

local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
function StorySaved.Data()
	local data = ns.Dev.SavedTable(GLOBAL)
	if checked ~= data then
		Clean(data)
		checked = data
	end
	return data
end

-- Keeps the `limit` keys of a map that come first in `order`, and drops the rest.
local function KeepFirst(map, limit, order)
	local keys = {}
	for key in pairs(map) do
		keys[#keys + 1] = key
	end
	if #keys <= limit then
		return
	end
	table.sort(keys, order)
	for n = limit + 1, #keys do
		map[keys[n]] = nil
	end
end

-- Keeps the newest `MAX_TOLD` stories, by `at`.
function StorySaved.TrimTold(told)
	KeepFirst(told, StorySaved.MAX_TOLD, function(a, b)
		return told[a].at > told[b].at
	end)
end

-- Keeps the newest `MAX_AUTHORS` numbers.
function StorySaved.TrimAuthors(authors)
	KeepFirst(authors, StorySaved.MAX_AUTHORS, function(a, b)
		return a > b
	end)
end
