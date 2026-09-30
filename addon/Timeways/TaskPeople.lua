-- The other players of player tasks: their names, how you know them, and whether they are
-- online or next to you (GAMEPLAY.md 4.7). A name here is always "Name-Realm", as the game
-- names the sender of an addon message.

local _, ns = ...

local TaskPeople = {}
ns.TaskPeople = TaskPeople

-- The trade distance of CheckInteractDistance: about 11 yards.
local TRADE_DISTANCE = 2
local MAX_NAME = 64

local function Readable(value)
	return type(value) == "string" and value ~= "" and not issecretvalue(value)
end

-- "Ada" becomes "Ada-Stormrage". A name that is too long or holds a space is no name.
function TaskPeople.Full(name)
	if not Readable(name) or #name > MAX_NAME or name:find("[%s%c|;]") then
		return nil
	end
	if name:find("-", 1, true) then
		return name
	end
	return name .. "-" .. GetNormalizedRealmName()
end

-- The name as the game shows it: without the realm for a player of your own realm.
function TaskPeople.Short(name)
	return Ambiguate(name, "none")
end

local function UnitFull(unit)
	local name, realm = UnitName(unit)
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

-- Returns whether the name is in the guild roster, and whether it is online there.
local function GuildMember(name)
	if not IsInGuild() then
		return false, false
	end
	for n = 1, GetNumGuildMembers() do
		local member, _, _, _, _, _, _, _, online = GetGuildRosterInfo(n)
		if TaskPeople.Full(member) == name then
			return true, online == true
		end
	end
	return false, false
end

local function Friend(name)
	local info = C_FriendList.GetFriendInfo(TaskPeople.Short(name))
	return type(info) == "table" and info or nil
end

-- "party", "guild", or "friend": the ways that a peer may send you a task. Nil for a
-- stranger.
function TaskPeople.Relation(name)
	if TaskPeople.GroupUnit(name) then
		return "party"
	end
	if GuildMember(name) then
		return "guild"
	end
	if Friend(name) then
		return "friend"
	end
end

-- A whisper to a player who is offline puts an error in the chat, so a message waits
-- until this is true.
function TaskPeople.IsOnline(name)
	local unit = TaskPeople.GroupUnit(name)
	if unit then
		return UnitIsConnected(unit) == true
	end
	local _, online = GuildMember(name)
	local friend = Friend(name)
	return online or (friend ~= nil and friend.connected == true)
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

-- Face to face: close enough to trade.
function TaskPeople.IsNear(name)
	local unit = TaskPeople.UnitOf(name)
	return unit ~= nil and CheckInteractDistance(unit, TRADE_DISTANCE) == true
end
