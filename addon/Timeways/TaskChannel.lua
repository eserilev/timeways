-- Sends and takes the addon messages of player tasks (GAMEPLAY.md 4.7), under the rate
-- limits of the game and of Timeways. A message to a player who is offline waits.

local _, ns = ...

local TaskChannel = {}
ns.TaskChannel = TaskChannel

TaskChannel.PREFIX = "Timeways"

-- The game drops addon messages past about 10 in a burst and 1 each second after it.
local BURST, PER_SECOND = 8, 1
-- A peer gets this many parts in a burst, and one more each few seconds.
local PEER_BURST, PEER_SECONDS = 24, 2
local MAX_PEERS = 64
local MAX_WAITING = 100
-- A whisper that found its player offline waits this long before it tries again, unless
-- the player shows as online first. Each try puts an error in the chat.
local OFFLINE_SECONDS = 300

local RESULT = Enum.SendAddonMessageResult
-- The game never takes a message with these results, so the message is dropped.
local PERMANENT = {
	[RESULT.InvalidPrefix] = true,
	[RESULT.InvalidMessage] = true,
	[RESULT.InvalidChatType] = true,
	[RESULT.NotInGroup] = true,
	[RESULT.TargetRequired] = true,
	[RESULT.InvalidChannel] = true,
	[RESULT.NotInGuild] = true,
}

local tokens, filled = BURST, nil
-- A random start: after a reload, a counter from 1 joins old parts of a peer to new ones.
local number = math.random(0, 9998)
local collector = ns.TaskChunks.NewCollector()
local allowance = {}
local allowanceCount = 0

-- The messages that wait, oldest first: { to, channel, parts, retryAt }. `parts` holds the
-- parts that the game has not taken yet.
local waiting = {}

C_ChatInfo.RegisterAddonMessagePrefix(TaskChannel.PREFIX)

local function Refill(now)
	filled = filled or now
	tokens = math.min(BURST, tokens + (now - filled) * PER_SECOND)
	filled = now
end

local function Ready(entry, now)
	if entry.channel ~= "WHISPER" then
		return true
	end
	if entry.retryAt and now < entry.retryAt and not ns.TaskPeople.IsOnline(entry.to) then
		return false
	end
	if ns.TaskPeople.Presence(entry.to) == "offline" then
		ns.TaskPeople.AskGuildRoster()
		return false
	end
	return true
end

-- Sends the parts while the rate limit allows. Returns "sent", "wait", or "drop".
local function SendParts(entry, now)
	local target = entry.channel == "WHISPER" and entry.to or nil
	while #entry.parts > 0 and tokens >= 1 do
		local result = C_ChatInfo.SendAddonMessage(TaskChannel.PREFIX, entry.parts[1], entry.channel, target)
		if PERMANENT[result] then
			return "drop"
		end
		if result == RESULT.TargetOffline then
			entry.retryAt = now + OFFLINE_SECONDS
			return "wait"
		end
		if result ~= RESULT.Success then
			return "wait"
		end
		table.remove(entry.parts, 1)
		tokens = tokens - 1
	end
	return #entry.parts == 0 and "sent" or "wait"
end

function TaskChannel.Flush()
	local now = GetTime()
	Refill(now)
	local kept = {}
	for _, entry in ipairs(waiting) do
		local outcome = Ready(entry, now) and SendParts(entry, now) or "wait"
		if outcome == "wait" then
			kept[#kept + 1] = entry
		end
	end
	waiting = kept
end

local function Queue(to, channel, message)
	number = ns.TaskChunks.NextNumber(number)
	local parts = ns.TaskChunks.Split(ns.TaskWire.Encode(message), number)
	if not parts then
		return
	end
	waiting[#waiting + 1] = { to = to, channel = channel, parts = parts }
	while #waiting > MAX_WAITING do
		table.remove(waiting, 1)
	end
	TaskChannel.Flush()
end

function TaskChannel.Whisper(to, message)
	Queue(to, "WHISPER", message)
end

-- LE_PARTY_CATEGORY_INSTANCE of the client, which the API gate does not list.
local INSTANCE_GROUP = 2

-- A group of the group finder has only INSTANCE_CHAT.
local function GroupChannel()
	if IsInGroup(INSTANCE_GROUP) then
		return "INSTANCE_CHAT"
	end
	return IsInRaid() and "RAID" or "PARTY"
end

-- To your group and your guild at once.
function TaskChannel.Broadcast(message)
	if IsInGroup() then
		Queue(nil, GroupChannel(), message)
	end
	if IsInGuild() then
		Queue(nil, "GUILD", message)
	end
end

function TaskChannel.Waiting()
	return #waiting
end

-- True while the peer has parts left. A peer that floods loses its parts, and nobody else's.
local function Allowed(sender, now)
	local peer = allowance[sender]
	if not peer then
		if allowanceCount >= MAX_PEERS then
			allowance, allowanceCount = {}, 0
		end
		peer = { parts = PEER_BURST, at = now }
		allowance[sender], allowanceCount = peer, allowanceCount + 1
	end
	peer.parts = math.min(PEER_BURST, peer.parts + (now - peer.at) / PEER_SECONDS)
	peer.at = now
	if peer.parts < 1 then
		return false
	end
	peer.parts = peer.parts - 1
	return true
end

-- The payload of CHAT_MSG_ADDON. The game names the sender, so the sender is known.
function TaskChannel.Received(prefix, text, channel, sender)
	if prefix ~= TaskChannel.PREFIX or type(text) ~= "string" or issecretvalue(text) then
		return
	end
	sender = ns.TaskPeople.Full(sender)
	local now = GetTime()
	if not sender or sender == ns.TaskPeople.Me() or not Allowed(sender, now) then
		return
	end
	ns.TaskPeople.Heard(sender)
	local whole = ns.TaskChunks.Add(collector, sender, text, now)
	local message = whole and ns.TaskWire.Decode(whole)
	if message then
		ns.PlayerTasks.Receive(sender, message, channel)
	end
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("CHAT_MSG_ADDON")
frame:SetScript("OnEvent", function(_, _, ...)
	TaskChannel.Received(...)
end)

-- A message waits for its player to come online, and for room under the rate limit.
C_Timer.NewTicker(1, TaskChannel.Flush)
