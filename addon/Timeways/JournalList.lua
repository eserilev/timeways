-- The list on the left of the journal: the tasks, the chapters, or the questions about the
-- hero. A click on an item opens it on the parchment. Over the map, the list floats in a dark
-- box. Alone, it fills the left half on parchment.

local _, ns = ...

local JournalList = {}
ns.JournalList = JournalList

local NAME = "TimewaysJournalList"

local FLOAT_WIDTH, FLOAT_INSET = 250, 10
local PADDING = 8
-- The scroll bar of the template stands right of the scroll frame.
local SCROLL_BAR = 24
local HELP_GAP = 8
local MARK_WIDTH = 60
-- The list floats over the pins of the map.
local BOX_LEVEL = 4

-- Over the map, the list prints in the gold UI fonts of the Quest Log. On parchment, it
-- prints in the quest fonts of the right page. Their shadow smudges dark ink, so it goes.
local SKINS = {
	map = {
		title = ns.Ink.cream,
		detail = ns.Ink.muted,
		group = ns.Ink.gold,
		selected = ns.Ink.selected,
		titleFont = "GameFontNormal",
		detailFont = "GameFontHighlightSmall",
		groupFont = "GameFontNormalSmall",
		shadow = { 1, -1 },
		capitals = true,
		itemHeight = 36,
		groupHeight = 22,
		detailTop = -20,
	},
	sheet = {
		title = ns.Ink.text,
		detail = ns.Ink.faded,
		group = ns.Ink.title,
		selected = ns.Ink.selectedOnParchment,
		titleFont = "QuestFont",
		detailFont = "QuestFontNormalSmall",
		groupFont = "QuestTitleFont",
		shadow = { 0, 0 },
		capitals = false,
		itemHeight = 42,
		groupHeight = 28,
		detailTop = -23,
	},
}

local pane, box, night, parchment, scroll, child
local paneWidth, paneHeight
local items, texts = {}, {}

function JournalList.Build(parent, width, height)
	pane, paneWidth, paneHeight = parent, width, height
	box = CreateFrame("Frame", nil, pane)
	ns.MapPane.Stack(box, pane, BOX_LEVEL)
	night = box:CreateTexture(nil, "BACKGROUND")
	night:SetAllPoints(box)
	night:SetColorTexture(unpack(ns.Ink.night))
	parchment = box:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(box)
	parchment:SetAtlas("QuestBG-Parchment")
	scroll = CreateFrame("ScrollFrame", NAME .. "Scroll", box, "UIPanelScrollFrameTemplate")
	scroll:SetPoint("TOPLEFT", box, "TOPLEFT", 0, -PADDING)
	scroll:SetPoint("BOTTOMRIGHT", box, "BOTTOMRIGHT", -SCROLL_BAR, PADDING)
	child = CreateFrame("Frame", nil, scroll)
	child:SetSize(1, 1)
	scroll:SetScrollChild(child)
	box:Hide()
end

local function Width(skin)
	return skin == "sheet" and paneWidth or FLOAT_WIDTH
end

local function RowWidth(skin)
	return Width(skin) - SCROLL_BAR - PADDING
end

local function Color(text, color)
	text:SetTextColor(color[1], color[2], color[3])
end

local function Ink(text, font, color, skin)
	text:SetFontObject(font)
	text:SetShadowOffset(unpack(SKINS[skin].shadow))
	Color(text, color)
end

local function Label(parent, font)
	local label = parent:CreateFontString(nil, "ARTWORK", font)
	label:SetJustifyH("LEFT")
	label:SetWordWrap(false)
	return label
end

local function ItemButton(n)
	if items[n] then
		return items[n]
	end
	local button = CreateFrame("Button", nil, child)
	button:SetHighlightTexture("Interface\\QuestFrame\\UI-QuestTitleHighlight", "ADD")
	button.shade = button:CreateTexture(nil, "BACKGROUND")
	button.shade:SetAllPoints(button)
	button.title = Label(button, "GameFontNormal")
	button.title:SetPoint("TOPLEFT", button, "TOPLEFT", PADDING, -5)
	button.detail = Label(button, "GameFontHighlightSmall")
	button.mark = Label(button, "GameFontHighlightSmall")
	button.mark:SetJustifyH("RIGHT")
	button.mark:SetPoint("RIGHT", button, "RIGHT", -PADDING, 0)
	button.mark:SetWidth(MARK_WIDTH)
	items[n] = button
	return button
