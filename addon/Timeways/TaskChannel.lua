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

local SUCCESS = 0

local tokens, filled = BURST, nil
local number = 0
local collector = ns.TaskChunks.NewCollector()
local allowance = {}
local allowanceCount = 0

-- The messages that wait, oldest first: { to, channel, text }. A whisper waits until its
-- player is online.
local waiting = {}

C_ChatInfo.RegisterAddonMessagePrefix(TaskChannel.PREFIX)

local function Refill(now)
	filled = filled or now
	tokens = math.min(BURST, tokens + (now - filled) * PER_SECOND)
	filled = now
end

local function Ready(entry)
	return entry.channel ~= "WHISPER" or ns.TaskPeople.IsOnline(entry.to)
end

-- Returns false when the game refused the first part, so the message waits.
local function SendNow(entry)
	number = ns.TaskChunks.NextNumber(number)
	local parts = ns.TaskChunks.Split(entry.text, number) or {}
	for n, part in ipairs(parts) do
		local target = entry.channel == "WHISPER" and entry.to or nil
		local result = C_ChatInfo.SendAddonMessage(TaskChannel.PREFIX, part, entry.channel, target)
		if result ~= SUCCESS and n == 1 then
			return false
		end
	end
	tokens = tokens - #parts
	return true
end

function TaskChannel.Flush()
	Refill(GetTime())
	local kept = {}
	for _, entry in ipairs(waiting) do
		local sent = tokens >= 1 and Ready(entry) and SendNow(entry)
		if not sent then
			kept[#kept + 1] = entry
		end
	end
	waiting = kept
end

local function Queue(to, channel, message)
	waiting[#waiting + 1] = { to = to, channel = channel, text = ns.TaskWire.Encode(message) }
	while #waiting > MAX_WAITING do
		table.remove(waiting, 1)
	end
	TaskChannel.Flush()
end

function TaskChannel.Whisper(to, message)
	Queue(to, "WHISPER", message)
end

-- To your group and your guild at once.
function TaskChannel.Broadcast(message)
	if IsInGroup() then
		Queue(nil, IsInRaid() and "RAID" or "PARTY", message)
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
function TaskChannel.Received(prefix, text, _, sender)
	if prefix ~= TaskChannel.PREFIX or type(text) ~= "string" or issecretvalue(text) then
		return
	end
	sender = ns.TaskPeople.Full(sender)
	local now = GetTime()
	if not sender or sender == ns.TaskPeople.Me() or not Allowed(sender, now) then
		return
	end
	local whole = ns.TaskChunks.Add(collector, sender, text, now)
	local message = whole and ns.TaskWire.Decode(whole)
	if message then
		ns.PlayerTasks.Receive(sender, message)
	end
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("CHAT_MSG_ADDON")
frame:SetScript("OnEvent", function(_, _, ...)
	TaskChannel.Received(...)
end)

-- A message waits for its player to come online, and for room under the rate limit.
C_Timer.NewTicker(1, TaskChannel.Flush)
