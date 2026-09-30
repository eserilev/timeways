-- The journal window, in the look of the game's Map and Quest Log: the path and the tabs on
-- top, the map on the left, the parchment on the right, and the buttons at the bottom.

local _, ns = ...

local JournalFrame = {}
ns.JournalFrame = JournalFrame

local NAME = "TimewaysJournalFrame"

-- The frame fits a screen of 1024 by 768 UI units. The map keeps about the 3:2 shape of the
-- map art, so it loses little at its edges.
local WIDTH, HEIGHT = 980, 520
local EDGE = 12
local BAR_TOP, BAR_HEIGHT = -34, 30
local BODY_TOP, BODY_BOTTOM = -68, 48
local BODY_HEIGHT = HEIGHT + BODY_TOP - BODY_BOTTOM
local MAP_WIDTH, GAP = 580, 6
local DETAIL_LEFT = EDGE + MAP_WIDTH + GAP
local DETAIL_WIDTH = WIDTH - DETAIL_LEFT - EDGE
local FOOTER_BOTTOM, FOOTER_HEIGHT = 12, 28

-- The scroll bar of the template stands right of the scroll frame, on the parchment.
local SCROLL_BAR = 26
local SCROLL_WIDTH, SCROLL_HEIGHT = DETAIL_WIDTH - SCROLL_BAR, BODY_HEIGHT - 16
local MARGIN_LEFT, MARGIN_RIGHT, MARGIN_TOP = 16, 8, 12
local PAGE_WIDTH = SCROLL_WIDTH - MARGIN_LEFT - MARGIN_RIGHT
local BULLET = 16

-- A hint and a help line fade, so they never read like a text of the player.
local STYLES = {
	heading = { font = "QuestTitleFont", indent = 0, gap = 14, ink = "title" },
	section = { font = "GameFontNormal", indent = 0, gap = 14, ink = "title" },
	prose = { font = "QuestFont", indent = 0, gap = 6, ink = "text" },
	entry = { font = "QuestFont", indent = BULLET + 2, gap = 10, ink = "text", bullet = true },
	text = { font = "QuestFont", indent = BULLET + 2, gap = 2, ink = "text" },
	note = { font = "QuestFontNormalSmall", indent = 0, gap = 0, ink = "faded" },
	hint = { font = "QuestFontNormalSmall", indent = BULLET + 2, gap = 2, ink = "faded" },
	help = { font = "QuestFont", indent = 0, gap = 6, ink = "faded" },
}

local TAB_PADDING, TAB_GAP = 16, 3
-- A button grows to its label, so a longer label never spills out.
local BUTTON_HEIGHT, BUTTON_PADDING, BUTTON_MIN = 22, 20, 48

local frame, detail, scroll, page, crumb, footer, footerText
local strings, bullets, actions, buttons, tabs = {}, {}, {}, {}, {}
local section = "chapters"

local function Band(top, height)
	local band = frame:CreateTexture(nil, "BORDER")
	band:SetColorTexture(0.05, 0.035, 0.025, 0.9)
	band:SetPoint("TOPLEFT", frame, "TOPLEFT", EDGE, top)
	band:SetSize(WIDTH - 2 * EDGE, height)
end

local function BuildFrame()
	frame = CreateFrame("Frame", NAME, UIParent, "BackdropTemplate")
	frame:SetSize(WIDTH, HEIGHT)
	frame:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
	frame:SetBackdrop({
		bgFile = "Interface\\FrameGeneral\\UI-Background-Rock",
		edgeFile = "Interface\\DialogFrame\\UI-DialogBox-Border",
		tile = true,
		tileSize = 256,
		edgeSize = 24,
		insets = { left = 6, right = 6, top = 6, bottom = 6 },
	})
	frame:SetFrameStrata("HIGH")
	frame:SetToplevel(true)
	frame:EnableMouse(true)
	frame:SetMovable(true)
	frame:RegisterForDrag("LeftButton")
	frame:SetScript("OnDragStart", frame.StartMoving)
	frame:SetScript("OnDragStop", frame.StopMovingOrSizing)
	frame:SetScript("OnShow", function()
		ns.Journal.Request(0)
	end)
	frame:Hide()
	-- Escape closes each frame in this list.
	table.insert(UISpecialFrames, NAME)

	local title = frame:CreateFontString(nil, "ARTWORK", "GameFontNormal")
	title:SetPoint("TOP", frame, "TOP", 0, -14)
	title:SetText("Timeways Journal")
	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("TOPRIGHT", frame, "TOPRIGHT", -4, -4)

	Band(BAR_TOP, BAR_HEIGHT)
	crumb = frame:CreateFontString(nil, "ARTWORK", "GameFontNormal")
	crumb:SetPoint("LEFT", frame, "TOPLEFT", EDGE + 10, BAR_TOP - BAR_HEIGHT / 2)
	crumb:SetJustifyH("LEFT")
