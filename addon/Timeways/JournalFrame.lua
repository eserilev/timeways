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
	-- As the headers of the quest frame of the game: "Description", "Rewards".
	section = { font = "QuestTitleFont", indent = 0, gap = 16, ink = "title" },
	prose = { font = "QuestFont", indent = 0, gap = 6, ink = "text" },
	entry = { font = "QuestFont", indent = BULLET + 2, gap = 10, ink = "text", bullet = true },
	-- A step of a quest that waits for another step.
	later = { font = "QuestFont", indent = BULLET + 2, gap = 10, ink = "faded", bullet = true },
	text = { font = "QuestFont", indent = BULLET + 2, gap = 2, ink = "text" },
	note = { font = "QuestFontNormalSmall", indent = 0, gap = 0, ink = "faded" },
	hint = { font = "QuestFontNormalSmall", indent = BULLET + 2, gap = 2, ink = "faded" },
	help = { font = "QuestFont", indent = 0, gap = 6, ink = "faded" },
	-- A row of a list of Knowledge: an icon, a name to click, and a faded word on the right.
	link = { font = "QuestFont", indent = BULLET + 2, gap = 4, ink = "text", icon = true },
	-- The counts of a place, each in a small box: "9 people", "11 quests".
	pills = { font = "QuestFontNormalSmall", indent = 0, gap = 8, ink = "title" },
	-- The inputs of JournalInputs.
	field = { indent = 0, gap = 8 },
	money = { indent = BULLET + 2, gap = 8 },
	slots = { indent = BULLET + 2, gap = 8 },
}

local TAB_PADDING, TAB_GAP = 16, 3
local BADGE_SIZE = 16
-- A button grows to its label, so a longer label never spills out.
local BUTTON_HEIGHT, BUTTON_PADDING, BUTTON_MIN = 22, 20, 48

-- The faded word on the right of a row, and the size of a box of a count.
local ROW_DETAIL_WIDTH = 110
local PILL_PADDING, PILL_HEIGHT, PILL_GAP = 6, 16, 4
-- The button over the map stands over its pins.
local MAP_BUTTON_LEVEL = 4

-- The thumbs of a rating stand right of the footer text (GAMEPLAY.md 3.2.2).
local THUMB_SIZE, THUMB_GAP = 18, 4
local THUMB_ICONS = {
	up = "Interface\\RaidFrame\\ReadyCheck-Ready",
	down = "Interface\\RaidFrame\\ReadyCheck-NotReady",
}
local THUMB_TIPS = { up = "Like this %s", down = "Dislike this %s" }

local frame, detail, scroll, page, crumb, footer, footerText, mapButton
local thumbs = {}
local strings, bullets, actions, buttons, tabs = {}, {}, {}, {}, {}
local details, pillTexts, pillBoxes = {}, {}, {}
-- The mouse is over a link of the page, so a press opens the link, not the editor.
local overLink = false
local section = "chapters"
-- The edit of the page that shows, or nil: a press on its text opens it for writing.
local pageEdit

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
	ns.JournalCards.Build(map, MAP_WIDTH, BODY_HEIGHT)

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
	page:SetScript("OnMouseDown", function()
		if pageEdit and not overLink then
			pageEdit()
		end
	end)
	page:SetHyperlinksEnabled(true)
	page:SetScript("OnHyperlinkEnter", function()
		overLink = true
	end)
	page:SetScript("OnHyperlinkLeave", function()
		overLink = false
	end)
	page:SetScript("OnHyperlinkClick", function(_, link)
		ns.JournalLinks.Open(link)
	end)
	scroll:SetScrollChild(page)
	-- Over the map: the way up from a page of a place, as "Eastern Kingdoms".
	mapButton = JournalFrame.SmallButton(map)
	ns.MapPane.Stack(mapButton, map, MAP_BUTTON_LEVEL)
	mapButton:SetPoint("TOPLEFT", map, "TOPLEFT", 8, -8)
	mapButton:Hide()
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

