-- The other players of player tasks: their names, how you know them, and whether they are
-- online or next to you (GAMEPLAY.md 4.7). A name here is always "Name-Realm", as the game
-- names the sender of an addon message.

local _, ns = ...

local TaskPeople = {}
ns.TaskPeople = TaskPeople

-- The distances of CheckInteractDistance: trade, about 11 yards, and follow, about 28.
local TRADE_DISTANCE, FOLLOW_DISTANCE = 2, 4
local MAX_NAME = 64

local function Readable(value)
	return type(value) == "string" and value ~= "" and not issecretvalue(value)
end

-- "Ada" becomes "Ada-Stormrage". The realm loses its spaces, as in the name of the sender
-- of an addon message: "Bram-Argent Dawn" becomes "Bram-ArgentDawn". A character name
-- with a space or a dash is no name, so "-Stormrage" never becomes a name.
function TaskPeople.Full(name)
	if not Readable(name) or #name > MAX_NAME or name:find("[%c|;]") then
		return nil
	end
	local short, realm = name:match("^([^-]+)%-(.+)$")
	if not short then
		short, realm = name, GetNormalizedRealmName()
	end
	realm = realm:gsub("%s", "")
	if short:find("[%s-]") or realm == "" then
		return nil
	end
	return short .. "-" .. realm
end

-- The name as the game shows it: without the realm for a player of your own realm.
function TaskPeople.Short(name)
	return Ambiguate(name, "none")
end

local function UnitFull(unit)
	local name, realm = UnitFullName(unit)
	if not Readable(name) then
		return nil
	end
	if Readable(realm) then
		return TaskPeople.Full(name .. "-" .. realm)
	end
	return TaskPeople.Full(name)
end

function TaskPeople.Me()
	return UnitFull("player")
end

-- The player of a unit, or nil for an NPC or a pet.
function TaskPeople.OfUnit(unit)
	if not UnitExists(unit) or not UnitIsPlayer(unit) then
		return nil
	end
	return UnitFull(unit)
end

local function GroupUnits()
	local units = {}
	for n = 1, 4 do
		units[#units + 1] = "party" .. n
	end
	for n = 1, 40 do
		units[#units + 1] = "raid" .. n
	end
	return units
end

local GROUP_UNITS = GroupUnits()

function TaskPeople.GroupUnit(name)
	if not IsInGroup() then
		return nil
	end
	for _, unit in ipairs(GROUP_UNITS) do
		if UnitExists(unit) and UnitFull(unit) == name then
			return unit
		end
	end
end

-- Every player of your group, as full names.
function TaskPeople.GroupNames()
	local names = {}
	for _, unit in ipairs(GROUP_UNITS) do
		local name = UnitExists(unit) and UnitIsPlayer(unit) and UnitFull(unit)
		if name then
			names[#names + 1] = name
		end
	end
	return names
end

-- The guild roster as { [name] = online }. The game fires GUILD_ROSTER_UPDATE when it
-- changes, so a whisper that waits never scans the whole guild each second.
local roster

local function Roster()
	if roster then
		return roster
	end
	roster = {}
	for n = 1, IsInGuild() and GetNumGuildMembers() or 0 do
		local member, _, _, _, _, _, _, _, online = GetGuildRosterInfo(n)
		local name = TaskPeople.Full(member)
		if name then
			roster[name] = online == true
		end
	end
	return roster
end

-- The client learns who is online in the guild only when it asks. The server answers one
-- ask in 10 seconds at most.
local ROSTER_SECONDS = 15
local rosterAskedAt

function TaskPeople.AskGuildRoster()
	local now = GetTime()
	if not IsInGuild() or (rosterAskedAt and now - rosterAskedAt < ROSTER_SECONDS) then
		return
	end
	rosterAskedAt = now
	C_GuildInfo.GuildRoster()
end

function TaskPeople.GuildNames()
	local names = {}
	for name in pairs(Roster()) do
		names[#names + 1] = name
	end
	return names
end

local function Friend(name)
	local info = C_FriendList.GetFriendInfo(TaskPeople.Short(name))
	return type(info) == "table" and info or nil
end

-- "party", "guild", or "friend": the ways that a peer can send you a task. Nil for a
-- stranger.
function TaskPeople.Relation(name)
	if TaskPeople.GroupUnit(name) then
		return "party"
	end
	if Roster()[name] ~= nil then
		return "guild"
	end
	if Friend(name) then
		return "friend"
	end
end

-- A peer who sent you a message in the last minute is online.
local HEARD_SECONDS = 60
local MAX_HEARD = 64
local heard, heardCount = {}, 0

function TaskPeople.Heard(name)
	if not heard[name] then
		if heardCount >= MAX_HEARD then
			heard, heardCount = {}, 0
		end
		heardCount = heardCount + 1
	end
	heard[name] = GetTime()
end

local function HeardLately(name)
	return heard[name] ~= nil and GetTime() - heard[name] <= HEARD_SECONDS
end

-- A unit of the player, for a check of distance: your target, the unit under the mouse,
-- or a member of your group.
function TaskPeople.UnitOf(name)
	for _, unit in ipairs({ "target", "mouseover" }) do
		if TaskPeople.OfUnit(unit) == name then
			return unit
		end
	end
	return TaskPeople.GroupUnit(name)
end

-- "online", "offline", or "unknown": a player who left your group and is in neither your
-- guild nor your friends list is unknown. A whisper to a player who is offline puts an
-- error in the chat, so a message to an offline player waits.
function TaskPeople.Presence(name)
	local unit = TaskPeople.UnitOf(name)
	if unit then
		return UnitIsConnected(unit) == true and "online" or "offline"
	end
	local friend = Friend(name)
	if (friend and friend.connected == true) or HeardLately(name) then
		return "online"
	end
	local inGuild = Roster()[name]
	if inGuild ~= nil then
		return inGuild and "online" or "offline"
	end
	return friend and "offline" or "unknown"
end

function TaskPeople.IsOnline(name)
	return TaskPeople.Presence(name) == "online"
end

-- Every friend, online or not, as the friends list names them.
function TaskPeople.FriendNames()
	local names = {}
	for n = 1, C_FriendList.GetNumFriends() do
		local info = C_FriendList.GetFriendInfoByIndex(n)
		if type(info) == "table" and type(info.name) == "string" and not issecretvalue(info.name) then
			names[#names + 1] = info.name
		end
	end
	return names
end

function TaskPeople.OnlineFriends()
	local names = {}
	for n = 1, C_FriendList.GetNumFriends() do
		local info = C_FriendList.GetFriendInfoByIndex(n)
		local name = type(info) == "table" and info.connected and TaskPeople.Full(info.name)
		if name then
			names[#names + 1] = name
		end
	end
	return names
end

-- Face to face: close enough to trade.
function TaskPeople.IsNear(name)
	local unit = TaskPeople.UnitOf(name)
	return unit ~= nil and CheckInteractDistance(unit, TRADE_DISTANCE) == true
end

-- Close enough that your addon sees what happens around the player: a kill, a talk, a place.
function TaskPeople.IsClose(name)
	local unit = TaskPeople.UnitOf(name)
	return unit ~= nil and CheckInteractDistance(unit, FOLLOW_DISTANCE) == true
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("GUILD_ROSTER_UPDATE")
frame:RegisterEvent("PLAYER_GUILD_UPDATE")
frame:SetScript("OnEvent", function()
	roster = nil
end)