end

local function BuildSides()
	local map = ns.MapPane.Build(frame, MAP_WIDTH, BODY_HEIGHT)
	map:SetPoint("TOPLEFT", frame, "TOPLEFT", EDGE, BODY_TOP)
	ns.JournalList.Build(map, MAP_WIDTH, BODY_HEIGHT)

	detail = CreateFrame("Frame", nil, frame)
	detail:SetSize(DETAIL_WIDTH, BODY_HEIGHT)
	detail:SetPoint("TOPLEFT", frame, "TOPLEFT", DETAIL_LEFT, BODY_TOP)
	local parchment = detail:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(detail)
	parchment:SetAtlas("QuestBG-Parchment")
	scroll = CreateFrame("ScrollFrame", NAME .. "Scroll", detail, "UIPanelScrollFrameTemplate")
	scroll:SetSize(SCROLL_WIDTH, SCROLL_HEIGHT)
	scroll:SetPoint("TOPLEFT", detail, "TOPLEFT", 0, -8)
	page = CreateFrame("Frame", nil, scroll)
	page:SetSize(SCROLL_WIDTH, 1)
	scroll:SetScrollChild(page)
end

local function BuildFooter()
	Band(-(HEIGHT - FOOTER_BOTTOM - FOOTER_HEIGHT), FOOTER_HEIGHT)
	footer = CreateFrame("Frame", nil, frame)
	footer:SetSize(WIDTH - 2 * EDGE, FOOTER_HEIGHT)
	footer:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", EDGE, FOOTER_BOTTOM)
	footerText = frame:CreateFontString(nil, "ARTWORK", "GameFontHighlightSmall")
	footerText:SetPoint("LEFT", footer, "LEFT", 10, 0)
	footerText:SetJustifyH("LEFT")
end

local function LabelWidth(button)
	return button:GetFontString():GetStringWidth()
end

local function SmallButton(parent)
	local button = CreateFrame("Button", nil, parent, "UIPanelButtonTemplate")
	button:SetHeight(BUTTON_HEIGHT)
	button:SetNormalFontObject("GameFontNormalSmall")
	button:SetHighlightFontObject("GameFontHighlightSmall")
	button:SetDisabledFontObject("GameFontDisableSmall")
	return button
end

local function FitButton(button, label)
	button:SetText(label)
	local width = math.max(LabelWidth(button) + BUTTON_PADDING, BUTTON_MIN)
	button:SetWidth(width)
	return width
end

-- The tabs stand in one row at the right end of the bar, as the game puts the tabs of a log.
local function Tab(name)
	local tab = SmallButton(frame)
	tab:SetText(ns.Journal.TITLES[name])
	tab:SetScript("OnClick", function()
		JournalFrame.Open(name)
	end)
	return tab
end

-- Each tab is as wide as its label, and the row ends at the right end of the bar.
local function BuildTabs()
	local widths, row = {}, -TAB_GAP
	for _, name in ipairs(ns.Journal.SECTIONS) do
		tabs[name] = Tab(name)
		widths[name] = LabelWidth(tabs[name]) + TAB_PADDING
		tabs[name]:SetWidth(widths[name])
		row = row + widths[name] + TAB_GAP
	end
	local x = WIDTH - EDGE - 6 - row
	for _, name in ipairs(ns.Journal.SECTIONS) do
		tabs[name]:SetPoint("TOPLEFT", frame, "TOPLEFT", x, BAR_TOP - (BAR_HEIGHT - BUTTON_HEIGHT) / 2)
		x = x + widths[name] + TAB_GAP
	end
end

local function FontString(n)
	strings[n] = strings[n] or page:CreateFontString(nil, "ARTWORK")
	return strings[n]
end

local function Bullet(n)
	if not bullets[n] then
		bullets[n] = page:CreateTexture(nil, "ARTWORK")
		bullets[n]:SetTexture("Interface\\QuestFrame\\UI-Quest-BulletPoint")
		bullets[n]:SetSize(BULLET, BULLET)
	end
	return bullets[n]
end

local function Action(n)
	actions[n] = actions[n] or SmallButton(page)
	return actions[n]
end

local function HideFrom(list, first)
	for n = first, #list do
		list[n]:Hide()
	end
end