local function Thumb(rating)
	local thumb = CreateFrame("Button", nil, footer)
	thumb:SetSize(THUMB_SIZE, THUMB_SIZE)
	thumb.icon = thumb:CreateTexture(nil, "ARTWORK")
	thumb.icon:SetAllPoints(thumb)
	thumb.icon:SetTexture(THUMB_ICONS[rating])
	thumb:SetScript("OnLeave", function()
		GameTooltip:Hide()
	end)
	return thumb
end

local function BuildThumbs()
	thumbs.up = Thumb("up")
	thumbs.up:SetPoint("LEFT", footerText, "RIGHT", 8, 0)
	thumbs.down = Thumb("down")
	thumbs.down:SetPoint("LEFT", thumbs.up, "RIGHT", THUMB_GAP, 0)
end

-- A thumb shows filled while it holds the rating of the page, and faded otherwise.
local function DrawThumb(rating, pageThumbs)
	local thumb = thumbs[rating]
	thumb:SetShown(pageThumbs ~= nil)
	if not pageThumbs then
		return
	end
	local filled = pageThumbs.rating == rating
	thumb.icon:SetDesaturated(not filled)
	thumb.icon:SetAlpha(filled and 1 or 0.6)
	local run = rating == "up" and pageThumbs.like or pageThumbs.dislike
	thumb:SetScript("OnClick", function()
		run(thumb)
	end)
	thumb:SetScript("OnEnter", function()
		GameTooltip:SetOwner(thumb, "ANCHOR_TOP")
		GameTooltip:SetText(THUMB_TIPS[rating]:format(pageThumbs.noun))
		GameTooltip:Show()
	end)
end

local function LabelWidth(button)
	return button:GetFontString():GetStringWidth()
end

-- The small red button of the book, for the tabs and the buttons of a page.
function JournalFrame.SmallButton(parent)
	local button = CreateFrame("Button", nil, parent, "UIPanelButtonTemplate")
	button:SetHeight(BUTTON_HEIGHT)
	button:SetNormalFontObject("GameFontNormalSmall")
	button:SetHighlightFontObject("GameFontHighlightSmall")
	button:SetDisabledFontObject("GameFontDisableSmall")
	return button
end

-- A button grows to its label. Returns its width.
function JournalFrame.FitButton(button, label)
	button:SetText(label)
	local width = math.max(LabelWidth(button) + BUTTON_PADDING, BUTTON_MIN)
	button:SetWidth(width)
	return width
end

local SmallButton = JournalFrame.SmallButton
local FitButton = JournalFrame.FitButton

-- A count on the corner of a tab, such as the stories that wait for an answer.
local function Badge(tab)
	local disc = tab:CreateTexture(nil, "OVERLAY")
	disc:SetSize(BADGE_SIZE, BADGE_SIZE)
	disc:SetPoint("CENTER", tab, "TOPRIGHT", -2, -2)
	disc:SetColorTexture(unpack(ns.Ink.badge))
	local count = tab:CreateFontString(nil, "OVERLAY", "GameFontHighlightSmall")
	count:SetPoint("CENTER", disc, "CENTER", 0, 0)
	return { disc = disc, count = count }
end

-- The tabs stand in one row at the right end of the bar, as the game puts the tabs of a log.
local function Tab(name)
	local tab = SmallButton(frame)
	tab:SetText(ns.Journal.TITLES[name])
	tab:SetScript("OnClick", function()
		JournalFrame.Open(name)
	end)
	tab.badge = Badge(tab)
	return tab
end

local function DrawBadges()
	for name, tab in pairs(tabs) do
		local count = ns.Journal.Badge(name)
		tab.badge.count:SetText(count and tostring(count) or "")
		tab.badge.count:SetShown(count ~= nil)
		tab.badge.disc:SetShown(count ~= nil)
	end
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

