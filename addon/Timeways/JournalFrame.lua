-- The journal window, in the look of the classic quest frame. The sizes and offsets come
-- from Blizzard_UIPanels_Game/Vanilla/QuestFrame.xml, so the art lines up as it does there.

local _, ns = ...

local JournalFrame = {}
ns.JournalFrame = JournalFrame

local NAME = "TimewaysJournalFrame"
local PAGE_WIDTH = 280
local INK = { 0.18, 0.12, 0.06 }
local BULLET = 16

local STYLES = {
	heading = { font = "QuestTitleFont", indent = 0, gap = 12 },
	prose = { font = "QuestFont", indent = 0, gap = 6 },
	entry = { font = "QuestFont", indent = BULLET + 2, gap = 8, bullet = true },
	text = { font = "QuestFont", indent = BULLET + 2, gap = 2 },
	note = { font = "QuestFontNormalSmall", indent = 0, gap = 0 },
}

local frame, scroll, page
local strings, bullets, tabs = {}, {}, {}
local section = ns.Journal.SECTIONS[1]

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
end

local function BuildPage()
	scroll = CreateFrame("ScrollFrame", NAME .. "Scroll", frame, "UIPanelScrollFrameTemplate")
	scroll:SetSize(300, 334)
	scroll:SetPoint("TOPLEFT", frame, "TOPLEFT", 23, -81)
	page = CreateFrame("Frame", nil, scroll)
	page:SetSize(PAGE_WIDTH, 1)
	scroll:SetScrollChild(page)
end

local function BuildTabs()
	for n, name in ipairs(ns.Journal.SECTIONS) do
		local tab = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
		tab:SetSize(78, 22)
		tab:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", 22 + (n - 1) * 82, 72)
		tab:SetText(ns.Journal.TITLES[name])
		tab:SetScript("OnClick", function()
			JournalFrame.Open(name)
		end)
		tabs[name] = tab
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

local function HideFrom(list, first)
	for n = first, #list do
		list[n]:Hide()
	end
end

local function DrawLine(n, line, y)
	local style = STYLES[line.style]
	local text = FontString(n)
	text:SetFontObject(style.font)
	text:SetTextColor(INK[1], INK[2], INK[3])
	text:SetJustifyH("LEFT")
	text:SetWidth(PAGE_WIDTH - style.indent)
	text:ClearAllPoints()
	text:SetPoint("TOPLEFT", page, "TOPLEFT", style.indent, -y)
	text:SetText(line.text)
	text:Show()
	local bullet = Bullet(n)
	bullet:ClearAllPoints()
	bullet:SetPoint("TOPLEFT", page, "TOPLEFT", 0, -y + 1)
	bullet:SetShown(style.bullet == true)
	return text:GetStringHeight()
end

function JournalFrame.Refresh()
	if not frame or not frame:IsShown() then
		return
	end
	local lines = ns.Journal.Lines(section)
	local y = 0
	for n, line in ipairs(lines) do
		y = y + (n > 1 and STYLES[line.style].gap or 0)
		y = y + DrawLine(n, line, y)
	end
	HideFrom(strings, #lines + 1)
	HideFrom(bullets, #lines + 1)
	page:SetHeight(math.max(y, 1))
	for name, tab in pairs(tabs) do
		tab:SetEnabled(name ~= section)
	end
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

function JournalFrame.Section()
	return section
end
