-- Fake players of dev mode, for player stories and player quests with no second player
-- (TESTING.md, "Dev mode"). A fake player sends its messages through the real receive path
-- (TaskChannel.Received), and counts as a member of your group. Each message to it stays in
-- the game: TaskChannel hands it here, and nothing goes on the wire.

local _, ns = ...

local DevPeer = {}
ns.DevPeer = DevPeer

local Dev = ns.Dev

-- A realm that no server has, so a fake player never shares the name of a real one.
DevPeer.REALM = "Devrealm"

-- A real player answers after a moment, never at once.
local ANSWER_SECONDS = 1

-- The fake players by full name: { room, near, story, given, taken }. `story` is the newest
-- story that you told it, `given` the newest quest that you gave it, and `taken` the newest
-- quest that it gave you.
local peers = {}
local number = 0

function DevPeer.Has(name)
	return name ~= nil and peers[name] ~= nil
end

function DevPeer.IsNear(name)
	return peers[name] ~= nil and peers[name].near
end

-- The fake players belong to the dev world, so they go when dev mode turns on or off.
function DevPeer.Clear()
	peers = {}
end

local function Short(name)
	return ns.TaskPeople.Short(name)
end

-- "Kobee" gives "Kobee-Devrealm", and the fake player starts.
local function Peer(short)
	local name = short .. "-" .. DevPeer.REALM
	peers[name] = peers[name] or { room = "open", near = false }
	return name, peers[name]
end

-- The message goes through the parts and the checks of the real receive path.
function DevPeer.Send(from, message)
	number = ns.TaskChunks.NextNumber(number)
	local parts = ns.TaskChunks.Split(ns.TaskWire.Encode(message), number)
	for _, part in ipairs(parts or {}) do
		ns.TaskChannel.Received(ns.TaskWire.Log(message.type), ns.TaskChannel.PREFIX, part, "WHISPER", from)
	end
end

-- A fake player that went in the meantime sends nothing: with dev mode off, its message
-- comes in as one of a real player.
local function SendSoon(from, message)
	C_Timer.After(ANSWER_SECONDS, function()
		if DevPeer.Has(from) then
			DevPeer.Send(from, message)
		end
	end)
end

-- What the fake player does with each message that you send it.
local ANSWERS = {
	hello = function(name, peer)
		if peer.room ~= "blocked" then
			SendSoon(name, { type = "here" })
		end
	end,
	story_ask = function(name, peer)
		SendSoon(name, { type = "story_room", room = peer.room })
	end,
	story = function(name, peer, message)
		if peer.room ~= "open" then
			SendSoon(name, { type = "story_room", room = peer.room })
			return
		end
		peer.story = message.id
		Dev.Say(Short(name) .. " got your story. Type /twdev peer " .. Short(name) .. " accept or decline.")
	end,
	offer = function(name, peer, message)
		peer.given = message.id
		Dev.Say(Short(name) .. " accepts your quest.")
		SendSoon(name, { type = "accept", id = message.id })
	end,
	turnin = function(name, _, message)
		Dev.Say(Short(name) .. " checks your turn-in, and says it's done.")
		SendSoon(name, { type = "result", id = message.id, verdict = "done" })
	end,
}

function DevPeer.Heard(to, message)
	local peer = peers[to]
	local answer = ANSWERS[message.type]
	if answer then
		answer(to, peer, message)
	else
		Dev.Say(Short(to) .. " got: " .. message.type)
	end
end

-- A message to your group and your guild reaches each fake player too.
function DevPeer.HeardByAll(message)
	for name in pairs(peers) do
		DevPeer.Heard(name, message)
	end
end

-- The story that a fake player tells about you.
local function TellStory(name, title)
	local body = Short(name) .. " saw you hold the line at the bridge.\n" .. "Nobody else stayed. You did."
	DevPeer.Send(name, { type = "story", id = ns.TaskStore.NewId(time()), story_title = title, body = body })
end

-- A quest that a fake player gives you: one step that the game checks, and one that it can't.
local function GiveQuest(name, peer)
	peer.taken = ns.TaskStore.NewId(time())
	DevPeer.Send(name, {
		type = "offer",
		id = peer.taken,
		title = "Wolves at the Farm",
		text = "The wolves took two of my sheep. Help me, and I'll owe you one.",
		reward = "1 gold",
		steps = {
			{ kind = "kill", target = "Prowler", count = 3 },
			{ kind = "other", target = "Tell Farmer Saldean the news", count = 1 },
		},
	})
end

-- The claims of the quest that you gave the fake player: each step done just now, where you
-- stand.
local function Claims(count)
	local claims = {}
	for index = 1, count do
		claims[index] = { index = index, at = time(), zone = GetRealZoneText() }
	end
	return claims
end

local ROOMS = { open = true, full = true, blocked = true, waiting = true }

