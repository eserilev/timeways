-- Stories that players tell about each other (GAMEPLAY.md 4.8). A player in your group tells
-- a short story about you. You accept it into your story, or you decline it. An accepted
-- story goes to the desktop with no real name in it: the author stays here.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local PlayerStories = {}
ns.PlayerStories = PlayerStories

local GLOBAL = "TimewaysStories"
-- A peer can have this many stories waiting for you, and all peers this many together.
local WAITING_PER_AUTHOR = 3
local MAX_WAITING = 20
-- The newest of the stories that you told stay, and the authors of the newest stories.
local MAX_KEPT = 100

PlayerStories.TYPES = { story = true, story_accept = true, story_decline = true }

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Short(name)
	return ns.TaskPeople.Short(name)
end

local function Table(value)
	return type(value) == "table" and value or {}
end

local function IsId(value)
	return type(value) == "string" and #value <= 16 and value:match("^%w+$") ~= nil
end

local function IsName(value)
	return type(value) == "string" and ns.TaskPeople.Full(value) == value
end

local function IsText(value)
	return type(value) == "string"
		and value ~= ""
		and #value <= ns.TaskWire.LIMITS.text
		and ns.TaskWire.IsCleanText(value)
end

local function IsCount(value)
	return type(value) == "number" and value >= 0 and value % 1 == 0
end

