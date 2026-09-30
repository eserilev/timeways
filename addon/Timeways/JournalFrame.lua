-- The journal window, in the look of the classic quest frame. The sizes and offsets come
-- from Blizzard_UIPanels_Game/Vanilla/QuestFrame.xml, so the art lines up as it does there.

local _, ns = ...

local JournalFrame = {}
ns.JournalFrame = JournalFrame

local NAME = "TimewaysJournalFrame"
local BULLET = 16

-- The scroll frame covers the parchment. The margins keep the text off its torn edge, and
-- the right one also keeps the buttons off it.
local SCROLL_WIDTH, SCROLL_HEIGHT = 300, 310
local MARGIN_LEFT, MARGIN_RIGHT, MARGIN_TOP = 12, 24, 12
local PAGE_WIDTH = SCROLL_WIDTH - MARGIN_LEFT - MARGIN_RIGHT

-- A hint and a help line fade, so they never read like a text of the player.
local STYLES = {
	heading = { font = "QuestTitleFont", indent = 0, gap = 14 },
	prose = { font = "QuestFont", indent = 0, gap = 6 },
	entry = { font = "QuestFont", indent = BULLET + 2, gap = 10, bullet = true },
	text = { font = "QuestFont", indent = BULLET + 2, gap = 2 },
	note = { font = "QuestFontNormalSmall", indent = 0, gap = 0 },
	hint = { font = "QuestFontNormalSmall", indent = BULLET + 2, gap = 2, faded = true },
	help = { font = "QuestFont", indent = 0, gap = 6, faded = true },
}

-- The tabs stand in one row on the dark band under the parchment, where QuestFrame.xml
-- puts Accept (x 23, y 72) and Decline (right edge at x 345). The band holds one row of
-- buttons only: a second row falls below the art. Each tab is as wide as its label, and the
-- tabs share the rest of the band.
local TABS_LEFT, TABS_RIGHT, TABS_BOTTOM = 23, 345, 73
local TAB_HEIGHT, TAB_GAP = 22, 2

-- The dark band between the title and the parchment holds the line on how to use the page.
local USAGE_LEFT, USAGE_WIDTH, USAGE_Y = 80, 250, -60

-- A button on a line uses the small font, so its label fits.
local ACTION_HEIGHT, ACTION_PADDING = 22, 20

local frame, scroll, page, usage
local strings, bullets, actions, tabs = {}, {}, {}, {}
local section = "chapters"

local function Art(file, width, height, point, x, y)
	local texture = frame:CreateTexture(nil, "BORDER")
	texture:SetTexture("Interface\\QuestFrame\\" .. file)
	texture:SetSize(width, height)
	texture:SetPoint(point, frame, point, x or 0, y or 0)
	return texture
end

local function BuildFrame()
	frame = CreateFrame("Frame", NAME, UIParent)
	frame:SetSize(384, 512)
	-- The art ends above and left of the frame edge. The rest lets clicks through to the world.
	frame:SetHitRectInsets(0, 30, 0, 70)
	frame:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 0, -104)
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

	-- The portrait sits under the border, which cuts it into a circle.
	local portrait = frame:CreateTexture(nil, "BACKGROUND")
	portrait:SetTexture("Interface\\QuestFrame\\UI-QuestLog-BookIcon")
	portrait:SetSize(60, 60)
	portrait:SetPoint("TOPLEFT", frame, "TOPLEFT", 7, -6)
	Art("UI-QuestGreeting-TopLeft", 256, 256, "TOPLEFT")
	Art("UI-QuestGreeting-TopRight", 128, 256, "TOPRIGHT")
	Art("UI-QuestGreeting-BotLeft", 256, 256, "BOTTOMLEFT")
	Art("UI-QuestGreeting-BotRight", 128, 256, "BOTTOMRIGHT")

	local title = frame:CreateFontString(nil, "ARTWORK", "GameFontHighlight")
	title:SetPoint("TOP", frame, "TOP", 0, -23)
	title:SetText("Timeways Journal")

	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("CENTER", frame, "TOPRIGHT", -42, -31)

	usage = frame:CreateFontString(nil, "ARTWORK", "GameFontNormalSmall")
	usage:SetPoint("LEFT", frame, "TOPLEFT", USAGE_LEFT, USAGE_Y)
	usage:SetWidth(USAGE_WIDTH)