local function DrawLine(n, line, y)
	local style = STYLES[line.style]
	local action = Action(n)
	action:SetShown(line.action ~= nil)
	local room = 0
	if line.action then
		room = FitButton(action, line.action.label) + 6
		action:ClearAllPoints()
		action:SetPoint("TOPRIGHT", page, "TOPLEFT", MARGIN_LEFT + PAGE_WIDTH, -y)
		action:SetScript("OnClick", line.action.run)
	end
	local text = FontString(n)
	local ink = ns.Ink[style.ink]
	text:SetFontObject(style.font)
	text:SetTextColor(ink[1], ink[2], ink[3])
	text:SetJustifyH("LEFT")
	text:SetWidth(PAGE_WIDTH - style.indent - room)
	text:ClearAllPoints()
	-- A line with a button sits in the middle of the button's height.
	local nudge = line.action and 4 or 0
	text:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN_LEFT + style.indent, -y - nudge)
	text:SetText(line.text)
	text:Show()
	local bullet = Bullet(n)
	bullet:ClearAllPoints()
	bullet:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN_LEFT, -y - nudge + 1)
	bullet:SetShown(style.bullet == true)
	return math.max(text:GetStringHeight() + nudge, line.action and BUTTON_HEIGHT or 0)
end

-- The scroll bar shows only when the page is longer than the parchment.
local function ShowScrollBar(height)
	local bar = scroll.ScrollBar
	if type(bar) == "table" then
		bar:SetShown(height > SCROLL_HEIGHT)
	end
end

local function DrawLines(lines)
	local y = MARGIN_TOP
	for n, line in ipairs(lines) do
		y = y + (n > 1 and STYLES[line.style].gap or 0)
		y = y + DrawLine(n, line, y)
	end
	HideFrom(strings, #lines + 1)
	HideFrom(bullets, #lines + 1)
	HideFrom(actions, #lines + 1)
	page:SetHeight(y + MARGIN_TOP)
	ShowScrollBar(y + MARGIN_TOP)
end

-- The buttons stand at the right end of the footer, the first one leftmost.
local function DrawButtons(list)
	local x = -6
	for n = #list, 1, -1 do
		buttons[n] = buttons[n] or SmallButton(footer)
		local button = buttons[n]
		local width = FitButton(button, list[n].label)
		button:ClearAllPoints()
		button:SetPoint("RIGHT", footer, "RIGHT", x, 0)
		button:SetEnabled(not list[n].disabled)
		button:SetScript("OnClick", list[n].run)
		button:Show()
		x = x - width - TAB_GAP
	end
	HideFrom(buttons, #list + 1)
end

-- Returns the name of the map that the left half shows, if any. The parchment of a sheet
-- covers the map, so a sheet leaves the map as it is.
local function DrawSide(journalPage, writing)
	local skin = journalPage.side
	-- While the player writes, the list hides, so a click cannot open another item.
	ns.JournalList.Draw(not writing and journalPage.list or nil, journalPage.selected, skin)
	if skin == "sheet" then
		return nil
	end
	local place = ns.MapPane.Show(journalPage.zone, journalPage.map, journalPage.pins)
	ns.MapPane.ShowVisited(place and ns.Journal.VisitedIn(place) or {})
	return place
end

local function Path(last)
	local path = "Journal  >  " .. ns.Journal.TITLES[section]
	return last and (path .. "  >  " .. last) or path
end

-- The editor takes the place of the page, the tabs, and the buttons until the player saves
-- or cancels.
local function ShowBook(shown)
	scroll:SetShown(shown)
	footer:SetShown(shown)
	for _, tab in pairs(tabs) do
		tab:SetShown(shown)
	end
end

function JournalFrame.Refresh()
	if not frame or not frame:IsShown() then
		return
	end
	local journalPage = ns.Journal.Page(section)
	local writing = ns.Editor.IsShown()
	DrawLines(journalPage.lines)
	DrawButtons(journalPage.buttons)
	footerText:SetText(journalPage.footer)
	local place = DrawSide(journalPage, writing)
	crumb:SetText(Path(journalPage.crumb or place))
	for name, tab in pairs(tabs) do
		tab:SetEnabled(name ~= section)
	end
	ShowBook(not writing)
end

function JournalFrame.Edit(edit)
	JournalFrame.Open()
	ns.Editor.Open(detail, edit, JournalFrame.Refresh)
	JournalFrame.Refresh()
end

-- Opens an item of the list of the open section.
function JournalFrame.Select(key)
	ns.Journal.Select(section, key)
	if scroll then
		scroll:SetVerticalScroll(0)
	end
	JournalFrame.Refresh()
end

function JournalFrame.Open(name)
	if not frame then
		BuildFrame()
		BuildSides()
		BuildFooter()
		BuildTabs()
	end
	if name and name ~= section then
		section = name
		scroll:SetVerticalScroll(0)
	end
	frame:Show()
	JournalFrame.Refresh()
end

function JournalFrame.Toggle()
	if frame and frame:IsShown() then
		frame:Hide()
	else
		JournalFrame.Open()
	end
end

function JournalFrame.IsShown()
	return frame ~= nil and frame:IsShown()
end

function JournalFrame.Section()
	return section
end
