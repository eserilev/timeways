-- The map on the left of the journal: the game's own art of one zone, with a pin where the
-- player stands. The art comes in tiles, as the world map draws it in
-- Blizzard_MapCanvas/Blizzard_MapCanvasDetailLayer.lua.

local _, ns = ...

local MapPane = {}
ns.MapPane = MapPane

local PIN_SIZE = 32

local pane, empty, pin
local paneWidth, paneHeight
local tiles = {}
-- The map of each zone by its name, read once from the map tree of the world.
local zones
local shown

function MapPane.Build(parent, width, height)
	paneWidth, paneHeight = width, height
	pane = CreateFrame("Frame", nil, parent)
	pane:SetSize(width, height)
	-- The art covers the pane and runs past its edges. The pane cuts it off.
	pane:SetClipsChildren(true)
	local shade = pane:CreateTexture(nil, "BACKGROUND")
	shade:SetAllPoints(pane)
	shade:SetColorTexture(0.16, 0.12, 0.07)
	pin = pane:CreateTexture(nil, "OVERLAY")
	pin:SetAtlas("Waypoint-MapPin-Tracked")
	pin:SetSize(PIN_SIZE, PIN_SIZE)
	empty = pane:CreateFontString(nil, "ARTWORK", "GameFontDisable")
	empty:SetPoint("CENTER", pane, "CENTER")
	empty:SetText("No map for this place.")
	return pane
end

local function ReadZones()
	local found = {}
	local world = C_Map.GetFallbackWorldMapID()
	for _, map in ipairs(C_Map.GetMapChildrenInfo(world, nil, true) or {}) do
		found[map.name] = found[map.name] or map.mapID
	end
	return found
end

-- The map of this zone, or else the map where the player stands.
function MapPane.MapFor(zone)
	-- The client can send an empty tree while it loads, so an empty tree is read again.
	if not zones or next(zones) == nil then
		zones = ReadZones()
	end
	return zones[zone] or C_Map.GetBestMapForUnit("player")
end

local function Positive(value)
	return type(value) == "number" and value > 0
end

-- The first layer of the art of the map, while it has a size that the pane can draw.
local function ArtLayer(map)
	local layers = C_Map.GetMapArtLayers(map)
	local layer = type(layers) == "table" and layers[1] or nil
	if type(layer) ~= "table" then
		return nil
	end
	local sized = Positive(layer.layerWidth) and Positive(layer.layerHeight)
	return sized and Positive(layer.tileWidth) and Positive(layer.tileHeight) and layer or nil
end

local function Tile(n)
	tiles[n] = tiles[n] or pane:CreateTexture(nil, "BACKGROUND", nil, 1)
	return tiles[n]
end

-- The art fills the pane and keeps its shape, so it runs past two edges of the pane.
local function Cover(layer)
	local scale = math.max(paneWidth / layer.layerWidth, paneHeight / layer.layerHeight)
	local width, height = layer.layerWidth * scale, layer.layerHeight * scale
	return {
		scale = scale,
		width = width,
		height = height,
		left = (paneWidth - width) / 2,
		top = (paneHeight - height) / 2,
	}
end

-- Each tile is square, and the last row and column run past the art.
local function DrawTiles(map, layer, art)
	local textures = C_Map.GetMapArtLayerTextures(map, 1) or {}
	local columns = math.ceil(layer.layerWidth / layer.tileWidth)
	local rows = math.ceil(layer.layerHeight / layer.tileHeight)
	local width, height = layer.tileWidth * art.scale, layer.tileHeight * art.scale
	local n = 0
	for row = 1, rows do
		for column = 1, columns do
			n = n + 1
			local tile = Tile(n)
			tile:SetTexture(textures[n])
			tile:SetSize(width, height)
			tile:ClearAllPoints()
			tile:SetPoint("TOPLEFT", pane, "TOPLEFT", art.left + (column - 1) * width, -(art.top + (row - 1) * height))
			tile:Show()
		end
	end
	for extra = n + 1, #tiles do
		tiles[extra]:Hide()
	end
end

local function PlacePin(map, art)
	local x, y = ns.Position.OnMap(map)
	pin:SetShown(x ~= nil)
	if not x then
		return
	end
	pin:ClearAllPoints()
	pin:SetPoint("BOTTOM", pane, "TOPLEFT", art.left + x * art.width, -(art.top + y * art.height))
end

local function ShowNothing()
	shown = nil
	empty:Show()
	pin:Hide()
	for _, tile in ipairs(tiles) do
		tile:Hide()
	end
end

-- Draws the map of the zone, and returns the name of the map that it draws, or nil.
function MapPane.Show(zone)
	local map = MapPane.MapFor(zone)
	local layer = map and ArtLayer(map)
	if not layer then
		ShowNothing()
		return nil
	end
	local art = Cover(layer)
	empty:Hide()
	DrawTiles(map, layer, art)
	PlacePin(map, art)
	shown = map
	local info = C_Map.GetMapInfo(map)
	return type(info) == "table" and info.name or nil
end

function MapPane.Shown()
	return shown
end

function MapPane.SetShown(visible)
	pane:SetShown(visible)
end
