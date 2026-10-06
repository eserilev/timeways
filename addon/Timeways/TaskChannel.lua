-- Sends and takes the addon messages of player tasks (GAMEPLAY.md 4.7), under the rate
-- limits of the game and of Timeways. A message to a player who is offline waits. A message
-- with text that a player wrote goes on the logged channel, so Blizzard support can read it.

local _, ns = ...

local TaskChannel = {}
ns.TaskChannel = TaskChannel

TaskChannel.PREFIX = "Timeways"

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

-- A random start: after a reload, a counter from 1 joins old parts of a peer to new ones.
local number = math.random(0, 9998)
-- One collector for each channel, so the parts of one message never come from both.
local collectors = {
	[ns.TaskWire.LOGGED] = ns.TaskChunks.NewCollector(),
	[ns.TaskWire.UNLOGGED] = ns.TaskChunks.NewCollector(),
}
local allowance = ns.PeerAllowance.New()

-- The messages that wait, oldest first: { to, channel, log, parts, retryAt }. `parts` holds
-- the parts that the game has not taken yet.
local waiting = {}

C_ChatInfo.RegisterAddonMessagePrefix(TaskChannel.PREFIX)

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

-- The logged channel can answer nil. Nil gives no reason to wait, so it counts as Success.
local function SendPart(entry)
	local target = entry.channel == "WHISPER" and entry.to or nil
	local send = C_ChatInfo.SendAddonMessage
	if entry.log == ns.TaskWire.LOGGED then
		send = C_ChatInfo.SendAddonMessageLogged
	end
	return send(TaskChannel.PREFIX, entry.parts[1], entry.channel, target) or RESULT.Success
end

-- Sends the parts while the rate limit allows. Returns "sent", "wait", or "drop".
local function SendParts(entry, now)
	while #entry.parts > 0 and ns.AddonBudget.Has() do
		local result = SendPart(entry)
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
		ns.AddonBudget.Spend()
	end
	return #entry.parts == 0 and "sent" or "wait"
end

function TaskChannel.Flush()
	local now = GetTime()
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
	-- A fake player of dev mode gets the message in the game itself, never on the wire.
	if to and ns.DevPeer.Has(to) then
		ns.DevPeer.Heard(to, message)
		return
	end
	number = ns.TaskChunks.NextNumber(number)
	local parts = ns.TaskChunks.Split(ns.TaskWire.Encode(message), number)
	if not parts then
		return
	end
	waiting[#waiting + 1] = { to = to, channel = channel, log = ns.TaskWire.Log(message.type), parts = parts }
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
	ns.DevPeer.HeardByAll(message)
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

-- `log` is the channel that the part came on. The rest is the payload of CHAT_MSG_ADDON. The
-- game names the sender, so the sender is known.
function TaskChannel.Received(log, prefix, text, channel, sender)
	if prefix ~= TaskChannel.PREFIX or type(text) ~= "string" or issecretvalue(text) then
		return
	end
	sender = ns.TaskPeople.Full(sender)
	local now = GetTime()
	if not sender or sender == ns.TaskPeople.Me() or not ns.PeerAllowance.Take(allowance, sender, now) then
		return
	end
	ns.TaskPeople.Heard(sender)
	local whole = ns.TaskChunks.Add(collectors[log], sender, text, now)
	local message = whole and ns.TaskWire.Decode(whole)
	if message and ns.TaskWire.CameOnItsChannel(message.type, log) then
		ns.PlayerTasks.Receive(sender, message, channel)
	end
end

local LOG_OF_EVENT = {
	CHAT_MSG_ADDON = ns.TaskWire.UNLOGGED,
	CHAT_MSG_ADDON_LOGGED = ns.TaskWire.LOGGED,
}

local frame = CreateFrame("Frame")
frame:RegisterEvent("CHAT_MSG_ADDON")
frame:RegisterEvent("CHAT_MSG_ADDON_LOGGED")
frame:SetScript("OnEvent", function(_, event, ...)
	TaskChannel.Received(LOG_OF_EVENT[event], ...)
end)

-- A message waits for its player to come online, and for room under the rate limit.
C_Timer.NewTicker(1, TaskChannel.Flush)
