-- Stories that players tell about each other (GAMEPLAY.md 4.8). A player in your group tells
-- a story about you. You accept it into your story, or you decline it. An accepted story
-- goes to the desktop with each known player marked: the author stays here.
--
-- One story at a time between two players: the author asks first (`story_ask`), and the
-- box answers with its room (`story_room`). The box checks it again when the story comes,
-- because a changed addon can skip the question.

local _, ns = ...

local PlayerStories = {}
ns.PlayerStories = PlayerStories

PlayerStories.TYPES = {
	story = true,
	story_accept = true,
	story_decline = true,
	story_ask = true,
	story_room = true,
}

-- A player with Timeways answers at once, so no answer in this time means none.
PlayerStories.ASK_SECONDS = 5

-- What the author reads about the room of a box. `%s` is the reader.
PlayerStories.ROOM_LINES = {
	full = "%s's story box is full.",
	waiting = "%s hasn't answered your last story yet.",
	blocked = "%s doesn't take stories from you.",
	none = "%s needs Timeways to get stories.",
}

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Short(name)
	return ns.TaskPeople.Short(name)
end

local function Data()
	return ns.StorySaved.Data()
end

local function Changed()
	ns.Journal.Request(0)
	ns.JournalFrame.Refresh()
end

function PlayerStories.RoomLine(room, name)
	local line = PlayerStories.ROOM_LINES[room]
	return line and string.format(line, Short(name))
end

-- The box ---------------------------------------------------------------------------------

local function WaitingFrom(author)
	for index, story in ipairs(Data().waiting) do
		if story.author == author then
			return index
		end
	end
end

-- Why a story of this author can or can't come in now.
function PlayerStories.RoomFor(author)
	if ns.TaskStore.Data().blocked[author] then
		return "blocked"
	end
	if WaitingFrom(author) then
		return "waiting"
	end
	if #Data().waiting >= ns.StorySaved.MAX_WAITING then
		return "full"
	end
	return "open"
end

local function Answer(to, room)
	ns.TaskChannel.Whisper(to, { type = "story_room", room = room })
end

-- Only a player of your group gets an answer.
local function Asked(sender)
	if ns.TaskPeople.InGroup(sender) then
		Answer(sender, PlayerStories.RoomFor(sender))
	end
end

local function ArrivedLine(sender, title)
	if title ~= "" then
		return Short(sender) .. " told a story about you: " .. title .. ". Type /stories to read it."
	end
	return Short(sender) .. " told a story about you. Type /stories to read it."
end

local function Heard(sender, message)
	if not ns.TaskPeople.InGroup(sender) then
		return
	end
	local index = WaitingFrom(sender)
	if index and Data().waiting[index].id == message.id then
		return
	end
	local room = PlayerStories.RoomFor(sender)
	if room ~= "open" then
		Answer(sender, room)
		return
	end
	local story = { id = message.id, author = sender, title = message.story_title, text = message.body, at = time() }
	table.insert(Data().waiting, story)
	Say(ArrivedLine(sender, story.title))
	ns.JournalFrame.Refresh()
end

-- The author -------------------------------------------------------------------------------

-- The open question to each player: { answer = function(room) }.
local asking = {}

-- The newest story told to this player that still waits for an answer.
local function SentTo(to)
	local newest
	for _, story in pairs(Data().told) do
		if story.to == to and story.status == "sent" and (not newest or story.at > newest.at) then
			newest = story
		end
	end
	return newest
end

local function MarkLost(to)
	local story = SentTo(to)
	while story do
		story.status = "lost"
		story = SentTo(to)
	end
end

-- The room is the truth. "open" means that no story of yours waits there anymore.
local function RoomCame(sender, room)
	if room == "blocked" then
		ns.TaskStore.AddName(ns.TaskStore.Data().refusedBy, sender)
	elseif room == "open" then
		ns.TaskStore.Data().refusedBy[sender] = nil
	end
	local question = asking[sender]
	if question then
		asking[sender] = nil
		if room == "open" then
			MarkLost(sender)
		end
		question.answer(room)
		return
	end
	local story = room ~= "open" and SentTo(sender)
	if story then
		story.status = room
		Say(PlayerStories.RoomLine(room, sender))
	end
