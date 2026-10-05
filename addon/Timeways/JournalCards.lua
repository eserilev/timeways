-- The cards of the Hero page (GAMEPLAY.md 3.7): sub-tabs on top, then lines and cards on
-- parchment over the left half of the journal. Two cards stand side by side, and a wide card
-- takes the whole row. A card shows at most 3 lines of its text.

local _, ns = ...

local JournalCards = {}
ns.JournalCards = JournalCards

local PADDING, GAP = 12, 8
local SCROLL_BAR = 24
local TABS_HEIGHT = 30
local CARD_TOP, CARD_PADDING = 26, 8
local LINE_HEIGHT, MAX_LINES = 14, 3
local LINKED_LABEL = 90

local pane, paneWidth, paneHeight
local box, scroll, child
local tabs, lines, cards, linked = {}, {}, {}, {}

function JournalCards.Build(parent, width, height)
	pane, paneWidth, paneHeight = parent, width, height
	box = CreateFrame("Frame", nil, pane)
	box:SetAllPoints(pane)
	local parchment = box:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(box)
	parchment:SetAtlas("QuestBG-Parchment")
	scroll = CreateFrame("ScrollFrame", "TimewaysJournalCardsScroll", box, "UIPanelScrollFrameTemplate")
	scroll:SetPoint("TOPLEFT", box, "TOPLEFT", 0, -(PADDING + TABS_HEIGHT))
	scroll:SetPoint("BOTTOMRIGHT", box, "BOTTOMRIGHT", -SCROLL_BAR, PADDING)
	child = CreateFrame("Frame", nil, scroll)
	child:SetSize(1, 1)
	scroll:SetScrollChild(child)
	box:Hide()
end

local function Inner()
	return paneWidth - SCROLL_BAR - 2 * PADDING
end

local function Ink(text, ink)
	local color = ns.Ink[ink]
	text:SetTextColor(color[1], color[2], color[3])
end

local function Label(parent, font, ink)
	local text = parent:CreateFontString(nil, "ARTWORK", font)
	text:SetJustifyH("LEFT")
	Ink(text, ink)
	return text
end

-- Returns the width of the button.
local function Fit(button, label)
	return ns.JournalFrame.FitButton(button, label)
end

-- The sub-tabs use the tab code of the top row: the open one is the button that is off.
local function DrawTabs(spec)
	local x = PADDING
	for n, tab in ipairs(spec.tabs) do
		tabs[n] = tabs[n] or ns.JournalFrame.SmallButton(box)
		local button = tabs[n]
		button:ClearAllPoints()
		button:SetPoint("TOPLEFT", box, "TOPLEFT", x, -PADDING)
		local width = Fit(button, tab.label)
		button:SetEnabled(tab.key ~= spec.tab)
		button:SetScript("OnClick", tab.run)
		button:Show()
		x = x + width + 3
	end
	for n = #spec.tabs + 1, #tabs do
		tabs[n]:Hide()
	end
end

local function Action(parent, slot, action)
	slot.button = slot.button or ns.JournalFrame.SmallButton(parent)
	local button = slot.button
	button:SetShown(action ~= nil)
	if not action then
		return 0
	end
	button:SetScript("OnClick", action.run)
	return Fit(button, action.label)
end

-- A line of text, with an optional button at its right end. Returns its height.
local function DrawLine(n, row, y)
	lines[n] = lines[n] or { text = Label(child, "QuestFont", "text") }
	local slot = lines[n]
	local room = Action(child, slot, row.action)
	slot.text:SetFontObject(row.kind == "heading" and "QuestTitleFont" or "QuestFont")
	Ink(slot.text, row.ink or (row.kind == "heading" and "title" or "text"))
	slot.text:SetWidth(Inner() - room - (room > 0 and 6 or 0))
	slot.text:SetText(row.text)
	slot.text:ClearAllPoints()
	slot.text:SetPoint("TOPLEFT", child, "TOPLEFT", PADDING, -y - (room > 0 and 4 or 0))
	slot.text:Show()
	if room > 0 then
		slot.button:ClearAllPoints()
		slot.button:SetPoint("TOPRIGHT", child, "TOPLEFT", PADDING + Inner(), -y)
	end
	return math.max(slot.text:GetStringHeight(), room > 0 and 22 or 0)
end

-- "Origin  Lordaeron, a farm outside Brill.  as Birthplace"
local function DrawLinked(n, row, y)
	if not linked[n] then
		linked[n] = {
			label = Label(child, "QuestFontNormalSmall", "title"),
			text = Label(child, "QuestFont", "text"),
			note = Label(child, "QuestFontNormalSmall", "faded"),
		}
	end
	local slot = linked[n]
	slot.label:SetText(row.label)
	slot.label:ClearAllPoints()
	slot.label:SetPoint("TOPLEFT", child, "TOPLEFT", PADDING, -y)
	slot.note:SetText(row.note)
	slot.note:ClearAllPoints()
	slot.note:SetPoint("TOPRIGHT", child, "TOPLEFT", PADDING + Inner(), -y)
	slot.text:SetText(row.text)
	Ink(slot.text, row.empty and "faded" or "text")
	slot.text:SetWidth(Inner() - LINKED_LABEL - 90)
	slot.text:SetMaxLines(MAX_LINES)
	slot.text:ClearAllPoints()
	slot.text:SetPoint("TOPLEFT", child, "TOPLEFT", PADDING + LINKED_LABEL, -y)
	for _, part in pairs({ slot.label, slot.text, slot.note }) do
		part:Show()
	end
	return math.min(slot.text:GetStringHeight(), MAX_LINES * LINE_HEIGHT)
