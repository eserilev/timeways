-- The map on the left of the journal: the game's own art of one zone, with a pin where the
-- player stands. The art comes in tiles, as the world map draws it in
-- Blizzard_MapCanvas/Blizzard_MapCanvasDetailLayer.lua.

local _, ns = ...

local MapPane = {}
ns.MapPane = MapPane

local PIN_SIZE = 32
local STEP_PIN_SIZE, GIVER_PIN_SIZE = 24, 20
-- A done step stays on the map, faded, so the path of the task still shows.
local DONE_ALPHA = 0.4
local VISITED_HEIGHT = 22

local pane, empty, pin, visitedBand, visited
local paneWidth, paneHeight
local tiles, explored, pins = {}, {}, {}
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
	visitedBand = pane:CreateTexture(nil, "ARTWORK")
	visitedBand:SetColorTexture(unpack(ns.Ink.night))
	visitedBand:SetPoint("BOTTOMLEFT", pane, "BOTTOMLEFT", 0, 0)
	visitedBand:SetSize(width, VISITED_HEIGHT)
	visited = pane:CreateFontString(nil, "OVERLAY", "GameFontHighlightSmall")
	visited:SetPoint("LEFT", pane, "BOTTOMLEFT", 10, VISITED_HEIGHT / 2)
	visited:SetWidth(width - 20)
	visited:SetJustifyH("LEFT")
	visited:SetWordWrap(false)
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

local function HideFrom(list, first)
	for n = first, #list do
		list[n]:Hide()
	end
end

-- The last tile of a part holds the rest of the part, in a file whose size is a power of 2.
local function TileSpan(total, tile, index, count)
	if index < count then
		return tile, tile
	end
	local pixels = total % tile
	if pixels == 0 then
		pixels = tile
	end
	local file = 16
	while file < pixels do
		file = file * 2
	end
	return pixels, file
end

local function Overlay(n)
	explored[n] = explored[n] or pane:CreateTexture(nil, "BACKGROUND", nil, 2)
	return explored[n]
end

local function IsPart(part)
	return type(part) == "table"
		and Positive(part.textureWidth)
		and Positive(part.textureHeight)
		and type(part.offsetX) == "number"
		and type(part.offsetY) == "number"
		and type(part.fileDataIDs) == "table"
end

-- One explored part of the zone, in tiles as the base art. Returns the last overlay used.
local function DrawPart(part, layer, art, n)
	local columns = math.ceil(part.textureWidth / layer.tileWidth)
	local rows = math.ceil(part.textureHeight / layer.tileHeight)
	for row = 1, rows do
		local height, fileHeight = TileSpan(part.textureHeight, layer.tileHeight, row, rows)
		for column = 1, columns do
			local width, fileWidth = TileSpan(part.textureWidth, layer.tileWidth, column, columns)
			n = n + 1
			local overlay = Overlay(n)
			overlay:SetTexture(part.fileDataIDs[(row - 1) * columns + column])
			overlay:SetTexCoord(0, width / fileWidth, 0, height / fileHeight)
			overlay:SetSize(width * art.scale, height * art.scale)
			overlay:ClearAllPoints()
			local x = part.offsetX + (column - 1) * layer.tileWidth
			local y = part.offsetY + (row - 1) * layer.tileHeight
			overlay:SetPoint("TOPLEFT", pane, "TOPLEFT", art.left + x * art.scale, -(art.top + y * art.scale))
			overlay:Show()
		end
	end
	return n
end

-- The base art leaves out the parts of a zone that the character never explored. The game
-- draws each explored part over it, as the world map does in
-- Blizzard_SharedMapDataProviders/MapExplorationDataProvider.lua. A part that shows only
-- under the mouse stays hidden.
local function DrawExplored(map, layer, art)
	local n = 0
	for _, part in ipairs(C_MapExplorationInfo.GetExploredMapTextures(map) or {}) do
		if IsPart(part) and not part.isShownByMouseOver then
			n = DrawPart(part, layer, art, n)
		end
	end
	HideFrom(explored, n + 1)
end

local function PinWidgets(n)
	if not pins[n] then
		local icon = pane:CreateTexture(nil, "ARTWORK")
		local number = pane:CreateFontString(nil, "OVERLAY", "NumberFontNormal")
		number:SetPoint("CENTER", icon, "CENTER", 0, 3)
		pins[n] = { icon = icon, number = number }
	end
	return pins[n]
end

-- The yellow marks of a quest giver in the game: "!" for an offer, "?" for a task taken.
local function Look(widgets, spec)
	if spec.kind == "giver" then
		local mark = spec.offered and "AvailableQuestIcon" or "ActiveQuestIcon"
		widgets.icon:SetTexture("Interface\\GossipFrame\\" .. mark)
		widgets.icon:SetSize(GIVER_PIN_SIZE, GIVER_PIN_SIZE)
		widgets.number:Hide()
		return
	end
	widgets.icon:SetAtlas("Waypoint-MapPin-Untracked")
	widgets.icon:SetSize(STEP_PIN_SIZE, STEP_PIN_SIZE)
	widgets.number:SetText(tostring(spec.number))
	widgets.number:Show()
end

-- Only the pins of the map that shows. A pin's point is its tip, as the pin of the player.
local function DrawPins(map, art, specs)
	local n = 0
	for _, spec in ipairs(specs or {}) do
		if spec.map == map then
			n = n + 1
			local widgets = PinWidgets(n)
			Look(widgets, spec)
			widgets.icon:SetAlpha(spec.done and DONE_ALPHA or 1)
			widgets.number:SetAlpha(spec.done and DONE_ALPHA or 1)
			widgets.icon:ClearAllPoints()
			local x, y = art.left + spec.x * art.width, -(art.top + spec.y * art.height)
			widgets.icon:SetPoint(spec.kind == "giver" and "CENTER" or "BOTTOM", pane, "TOPLEFT", x, y)
			widgets.icon:Show()
		end
	end
	for extra = n + 1, #pins do
		pins[extra].icon:Hide()
		pins[extra].number:Hide()
	end
end

local function ShowNothing()
	shown = nil
	empty:Show()
	pin:Hide()
	HideFrom(tiles, 1)
	HideFrom(explored, 1)
	DrawPins(nil, nil, {})
end

-- The map with this id when the game has its art, or else the map of the zone.
local function Choose(zone, map)
	if map and ArtLayer(map) then
		return map
	end
	return MapPane.MapFor(zone)
end

-- Draws the map, and returns the name of the map that it draws, or nil. `map` is an id that
-- wins over `zone`, and `pins` come from `TaskPins.For`.
function MapPane.Show(zone, map, pinSpecs)
	map = Choose(zone, map)
	local layer = map and ArtLayer(map)
	if not layer then
		ShowNothing()
		return nil
	end
	local art = Cover(layer)
	empty:Hide()
	DrawTiles(map, layer, art)
	DrawExplored(map, layer, art)
	PlacePin(map, art)
	DrawPins(map, art, pinSpecs)
	shown = map
	local info = C_Map.GetMapInfo(map)
	return type(info) == "table" and info.name or nil
end

-- The subzones of the shown zone that the player visited. The game names no subzone that
-- the player did not visit, so the list holds only visits.
function MapPane.ShowVisited(names)
	local any = #names > 0
	visitedBand:SetShown(any)
	visited:SetShown(any)
	visited:SetText(any and ("Visited: " .. table.concat(names, ", ")) or "")
end

function MapPane.Shown()
	return shown
end

function MapPane.SetShown(visible)
	pane:SetShown(visible)
end