end

-- Asks the player for the room of the box. `answer` gets the room, or "none" when no answer
-- came in time.
function PlayerStories.Ask(to, answer)
	local question = { answer = answer }
	asking[to] = question
	ns.TaskChannel.Whisper(to, { type = "story_ask" })
	C_Timer.After(PlayerStories.ASK_SECONDS, function()
		if asking[to] == question then
			asking[to] = nil
			answer("none")
		end
	end)
end

local function Answered(sender, message, status)
	local story = Data().told[message.id]
	if not story or story.to ~= sender or story.status ~= "sent" then
		return
	end
	story.status = status
	Say(Short(sender) .. (status == "accepted" and " accepted your story." or " declined your story."))
end

-- Sends a checked story now, and keeps its title for the answer. A story goes only after the
-- answer "open", so an older story to this player that waits for an answer is lost. A sent
-- story deletes its draft.
function PlayerStories.Send(to, title, body)
	local data = Data()
	MarkLost(to)
	local id = ns.TaskStore.NewId(time())
	data.told[id] = { to = to, title = title, at = time(), status = "sent" }
	ns.StorySaved.TrimTold(data.told)
	ns.TaskChannel.Whisper(to, { type = "story", id = id, story_title = title, body = body })
	ns.StoryDrafts.Delete(to)
	return id
end

-- `sender` is the full name that the game gave. The message came as a whisper.
function PlayerStories.Receive(sender, message)
	if message.type == "story" then
		Heard(sender, message)
	elseif message.type == "story_ask" then
		Asked(sender)
	elseif message.type == "story_room" then
		RoomCame(sender, message.room)
	elseif message.type == "story_accept" then
		Answered(sender, message, "accepted")
	elseif message.type == "story_decline" then
		Answered(sender, message, "declined")
	end
end

-- A quick story from the chat: one paragraph and no title, after the same question.
function PlayerStories.Tell(to, words)
	local body = ns.StoryText.Body(words)
	if not to or not ns.TaskPeople.InGroup(to) then
		Say("Target a player in your group first.")
		return
	end
	local problem = ns.StoryText.Problem("", body)
	if problem then
		Say(problem)
		return
	end
	PlayerStories.Ask(to, function(room)
		if room ~= "open" then
			Say(PlayerStories.RoomLine(room, to))
			return
		end
		PlayerStories.Send(to, "", body)
		Say("Sent to " .. Short(to) .. ". They'll decide if it's part of their story.")
	end)
end

-- Answering --------------------------------------------------------------------------------

function PlayerStories.Waiting()
	return Data().waiting
end

-- The author of an accepted story, by the number that the desktop knows.
function PlayerStories.AuthorOf(number)
	return Data().authors[number]
end

