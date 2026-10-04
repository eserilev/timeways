-- The client calls that only the MSP libraries of MyRolePlay make, on top of the fake game
-- of addon/tests/wow.lua (docs/plans/msp.md, section 4). The API gate never reads this
-- file: these names belong to the libraries, not to Timeways.

-- The short names of the Lua library that WoW adds.
strmatch, strfind, strsub, strlen = string.match, string.find, string.sub, string.len
strupper, strlower, strbyte, strchar = string.upper, string.lower, string.byte, string.char
strrep, format, gsub, gmatch = string.rep, string.format, string.gsub, string.gmatch
tinsert, tremove, tconcat, sort = table.insert, table.remove, table.concat, table.sort
floor, ceil, max, min, abs = math.floor, math.ceil, math.max, math.min, math.abs

-- LibMSP writes a version as format("%.X", bit.arshift(crc, 0)). WoW prints a negative
-- number with "%X" in 32 bits, and plain Lua in 64. So here the shift gives the unsigned
-- number, which plain Lua prints with the same digits as WoW.
function bit.arshift(number, shift)
	local unsigned = number % 2 ^ 32
	if shift == 0 then
		return unsigned
	end
	local signed = unsigned >= 2 ^ 31 and unsigned - 2 ^ 32 or unsigned
	return math.floor(signed / 2 ^ shift)
end

-- The xpcall of WoW passes its extra arguments to the function, as Lua 5.2 does.
local plainXpcall = xpcall
function xpcall(callback, handler, ...)
	local arguments, count = { ... }, select("#", ...)
	return plainXpcall(function()
		return callback(unpack(arguments, 1, count))
	end, handler)
end

local loggedIn = false

function IsLoggedIn()
	return loggedIn
end

-- The login of the game: the libraries finish their setup on PLAYER_LOGIN.
function wow.Login()
	loggedIn = true
	wow.Fire("PLAYER_LOGIN")
	wow.Fire("ADDON_LOADED", "MyRolePlay")
end

-- The global form, hooksecurefunc("Name", hook), next to the form of wow.lua.
local hookOwner = hooksecurefunc
function hooksecurefunc(owner, name, hook)
	if type(owner) == "string" then
		return hookOwner(_G, owner, name)
	end
	return hookOwner(owner, name, hook)
end

function string.join(separator, ...)
	return table.concat({ ... }, separator)
end

function strsplit(separator, text, limit)
	local parts, start = {}, 1
	while true do
		local at = text:find(separator, start, true)
		if not at or (limit and #parts == limit - 1) then
			parts[#parts + 1] = text:sub(start)
			break
		end
		parts[#parts + 1] = text:sub(start, at - 1)
		start = at + #separator
	end
	return unpack(parts)
end

string.split = strsplit

function tContains(list, value)
	for _, each in ipairs(list) do
		if each == value then
			return true
		end
	end
	return false
end

function tInvert(list)
	local inverted = {}
	for key, value in pairs(list) do
		inverted[value] = key
	end
	return inverted
end

function wipe(map)
	for key in pairs(map) do
		map[key] = nil
	end
	return map
end

table.wipe = wipe

function securecallfunction(callback, ...)
	return callback(...)
end

-- The libraries catch the errors of a send. A test reads them here.
wow.errors = {}

function CallErrorHandler(message)
	wow.errors[#wow.errors + 1] = tostring(message)
	return message
end

function geterrorhandler()
	return CallErrorHandler
end

function strcmputf8i(a, b)
	a, b = a:lower(), b:lower()
	return a == b and 0 or (a < b and -1 or 1)
end

function debugprofilestop()
	return GetTime() * 1000
end

function GetFramerate()
	return 60
end

function GetNetStats()
	return 0, 0, 50, 50
end

function GetAutoCompleteRealms()
	return {}
end

function ChatFrame_AddMessageEventFilter() end

function UnitRealmRelationship()
	return nil
end

function IsTrialAccount()
	return false
end

function IsVeteranTrialAccount()
	return false
end

function BNFeaturesEnabledAndConnected()
	return false
end

function BNGetNumFriends()
	return 0, 0
end

function BNSendGameData() end

C_BattleNet = {
	GetFriendNumGameAccounts = function()
		return 0
	end,
	GetFriendGameAccountInfo = function()
		return nil
	end,
}

C_ChatInfo.IsAddonMessagePrefixRegistered = function(prefix)
	return wow.prefixes[prefix] == true
end

Constants = {}
ERR_CHAT_PLAYER_NOT_FOUND_S = "No player named '%s' is currently playing."
LE_REALM_RELATION_COALESCED = 3
BNET_CLIENT_WOW = "WoW"
WOW_PROJECT_MAINLINE = 1
WOW_PROJECT_ID = WOW_PROJECT_MAINLINE
UNKNOWNOBJECT = "Unknown"
MAX_PARTY_MEMBERS = 4
MAX_RAID_MEMBERS = 40

-- A second of the game: ChatThrottleLib sends from the OnUpdate of its frame.
function wow.Update(elapsed)
	for _, frame in ipairs(wow.widgets) do
		local update = frame.scripts.OnUpdate
		if update and frame:IsShown() then
			update(frame, elapsed)
		end
	end
	for index = #wow.after, 1, -1 do
		local timer = wow.after[index]
		if timer.at <= wow.now then
			table.remove(wow.after, index)
			timer.callback()
		end
	end
end

-- A frame of wow.lua answers any capitalized key with a no-op, but Chomp keeps its own
-- fields on its frame. While the libraries load, a new frame answers only real methods.
wow.strictFrames = true
local createFrame = CreateFrame
function CreateFrame(...)
	local frame = createFrame(...)
	if not wow.strictFrames then
		return frame
	end
	local index = getmetatable(frame).__index
	local nothing = index(frame, "NoMethodOfTheFakeGame")
	setmetatable(frame, {
		__index = function(self, key)
			local value = index(self, key)
			if value ~= nothing then
				return value
			end
		end,
	})
	return frame
end