-- Any addon can write saved variables, so each entry is checked once, and a broken one goes.
local function Clean(data)
	local waiting = {}
	for _, story in ipairs(Table(data.waiting)) do
		if type(story) == "table" and IsId(story.id) and IsName(story.author) and IsText(story.text) then
			waiting[#waiting + 1] = story
		end
	end
	data.waiting = waiting
	local told = {}
	for id, story in pairs(Table(data.told)) do
		if IsId(id) and type(story) == "table" and IsName(story.to) and IsText(story.text) then
			told[id] = story
		end
	end
	data.told = told
	local authors = {}
	for number, author in pairs(Table(data.authors)) do
		if IsCount(number) and IsName(author) then
			authors[number] = author
		end
	end
	data.authors = authors
	data.nextNumber = IsCount(data.nextNumber) and data.nextNumber or 1
end

local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
local function Data()
	local data = Table(_G[GLOBAL])
	_G[GLOBAL] = data
	if checked ~= data then
		Clean(data)
		checked = data
	end
	return data
end

local function Changed()
	ns.Journal.Request(0)
	ns.JournalFrame.Refresh()
end

-- Keeps the newest `MAX_KEPT` of a map whose values carry `at`.
local function Trim(map)
	local count, oldest = 0, nil
	for key, value in pairs(map) do
		count = count + 1
		if not oldest or (value.at or 0) < (map[oldest].at or 0) then
			oldest = key
		end
	end
	if count > MAX_KEPT then
		map[oldest] = nil
	end
end

-- Telling ------------------------------------------------------------------------------------

-- Only a player of your group hears your story, and only one who has not blocked you.
function PlayerStories.Tell(to, text)
	text = tostring(text or ""):gsub("%s+", " "):match("^%s*(.-)%s*$")
	if not to or not ns.TaskPeople.GroupUnit(to) then
		Say("Target a player in your group first.")
		return nil
	end
	if not IsText(text) then
		Say(string.format("A story is one line of up to %d letters, with no |.", ns.TaskWire.LIMITS.text))
		return nil
	end
	if ns.TaskStore.Data().refusedBy[to] then
		Say(Short(to) .. " doesn't take stories from you.")
		return nil
	end
	local data = Data()
	local id = ns.TaskStore.NewId(time())
	data.told[id] = { to = to, text = text, at = time(), status = "sent" }
	Trim(data.told)
	ns.TaskChannel.Whisper(to, { type = "story", id = id, text = text })
	Say("You told " .. Short(to) .. " a story about them.")
	return id
end

-- Hearing ------------------------------------------------------------------------------------

local function CountFrom(waiting, author)
	local count = 0
	for _, story in ipairs(waiting) do
		if story.author == author then
			count = count + 1
		end
	end
	return count
end

local function Heard(sender, message)
	local data = Data()
	local blocked = ns.TaskStore.Data().blocked[sender]
	if blocked or not IsText(message.text) or not ns.TaskPeople.GroupUnit(sender) then
		return
	end
	if #data.waiting >= MAX_WAITING or CountFrom(data.waiting, sender) >= WAITING_PER_AUTHOR then
		return
	end
	for _, story in ipairs(data.waiting) do
		if story.author == sender and story.id == message.id then
			return
		end
	end
	table.insert(data.waiting, { id = message.id, author = sender, text = message.text, at = time() })
	Say(Short(sender) .. " told a story about you. Type /story to read it.")
	ns.JournalFrame.Refresh()
end

local function Answered(sender, message, status)
	local story = Data().told[message.id]
	if not story or story.to ~= sender or story.status ~= "sent" then
		return
	end
	story.status = status
	Say(Short(sender) .. (status == "accepted" and " accepted your story." or " declined your story."))
end

-- `sender` is the full name that the game gave. The message came as a whisper.
function PlayerStories.Receive(sender, message)
	if message.type == "story" then
		Heard(sender, message)
	elseif message.type == "story_accept" then
		Answered(sender, message, "accepted")
	elseif message.type == "story_decline" then
		Answered(sender, message, "declined")
	end
end

-- Answering ----------------------------------------------------------------------------------

function PlayerStories.Waiting()
	return Data().waiting
end

-- The author of an accepted story, by the number that the desktop knows.
function PlayerStories.AuthorOf(number)
	return Data().authors[number]
end

local function Take(index)
	return table.remove(Data().waiting, index)
end

-- Names of players become "my friend", and yours becomes `$N`, before the text leaves.
function PlayerStories.Accept(index)
	local story = Take(index or #Data().waiting)
	if not story then
		return
	end
	local data = Data()
	local number = data.nextNumber
	data.nextNumber = number + 1
	data.authors[number] = story.author
	local text = ns.TaskNames.WithoutNames(story.text)
	ns.Outbox.Add(ns.Inputs.StoryAccepted(time(), number, text))
	ns.TaskChannel.Whisper(story.author, { type = "story_accept", id = story.id })
	Say("Story accepted. It's part of your story now.")
	Changed()
end

function PlayerStories.Decline(index)
	local story = Take(index or #Data().waiting)
	if not story then
		return
	end
	ns.TaskChannel.Whisper(story.author, { type = "story_decline", id = story.id })
	Say("Story declined.")
	ns.JournalFrame.Refresh()
end

StaticPopupDialogs.TIMEWAYS_STORY_REMOVE = {
	text = "Remove this story?\n\n%s",
	button1 = "Remove",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		ns.Outbox.Add(ns.Inputs.StoryRemoved(time(), data.number))
		Changed()
	end,
}

-- Asks first, because a removed story never comes back.
function PlayerStories.Remove(story)
	local number = story.number
	if IsCount(number) then
		StaticPopup_Show("TIMEWAYS_STORY_REMOVE", ns.Plain(tostring(story.text)), nil, { number = number })
	end
end

-- `$N` stands for you in an accepted story.
function PlayerStories.Shown(text)
	local me = UnitName("player") or "you"
	return (ns.Plain(tostring(text)):gsub("%$N", me))
end

local USAGE = "Usage: /story <words> to tell your target a story about them, /story accept, or /story decline."

-- `/story` alone shows the newest story that waits.
function PlayerStories.Command(message)
	local words = tostring(message or ""):match("^%s*(.-)%s*$")
	local verb = words:lower()
	if verb == "accept" then
		PlayerStories.Accept()
	elseif verb == "decline" then
		PlayerStories.Decline()
	elseif verb == "" then
		local story = Data().waiting[#Data().waiting]
		if story then
			Say(Short(story.author) .. " says: " .. ns.Plain(story.text))
			Say("Type /story accept or /story decline.")
			Say(ns.TaskPages.REPORT)
		else
			Say(USAGE .. " " .. ns.TaskPages.LOGGED)
		end
	else
		PlayerStories.Tell(ns.TaskPeople.OfUnit("target"), words)
	end
end