local function Detail(n)
	if not details[n] then
		details[n] = page:CreateFontString(nil, "ARTWORK")
		details[n]:SetJustifyH("RIGHT")
		details[n]:SetWordWrap(false)
	end
	return details[n]
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

-- Returns the room that the button of the line takes.
local function DrawAction(n, line, y)
	local action = Action(n)
	action:SetShown(line.action ~= nil)
	if not line.action then
		return 0
	end
	action:ClearAllPoints()
	action:SetPoint("TOPRIGHT", page, "TOPLEFT", MARGIN_LEFT + PAGE_WIDTH, -y)
	action:SetScript("OnClick", line.action.run)
	action:SetEnabled(not line.action.disabled)
	return FitButton(action, line.action.label) + 6
end

local function DrawInput(n, line, y, room)
	FontString(n):Hide()
	Bullet(n):Hide()
	local indent = STYLES[line.style].indent
	local height = ns.JournalInputs.Draw(page, line, MARGIN_LEFT + indent, PAGE_WIDTH - indent - room, y)
	return math.max(height, line.action and BUTTON_HEIGHT or 0)
end

local function PillText(n)
	pillTexts[n] = pillTexts[n] or page:CreateFontString(nil, "ARTWORK")
	return pillTexts[n]
end

local function PillBox(n)
	if not pillBoxes[n] then
		pillBoxes[n] = page:CreateTexture(nil, "BACKGROUND")
		pillBoxes[n]:SetColorTexture(unpack(ns.Ink.selectedOnParchment))
	end
	return pillBoxes[n]
end

-- The boxes of the counts in a row, which wraps at the edge of the page. Returns the next
-- box to use and the height of the rows.
local function DrawPills(line, y, first)
	local style = STYLES.pills
	local ink = ns.Ink[style.ink]
	local x, rows, used = 0, 1, first
	for _, count in ipairs(line.pills) do
		local text, box = PillText(used), PillBox(used)
		text:SetFontObject(style.font)
		text:SetTextColor(ink[1], ink[2], ink[3])
		text:SetText(count)
		local width = text:GetStringWidth() + 2 * PILL_PADDING
		if x > 0 and x + width > PAGE_WIDTH then
			x, rows = 0, rows + 1
		end
		local top = -y - (rows - 1) * (PILL_HEIGHT + PILL_GAP)
		box:ClearAllPoints()
		box:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN_LEFT + x, top)
		box:SetSize(width, PILL_HEIGHT)
		box:Show()
		text:ClearAllPoints()
		text:SetPoint("CENTER", box, "CENTER", 0, 0)
		text:Show()
		x, used = x + width + PILL_GAP, used + 1
	end
	return used, rows * PILL_HEIGHT + (rows - 1) * PILL_GAP
end

-- The icon of a row of Knowledge stands where an entry has its bullet.
local function DrawMark(n, line, style, y, nudge)
	local bullet = Bullet(n)
	bullet:ClearAllPoints()
	bullet:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN_LEFT, -y - nudge + 1)
	bullet:SetTexture(line.icon or "Interface\\QuestFrame\\UI-Quest-BulletPoint")
	bullet:SetShown(style.bullet == true or (style.icon == true and line.icon ~= nil))
end

-- Returns the room that the faded word on the right takes.
local function DrawDetail(n, line, y)
	local text = Detail(n)
	text:SetShown(line.detail ~= nil)
	if not line.detail then
		return 0
	end
	local ink = ns.Ink.faded
	text:SetFontObject("QuestFontNormalSmall")
	text:SetTextColor(ink[1], ink[2], ink[3])
	text:SetWidth(ROW_DETAIL_WIDTH)
	text:ClearAllPoints()
	text:SetPoint("TOPRIGHT", page, "TOPLEFT", MARGIN_LEFT + PAGE_WIDTH, -y - 1)
	text:SetText(line.detail)
	return ROW_DETAIL_WIDTH + 6
