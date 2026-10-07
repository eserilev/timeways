-- The map on the left of the journal: the game's own art of one zone, with a pin where the
-- player stands. The art comes in tiles, as the world map draws it in
-- Blizzard_MapCanvas/Blizzard_MapCanvasDetailLayer.lua.

local _, ns = ...

local MapPane = {}
ns.MapPane = MapPane

local PIN_SIZE = 32
local STEP_PIN_SIZE, GIVER_PIN_SIZE = 24, 20
-- The pin of the open place or person stands out.
local SELECTED_SCALE = 1.4
-- A done step stays on the map, faded, so the path of the task still shows.
local DONE_ALPHA = 0.4
local VISITED_HEIGHT = 22
-- A map lies a few steps below its continent: a cave, a zone, a continent. More steps
-- than this means a loop in the tree.
local MAX_DEPTH = 8
-- Regions of one layer and one sublevel draw in no fixed order, so the band of visited places
-- sits under the task pins.
local BAND_SUBLEVEL = -1
-- The pins are buttons, which draw over every texture of the pane, and under the list of
-- the journal and the button over the map.
local PIN_LEVEL = 2

local pane, empty, pin, visitedBand, visited
local paneWidth, paneHeight
local tiles, explored, pins = {}, {}, {}
-- The map of each zone by its name, read once from the map tree of the world.
local zones
local shown

-- Stacks `frame` over the other children of `parent`, `steps` levels up. The game can hide
-- a frame level, and then the frame keeps its level.
function MapPane.Stack(frame, parent, steps)
	local level = parent:GetFrameLevel()
	if not issecretvalue(level) then
		frame:SetFrameLevel(level + steps)
	end
end

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
	visitedBand = pane:CreateTexture(nil, "ARTWORK", nil, BAND_SUBLEVEL)
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
-- The map of the zone with this name, or nil.
local function Named(zone)
	-- The client can send an empty tree while it loads, so an empty tree is read again.
	if not zones or next(zones) == nil then
		zones = ReadZones()
	end
	return zone and zones[zone]
end

function MapPane.MapFor(zone)
	return Named(zone) or C_Map.GetBestMapForUnit("player")
end

-- The continent around the map of this zone: { map, name }, or nil when the game knows
-- none. A parent of each map leads up to it.
function MapPane.ContinentOf(zone)
	local map = Named(zone)
	for _ = 1, MAX_DEPTH do
		local info = map and C_Map.GetMapInfo(map)
		if type(info) ~= "table" then
			return nil
		end
		if info.mapType == Enum.UIMapType.Continent then
			return { map = map, name = info.name }
		end
		map = info.parentMapID
	end
	return nil
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

-- A pin is a button, so a pin of the atlas takes a click and shows its tooltip.
local function PinWidgets(n)
	if not pins[n] then
		local button = CreateFrame("Button", nil, pane)
		MapPane.Stack(button, pane, PIN_LEVEL)
		local icon = button:CreateTexture(nil, "ARTWORK")
		icon:SetAllPoints(button)
		local number = button:CreateFontString(nil, "OVERLAY", "NumberFontNormal")
		number:SetPoint("CENTER", button, "CENTER", 0, 3)
		pins[n] = { button = button, icon = icon, number = number }
	end
	return pins[n]
end

-- The look of each pin of the atlas: its art, and its size.
local ATLAS_LOOKS = {
	place = { atlas = "Waypoint-MapPin-Untracked", size = 18 },
	person = { file = "Interface\\GossipFrame\\GossipGossipIcon", size = 14 },
	death = { file = "Interface\\TargetingFrame\\UI-TargetingFrame-Skull", size = 14 },
}

local function Art(widgets, look)
	if look.atlas then
		widgets.icon:SetAtlas(look.atlas)
	else
		widgets.icon:SetTexture(look.file)
	end
end

-- The yellow marks of a quest giver in the game: "!" for an offer, "?" for a task taken.
local function Look(widgets, spec)
	widgets.number:Hide()
	if spec.kind == "giver" then
		local mark = spec.offered and "AvailableQuestIcon" or "ActiveQuestIcon"
		widgets.icon:SetTexture("Interface\\GossipFrame\\" .. mark)
		widgets.button:SetSize(GIVER_PIN_SIZE, GIVER_PIN_SIZE)
		return
	end
	local look = ATLAS_LOOKS[spec.kind]
	if look then
		Art(widgets, look)
		local size = spec.selected and look.size * SELECTED_SCALE or look.size
		widgets.button:SetSize(size, size)
		return
	end
	widgets.icon:SetAtlas("Waypoint-MapPin-Untracked")
	widgets.button:SetSize(STEP_PIN_SIZE, STEP_PIN_SIZE)
	widgets.number:SetText(tostring(spec.number))
	widgets.number:Show()
end

local function ShowTip(button, spec)
	GameTooltip:SetOwner(button, "ANCHOR_RIGHT")
	GameTooltip:SetText(spec.title)
	if type(spec.text) == "string" and spec.text ~= "" then
		GameTooltip:AddLine(spec.text, 1, 1, 1)
	end
	GameTooltip:Show()
end

-- A pin of the atlas opens its page and shows a tooltip. A pin of a quest only marks a spot.
local function Behave(button, spec)
	button:EnableMouse(spec.title ~= nil)
	button:SetScript("OnEnter", spec.title and function()
		ShowTip(button, spec)
	end or nil)
	button:SetScript("OnLeave", function()
		GameTooltip:Hide()
	end)
	button:SetScript("OnClick", spec.link and function()
		ns.JournalLinks.Open(spec.link)
	end or nil)
end

local function Fade(spec)
	if spec.done or spec.dim then
		return DONE_ALPHA
	end
	return 1
end

-- Only the pins of the map that shows. A pin's point is its tip, as the pin of the player.
-- A round mark stands on its point.
local function DrawPins(map, art, specs)
	local n = 0
	for _, spec in ipairs(specs or {}) do
		if spec.map == map then
			n = n + 1
			local widgets = PinWidgets(n)
			Look(widgets, spec)
			Behave(widgets.button, spec)
			widgets.button:SetAlpha(Fade(spec))
			widgets.button:ClearAllPoints()
			local x, y = art.left + spec.x * art.width, -(art.top + spec.y * art.height)
			local anchor = (spec.kind == "step" or spec.kind == "place") and "BOTTOM" or "CENTER"
			widgets.button:SetPoint(anchor, pane, "TOPLEFT", x, y)
			widgets.button:Show()
		end
	end
	for extra = n + 1, #pins do
		pins[extra].button:Hide()
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