-- What each word after the name of a fake player does.
local ACTIONS = {
	story = function(name, _, rest)
		TellStory(name, rest)
	end,
	quest = function(name, peer)
		GiveQuest(name, peer)
	end,
	write = function(name)
		ns.StoryScroll.OpenFor(name)
	end,
	accept = function(name, peer)
		if peer.story then
			DevPeer.Send(name, { type = "story_accept", id = peer.story })
		end
	end,
	decline = function(name, peer)
		if peer.story then
			DevPeer.Send(name, { type = "story_decline", id = peer.story })
		end
	end,
	near = function(_, peer)
		peer.near = true
	end,
	far = function(_, peer)
		peer.near = false
	end,
	step = function(name, peer, rest)
		local index = tonumber(rest)
		if peer.given and index then
			DevPeer.Send(name, { type = "step", id = peer.given, index = index, at = time(), zone = GetRealZoneText() })
		end
	end,
	turnin = function(name, peer, rest)
		if peer.given then
			DevPeer.Send(name, { type = "turnin", id = peer.given, claims = Claims(tonumber(rest) or 1) })
		end
	end,
}

for room in pairs(ROOMS) do
	ACTIONS[room] = function(_, peer)
		peer.room = room
	end
end

Dev.Add(
	"peer",
	"peer <name> story [title] | quest | write | open|full|blocked|waiting | accept|decline | near|far"
		.. " | step <n> | turnin [steps]: a fake player of your group.",
	function(rest)
		local short, word, more = rest:match("^(%a+)%s*(%S*)%s*(.-)$")
		local action = word and ACTIONS[word:lower()]
		if not short or not action then
			Dev.Say("usage: /twdev peer <name> story [title] | quest | write | full | blocked | ...")
			return
		end
		local name, peer = Peer(short)
		action(name, peer, more)
	end
)

-- Five players of your group, each with a story about you that waits for your answer.
local TELLERS = {
	{ "Kobee", "The Bridge at Pyrewood" },
	{ "Morvane", "A Debt of Bandages" },
	{ "Brokka", "" },
	{ "Liandra", "The Night Watch" },
	{ "Thrandok", "Ogres, Again" },
}

Dev.Add("inbox", "inbox: five players each tell a story that waits for your answer.", function()
	for _, teller in ipairs(TELLERS) do
		local name = Peer(teller[1])
		TellStory(name, teller[2])
	end
end)

-- Players that you remember, and the profiles of roleplay addons ------------------------------

-- The profile of a roleplay addon that a fake player shares, through the real receive path
-- of MSP (Msp.lua). It needs Share on in your Roleplay Profile, as for a real player.
local mspSession = 0

Dev.Add("msp", "msp <name> / <title>: a fake player shares a roleplay profile with a title.", function(rest)
	local short, title = Dev.Parts(rest)
	if not short:match("^%a+$") or not title then
		Dev.Say("usage: /twdev msp <name> / <title>")
		return
	end
	local name = Peer(short)
	mspSession = ns.MspParts.NextSession(mspSession)
	local text = ns.MspWire.Tooltip({ NA = short .. " of the Reef", NT = title })
	for _, part in ipairs(ns.MspParts.Split(text, mspSession) or {}) do
		ns.Msp.Received(ns.MspWire.PREFIX, part, name, true)
	end
	Dev.Say("Type /twdev tooltip " .. short .. ". With Share off, no profile comes in.")
end)

Dev.Add("remember", "remember <name> friendly|neutral|avoid: mark a fake player.", function(rest)
	local short, mark = rest:match("^(%a+)%s+(%a+)$")
	if not short or not ns.PlayerNotes.LABELS[mark] then
		Dev.Say("usage: /twdev remember <name> friendly|neutral|avoid")
		return
	end
	ns.PlayerNotes.SetMark((Peer(short)), mark)
end)

Dev.Add("note", "note <name>: write a note on a fake player, in the editor of the book.", function(rest)
	if rest:match("^%a+$") then
		ns.PlayerNotes.EditNote((Peer(rest)))
	end
end)

-- The lines that Timeways adds to the tooltip of a player, as the hooks of Msp.lua and
-- PlayerNotes.lua add them for a real one.
Dev.Add("tooltip", "tooltip <name>: the tooltip of a fake player, with its profile and your note.", function(rest)
	if not rest:match("^%a+$") then
		Dev.Say("usage: /twdev tooltip <name>")
		return
	end
	local name = Peer(rest)
	GameTooltip:SetOwner(UIParent, "ANCHOR_CURSOR")
	GameTooltip:AddLine(rest)
	local profile = ns.Msp.TooltipLine(name)
	if profile then
		GameTooltip:AddLine(profile, 1, 0.82, 0)
	end
	local note = ns.PlayerNotes.Line(name)
	if note then
		GameTooltip:AddLine(note, 0.78, 0.63, 0.39)
	end
	GameTooltip:Show()
end)
