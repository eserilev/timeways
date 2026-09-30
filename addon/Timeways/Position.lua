-- Where the player stands on the map of the game, for the pins of the journal map
-- (GAMEPLAY.md 3.6). The client can hide a value from addons, and a hidden value is no
-- position.

local _, ns = ...

local Position = {}
ns.Position = Position

local function Readable(value)
	return type(value) == "number" and not issecretvalue(value)
end

-- The player's point on this map, each from 0 to 1, or nil on a map where the player is not.
function Position.OnMap(map)
	local spot = C_Map.GetPlayerMapPosition(map, "player")
	if type(spot) ~= "table" or not Readable(spot.x) or not Readable(spot.y) then
		return nil
	end
	return spot.x, spot.y
end

-- Whole thousandths, because the journal of the desktop takes no fractions.
local function Thousandths(value)
	return math.floor(value * 1000 + 0.5)
end

local function OnTheMap(n)
	return n >= 0 and n <= 1000
end

-- { map, x, y } in thousandths, or nil. The game gives 0, 0 where it knows no position.
local function SpotOn(map)
	local x, y = Position.OnMap(map)
	if not x then
		return nil
	end
	x, y = Thousandths(x), Thousandths(y)
	if not (OnTheMap(x) and OnTheMap(y)) or (x == 0 and y == 0) then
		return nil
	end
	return { map = map, x = x, y = y }
end

-- Enough for a cave in a district of a city, and a guard against a loop of parents.
local MAX_PARENTS = 8

-- The journal map shows zones, so a city district or a cave goes up to the zone that holds
-- it. Nil when no zone holds the map.
local function ZoneOf(map)
	for _ = 1, MAX_PARENTS do
		local info = C_Map.GetMapInfo(map)
		if type(info) ~= "table" or not Readable(info.mapType) then
			return nil
		end
		if info.mapType == Enum.UIMapType.Zone then
			return map
		end
		if info.mapType < Enum.UIMapType.Zone or not Readable(info.parentMapID) then
			return nil
		end
		map = info.parentMapID
	end
end

-- On the zone map when the zone knows the position. A dungeon is apart from its zone, so
-- there the position stays on the dungeon map.
function Position.Here()
	local map = C_Map.GetBestMapForUnit("player")
	if not Readable(map) then
		return nil
	end
	local zone = ZoneOf(map)
	return zone and SpotOn(zone) or SpotOn(map)
end