end

local function Text(n)
	texts[n] = texts[n] or child:CreateFontString(nil, "ARTWORK")
	return texts[n]
end

local function DrawItem(button, row, skin, openKey)
	local skinned = SKINS[skin]
	local width = RowWidth(skin)
	button:SetSize(width, skinned.itemHeight)
	button.title:SetText(row.text)
	button.title:SetWidth(width - 2 * PADDING - (row.mark and MARK_WIDTH or 0))
	-- A row with no title of its own, such as "A story from Brokka", prints in faded ink.
	Ink(button.title, skinned.titleFont, row.faded and skinned.detail or skinned.title, skin)
	button.detail:SetPoint("TOPLEFT", button, "TOPLEFT", PADDING, skinned.detailTop)
	button.detail:SetText(row.detail or "")
	button.detail:SetWidth(width - 2 * PADDING)
	Ink(button.detail, skinned.detailFont, skinned.detail, skin)
	button.mark:SetText(row.mark or "")
	Ink(button.mark, skinned.detailFont, skinned.detail, skin)
	button.shade:SetColorTexture(unpack(skinned.selected))
	button.shade:SetShown(row.key == openKey)
	button:SetScript("OnClick", function()
		ns.JournalFrame.Select(row.key)
	end)
	return skinned.itemHeight
end

-- A group title is one short line, in capitals over the map. A line of help wraps.
local function DrawText(text, row, skin)
	local skinned = SKINS[skin]
	text:SetJustifyH("LEFT")
	text:SetWidth(RowWidth(skin) - 2 * PADDING)
	if row.style == "group" then
		Ink(text, skinned.groupFont, skinned.group, skin)
		text:SetText(skinned.capitals and row.text:upper() or row.text)
		return skinned.groupHeight
	end
	Ink(text, skinned.detailFont, skinned.detail, skin)
	text:SetText(row.text)
	return text:GetStringHeight() + HELP_GAP
end

local function Place(widget, y)
	widget:ClearAllPoints()
	widget:SetPoint("TOPLEFT", child, "TOPLEFT", PADDING, -y)
	widget:Show()
end

local function HideFrom(list, first)
	for n = first, #list do
		list[n]:Hide()
	end
end

-- Returns the height of the rows.
local function DrawRows(rows, openKey, skin)
	local y, itemCount, textCount = 0, 0, 0
	for _, row in ipairs(rows) do
		local widget, height
		if row.style == "item" then
			itemCount = itemCount + 1
			widget = ItemButton(itemCount)
			height = DrawItem(widget, row, skin, openKey)
		else
			textCount = textCount + 1
			widget = Text(textCount)
			height = DrawText(widget, row, skin)
		end
		Place(widget, y)
		y = y + height
	end
	HideFrom(items, itemCount + 1)
	HideFrom(texts, textCount + 1)
	return y
end

-- The box over the map grows with its rows, so it hides as little of the map as it can.
local function Fit(skin, rowsHeight)
	box:ClearAllPoints()
	if skin == "sheet" then
		box:SetPoint("TOPLEFT", pane, "TOPLEFT", 0, 0)
		box:SetSize(paneWidth, paneHeight)
	else
		box:SetPoint("TOPLEFT", pane, "TOPLEFT", FLOAT_INSET, -FLOAT_INSET)
		box:SetSize(FLOAT_WIDTH, math.min(rowsHeight + 2 * PADDING, paneHeight - 2 * FLOAT_INSET))
	end
	night:SetShown(skin == "map")
	parchment:SetShown(skin == "sheet")
end

-- The scroll bar shows only when the rows are longer than the box.
local function ShowScrollBar(rowsHeight, skin)
	local bar = scroll.ScrollBar
	local room = skin == "sheet" and paneHeight or paneHeight - 2 * FLOAT_INSET
	if type(bar) == "table" then
		bar:SetShown(rowsHeight + 2 * PADDING > room)
	end
end

-- `skin` is "map" for a box over the map, or "sheet" for the whole left half.
function JournalList.Draw(rows, openKey, skin)
	if not rows or #rows == 0 then
		box:Hide()
		return
	end
	local height = DrawRows(rows, openKey, skin)
	child:SetSize(RowWidth(skin), height)
	Fit(skin, height)
	ShowScrollBar(height, skin)
	box:Show()
end

function JournalList.SetShown(visible)
	box:SetShown(visible)
end
