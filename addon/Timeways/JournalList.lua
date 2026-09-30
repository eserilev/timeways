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
local ITEM_HEIGHT, GROUP_HEIGHT, HELP_GAP = 36, 22, 8
local MARK_WIDTH = 60

local SKINS = {
	map = { title = ns.Ink.cream, detail = ns.Ink.muted, group = ns.Ink.gold },
	sheet = { title = ns.Ink.text, detail = ns.Ink.faded, group = ns.Ink.title },
}

local pane, box, night, parchment, scroll, child
local paneWidth, paneHeight
local items, texts = {}, {}

function JournalList.Build(parent, width, height)
	pane, paneWidth, paneHeight = parent, width, height
	box = CreateFrame("Frame", nil, pane)
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
	button:SetHeight(ITEM_HEIGHT)
	button:SetHighlightTexture("Interface\\QuestFrame\\UI-QuestTitleHighlight", "ADD")
	button.shade = button:CreateTexture(nil, "BACKGROUND")
	button.shade:SetAllPoints(button)
	button.shade:SetColorTexture(unpack(ns.Ink.selected))
	button.title = Label(button, "GameFontNormal")
	button.title:SetPoint("TOPLEFT", button, "TOPLEFT", PADDING, -4)
	button.detail = Label(button, "GameFontHighlightSmall")
	button.detail:SetPoint("TOPLEFT", button, "TOPLEFT", PADDING, -20)
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
	local colors = SKINS[skin]
	local width = RowWidth(skin)
	button:SetWidth(width)
	button.title:SetText(row.text)
	button.title:SetWidth(width - 2 * PADDING - (row.mark and MARK_WIDTH or 0))
	Color(button.title, colors.title)
	button.detail:SetText(row.detail or "")
	button.detail:SetWidth(width - 2 * PADDING)
	Color(button.detail, colors.detail)
	button.mark:SetText(row.mark or "")
	Color(button.mark, colors.detail)
	button.shade:SetShown(row.key == openKey)
	button:SetScript("OnClick", function()
		ns.JournalFrame.Select(row.key)
	end)
	return ITEM_HEIGHT
end

-- A group title is one short line in capitals. A line of help wraps.
local function DrawText(text, row, skin)
	local colors = SKINS[skin]
	local group = row.style == "group"
	text:SetFontObject(group and "GameFontNormalSmall" or "GameFontHighlightSmall")
	text:SetJustifyH("LEFT")
	text:SetWidth(RowWidth(skin) - 2 * PADDING)
	text:SetText(group and row.text:upper() or row.text)
	Color(text, group and colors.group or colors.detail)
	return group and GROUP_HEIGHT or text:GetStringHeight() + HELP_GAP
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