-- The players of every story here: the authors, the players you told stories about, and
-- the players of your drafts.
function PlayerStories.Names()
	local data, names = Data(), {}
	for _, story in ipairs(data.waiting) do
		names[#names + 1] = story.author
	end
	for _, author in pairs(data.authors) do
		names[#names + 1] = author
	end
	for _, story in pairs(data.told) do
		names[#names + 1] = story.to
	end
	for _, draft in ipairs(data.drafts) do
		names[#names + 1] = draft.to
	end
	return names
end

-- The title and the paragraphs with each known player marked, and the players marked.
local function Marked(story)
	local players, seen = {}, {}
	local function Mark(text)
		local marked, named = ns.TaskNames.Marked(text)
		for _, full in ipairs(named) do
			if not seen[full] then
				seen[full] = true
				players[#players + 1] = full
			end
		end
		return marked
	end
	local title = story.title ~= "" and Mark(story.title) or nil
	local paragraphs = {}
	for _, paragraph in ipairs(ns.StoryText.Paragraphs(story.text)) do
		paragraphs[#paragraphs + 1] = Mark(paragraph)
	end
	return title, paragraphs, players
end

PlayerStories.TOO_LONG_TO_KEEP = "This story is too long to keep. Decline it, or ask %s for a shorter one."

-- The story program gives each marked player an ID (5.11), and your name becomes `$N`. A
-- story whose marks make the line too long for one strip stays, and the reason comes back.
function PlayerStories.Accept(index)
	local waiting = Data().waiting
	index = index or #waiting
	local story = waiting[index]
	if not story then
		return
	end
	local data = Data()
	local title, paragraphs, players = Marked(story)
	local accepted = ns.Inputs.StoryAccepted(time(), data.nextNumber, title, paragraphs)
	if not ns.Outbox.Fits(accepted) then
		local problem = string.format(PlayerStories.TOO_LONG_TO_KEEP, Short(story.author))
		Say(problem)
		return problem
	end
	table.remove(waiting, index)
	data.authors[data.nextNumber] = story.author
	ns.StorySaved.TrimAuthors(data.authors)
	data.nextNumber = data.nextNumber + 1
	for _, line in ipairs(ns.TaskNames.Described(players)) do
		ns.Outbox.Add(line)
	end
	ns.Outbox.Add(accepted)
	ns.TaskChannel.Whisper(story.author, { type = "story_accept", id = story.id })
	Say("Story accepted. It's part of your story now.")
	Changed()
end

function PlayerStories.Decline(index)
	local waiting = Data().waiting
	local story = table.remove(waiting, index or #waiting)
	if not story then
		return
	end
	ns.TaskChannel.Whisper(story.author, { type = "story_decline", id = story.id })
	Say("Story declined.")
	ns.JournalFrame.Refresh()
end

-- No more stories or quests from the author. It sends nothing: the author's next question
-- gets the room "blocked".
function PlayerStories.Block(author)
	ns.TaskStore.AddName(ns.TaskStore.Data().blocked, author)
	local index = WaitingFrom(author)
	if index then
		table.remove(Data().waiting, index)
	end
	Say("You won't get quests or stories from " .. Short(author) .. " anymore.")
	ns.JournalFrame.Refresh()
end

StaticPopupDialogs.TIMEWAYS_STORY_BLOCK = {
	text = "Block %s? You won't get quests or stories from them anymore.",
	button1 = "Block",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		PlayerStories.Block(data.author)
	end,
}

-- Asks first.
function PlayerStories.AskBlock(author)
	StaticPopup_Show("TIMEWAYS_STORY_BLOCK", Short(author), nil, { author = author })
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

local function Paragraphs(story)
	local paragraphs = {}
	for _, paragraph in ipairs(type(story.paragraphs) == "table" and story.paragraphs or {}) do
		paragraphs[#paragraphs + 1] = tostring(paragraph)
	end
	return paragraphs
end

-- The paragraphs of a story of the journal, on one line.
function PlayerStories.AcceptedText(story)
	return table.concat(Paragraphs(story), " ")
end

-- What names a story of the journal: its title, or its first paragraph.
function PlayerStories.Label(story)
	if type(story.title) == "string" then
		return story.title
	end
	return Paragraphs(story)[1] or ""
end

-- Asks first, because a removed story never comes back.
function PlayerStories.Remove(story)
	local number = story.number
	if type(number) == "number" then
		local text = ns.WithName(PlayerStories.Label(story))
		StaticPopup_Show("TIMEWAYS_STORY_REMOVE", text, nil, { number = number })
	end
end

local USAGE = "Usage: /story to write a story about your target, /story <words> to tell it"
	.. " now, /story accept, or /story decline. /stories shows the stories about you."

-- `/story` alone opens the scroll for your target.
function PlayerStories.Command(message)
	local words = tostring(message or ""):match("^%s*(.-)%s*$")
	local verb = words:lower()
	if verb == "accept" then
		PlayerStories.Accept()
	elseif verb == "decline" then
		PlayerStories.Decline()
	elseif verb == "help" then
		Say(USAGE .. " " .. ns.TaskPages.LOGGED)
	elseif verb == "" then
		ns.StoryScroll.OpenFor(ns.TaskPeople.OfUnit("target"))
	else
		PlayerStories.Tell(ns.TaskPeople.OfUnit("target"), words)
	end
end