end

local function BuildPage()
	scroll = CreateFrame("ScrollFrame", NAME .. "Scroll", frame, "UIPanelScrollFrameTemplate")
	scroll:SetSize(SCROLL_WIDTH, SCROLL_HEIGHT)
	scroll:SetPoint("TOPLEFT", frame, "TOPLEFT", 23, -81)
	page = CreateFrame("Frame", nil, scroll)
	page:SetSize(SCROLL_WIDTH, 1)
	scroll:SetScrollChild(page)
end

local function LabelWidth(button)
	return button:GetFontString():GetStringWidth()
end

local function Tab(name)
	local tab = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
	tab:SetHeight(TAB_HEIGHT)
	tab:SetNormalFontObject("GameFontNormalSmall")
	tab:SetHighlightFontObject("GameFontHighlightSmall")
	tab:SetDisabledFontObject("GameFontDisableSmall")
	tab:SetText(ns.Journal.TITLES[name])
	tab:SetScript("OnClick", function()
		JournalFrame.Open(name)
	end)
	return tab
end

local function BuildTabs()
	local labels = 0
	for _, name in ipairs(ns.Journal.SECTIONS) do
		tabs[name] = Tab(name)
		labels = labels + LabelWidth(tabs[name])
	end
	local count = #ns.Journal.SECTIONS
	local padding = (TABS_RIGHT - TABS_LEFT - (count - 1) * TAB_GAP - labels) / count
	local x = TABS_LEFT
	for _, name in ipairs(ns.Journal.SECTIONS) do
		local width = LabelWidth(tabs[name]) + padding
		tabs[name]:SetWidth(width)
		tabs[name]:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", x, TABS_BOTTOM)
		x = x + width + TAB_GAP
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
	if not actions[n] then
		actions[n] = CreateFrame("Button", nil, page, "UIPanelButtonTemplate")
		actions[n]:SetNormalFontObject("GameFontNormalSmall")
		actions[n]:SetHighlightFontObject("GameFontHighlightSmall")
		actions[n]:SetHeight(ACTION_HEIGHT)
	end
	return actions[n]
end

-- The button grows to its label, so a longer label never spills out.
local function FitAction(action, label)
	action:SetText(label)
	local fitted = math.max(LabelWidth(action) + ACTION_PADDING, 48)
	action:SetWidth(fitted)
	return fitted
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
		room = FitAction(action, line.action.label) + 6
		action:ClearAllPoints()
		action:SetPoint("TOPRIGHT", page, "TOPRIGHT", -MARGIN_RIGHT, -y)
		action:SetScript("OnClick", line.action.run)
	end
	local text = FontString(n)
	local ink = style.faded and ns.Ink.faded or ns.Ink.text
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
	return math.max(text:GetStringHeight() + nudge, line.action and ACTION_HEIGHT or 0)
end

-- The scroll bar shows only when the page is longer than the parchment.
local function ShowScrollBar(height)
	-- The template names its bar after the scroll frame.
	local bar = _G[NAME .. "ScrollScrollBar"]
	if type(bar) == "table" then
		bar:SetShown(height > SCROLL_HEIGHT)
	end
end

function JournalFrame.Refresh()
	if not frame or not frame:IsShown() then
		return
	end
	local lines = ns.Journal.Lines(section)
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
	usage:SetText(ns.Journal.USAGE[section])
	for name, tab in pairs(tabs) do
		tab:SetEnabled(name ~= section)
	end
end

local function ShowBook(shown)
	scroll:SetShown(shown)
	for _, tab in pairs(tabs) do
		tab:SetShown(shown)
	end
end

-- The editor takes the place of the page and the tabs until the player saves or cancels.
function JournalFrame.Edit(edit)
	JournalFrame.Open()
	ShowBook(false)
	ns.Editor.Open(frame, edit, function()
		ShowBook(true)
	end)
end

function JournalFrame.Open(name)
	if not frame then
		BuildFrame()
		BuildPage()
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