end

local function DrawLine(n, line, y)
	local style = STYLES[line.style]
	local room = DrawAction(n, line, y) + DrawDetail(n, line, y)
	if ns.JournalInputs.Holds(line) then
		return DrawInput(n, line, y, room)
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
	-- A line can stop after a few lines, and the game ends it with "...".
	text:SetMaxLines(line.maxLines or 0)
	text:SetText(line.text)
	text:Show()
	DrawMark(n, line, style, y, nudge)
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
	ns.JournalInputs.Begin()
	local y, pills = MARGIN_TOP, 1
	for n, line in ipairs(lines) do
		y = y + (n > 1 and STYLES[line.style].gap or 0)
		if line.style == "pills" then
			FontString(n):Hide()
			Bullet(n):Hide()
			Action(n):Hide()
			Detail(n):Hide()
			local height
			pills, height = DrawPills(line, y, pills)
			y = y + height
		else
			y = y + DrawLine(n, line, y)
		end
	end
	ns.JournalInputs.Finish()
	HideFrom(strings, #lines + 1)
	HideFrom(bullets, #lines + 1)
	HideFrom(actions, #lines + 1)
	HideFrom(details, #lines + 1)
	HideFrom(pillTexts, pills)
	HideFrom(pillBoxes, pills)
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

local function DrawMapButton(button)
	mapButton:SetShown(button ~= nil)
	if button then
		FitButton(mapButton, button.label)
		mapButton:SetScript("OnClick", button.run)
	end
end

-- Returns the name of the map that the left half shows, if any. The parchment of a sheet
-- covers the map, so a sheet leaves the map as it is.
local function DrawSide(journalPage, writing)
	local skin = journalPage.side
	-- While the player writes, the list and the cards hide, so a click cannot open another
	-- item.
	ns.JournalList.Draw(not writing and journalPage.list or nil, journalPage.selected, skin)
	ns.JournalCards.Draw(not writing and journalPage.cards or nil)
	if skin == "sheet" or skin == "cards" then
		return nil
	end
	local place = ns.MapPane.Show(journalPage.zone, journalPage.map, journalPage.pins)
	ns.MapPane.ShowVisited(place and ns.Journal.VisitedIn(place) or {})
	DrawMapButton(not writing and journalPage.mapButton or nil)
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
	pageEdit = journalPage.edit
	page:EnableMouse(pageEdit ~= nil)
	DrawLines(journalPage.lines)
	DrawButtons(journalPage.buttons)
	footerText:SetText(journalPage.footer)
	DrawThumb("up", journalPage.thumbs)
	DrawThumb("down", journalPage.thumbs)
	local place = DrawSide(journalPage, writing)
	if journalPage.side ~= "map" then
		mapButton:Hide()
	end
	crumb:SetText(Path(journalPage.crumb or place))
	for name, tab in pairs(tabs) do
		tab:SetEnabled(name ~= section)
	end
	DrawBadges()
	ShowBook(not writing)
end

function JournalFrame.Edit(edit)
	JournalFrame.Open()
	ns.Editor.Open(detail, edit, JournalFrame.Refresh)
	JournalFrame.Refresh()
end

-- Opens an item of the list of the open section. An item with boxes, such as a new quest,
-- opens with the cursor in its first empty box.
function JournalFrame.Select(key)
	ns.Journal.Select(section, key)
	if scroll then
		scroll:SetVerticalScroll(0)
	end
	JournalFrame.Refresh()
	if frame and frame:IsShown() then
		ns.JournalInputs.FocusFirstEmpty()
	end
end

-- The page of the open section turned to another one, read from its top.
function JournalFrame.Turn()
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
		BuildThumbs()
		BuildTabs()
	end
	if name and name ~= section then
		section = name
		scroll:SetVerticalScroll(0)
		ns.Journal.Opened(section)
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