end

local function CardFrame(n)
	if cards[n] then
		return cards[n]
	end
	local card = CreateFrame("Frame", nil, child)
	local shade = card:CreateTexture(nil, "BACKGROUND")
	shade:SetAllPoints(card)
	local ink = ns.Ink.text
	shade:SetColorTexture(ink[1], ink[2], ink[3], 0.07)
	card.label = Label(card, "QuestTitleFont", "title")
	card.label:SetPoint("TOPLEFT", card, "TOPLEFT", CARD_PADDING, -CARD_PADDING + 2)
	card.tag = Label(card, "QuestFontNormalSmall", "faded")
	card.text = Label(card, "QuestFont", "text")
	card.text:SetPoint("TOPLEFT", card, "TOPLEFT", CARD_PADDING, -CARD_TOP)
	card.text:SetMaxLines(MAX_LINES)
	card.note = Label(card, "QuestFontNormalSmall", "faded")
	cards[n] = card
	return card
end

-- A card: its label, a tag such as "Shared", its button, and the answer or the question.
-- Returns its height.
local function DrawCard(n, row, x, y, width)
	local card = CardFrame(n)
	card:ClearAllPoints()
	card:SetPoint("TOPLEFT", child, "TOPLEFT", x, -y)
	card.label:SetText(row.label)
	card.tag:SetText(row.tag or "")
	card.tag:ClearAllPoints()
	card.tag:SetPoint("LEFT", card.label, "RIGHT", 6, 0)
	local room = Action(card, card, row.action)
	if room > 0 then
		card.button:ClearAllPoints()
		card.button:SetPoint("TOPRIGHT", card, "TOPRIGHT", -4, -3)
	end
	card.text:SetWidth(width - 2 * CARD_PADDING)
	card.text:SetText(row.text)
	Ink(card.text, row.empty and "faded" or "text")
	local height = math.min(card.text:GetStringHeight(), MAX_LINES * LINE_HEIGHT)
	card.note:SetText(row.note or "")
	card.note:ClearAllPoints()
	card.note:SetPoint("TOPLEFT", card, "TOPLEFT", CARD_PADDING, -(CARD_TOP + height + 2))
	if row.note then
		height = height + LINE_HEIGHT
	end
	height = CARD_TOP + height + CARD_PADDING
	card:SetSize(width, height)
	card:Show()
	return height
end

local function HideFrom(list, first)
	for n = first, #list do
		local slot = list[n]
		if slot.Hide then
			slot:Hide()
		else
			for _, part in pairs(slot) do
				part:Hide()
			end
		end
	end
end

-- Two narrow cards share a row. A wide card, or a line, closes the open row first.
local function DrawRows(rows)
	local y, counts = PADDING, { line = 0, card = 0, linked = 0 }
	local column, rowHeight = 0, 0
	local half = (Inner() - GAP) / 2
	local function CloseRow()
		if column > 0 then
			y = y + rowHeight + GAP
			column, rowHeight = 0, 0
		end
	end
	for _, row in ipairs(rows) do
		if row.kind == "card" and not row.wide then
			counts.card = counts.card + 1
			local x = PADDING + column * (half + GAP)
			rowHeight = math.max(rowHeight, DrawCard(counts.card, row, x, y, half))
			column = column + 1
			if column == 2 then
				CloseRow()
			end
		else
			CloseRow()
			if row.kind == "card" then
				counts.card = counts.card + 1
				y = y + DrawCard(counts.card, row, PADDING, y, Inner()) + GAP
			elseif row.kind == "linked" then
				counts.linked = counts.linked + 1
				y = y + DrawLinked(counts.linked, row, y) + GAP
			else
				counts.line = counts.line + 1
				y = y + DrawLine(counts.line, row, y) + GAP
			end
		end
	end
	CloseRow()
	HideFrom(cards, counts.card + 1)
	HideFrom(lines, counts.line + 1)
	HideFrom(linked, counts.linked + 1)
	return y
end

-- `spec` is { tabs = { { key, label, run } }, tab = the open key, rows = { ... } }. A row is
-- { kind = "line" | "heading", text, ink, action }, { kind = "card", label, tag, text, empty,
-- note, wide, action }, or { kind = "linked", label, text, note, empty }. Nil hides the cards.
function JournalCards.Draw(spec)
	if not spec then
		box:Hide()
		return
	end
	DrawTabs(spec)
	local height = DrawRows(spec.rows)
	child:SetSize(Inner() + 2 * PADDING, height)
	local bar = scroll.ScrollBar
	if type(bar) == "table" then
		bar:SetShown(height > paneHeight - TABS_HEIGHT - 2 * PADDING)
	end
	box:Show()
end
