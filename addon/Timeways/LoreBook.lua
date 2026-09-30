-- The lore book: a small window with the answers of `/lore` (GAMEPLAY.md 3.1), in the look
-- of the journal. The question is the heading, and the answer is the page.

local _, ns = ...

local LoreBook = {}
ns.LoreBook = LoreBook

local NAME = "TimewaysLoreFrame"
local WIDTH, HEIGHT = 480, 420
local EDGE = 12
local SHEET_TOP, SHEET_BOTTOM = -34, 44
-- The scroll bar of the template stands right of the scroll frame, on the parchment.
local SCROLL_BAR = 26
local MARGIN = 16
local GAP = 12
local BUTTON_WIDTH, BUTTON_HEIGHT = 90, 22

local STATUS = {
	asking = "Asking...",
	failed = "No answer came back. Try asking again.",
}

local frame, scroll, page, heading, body, counter, previousButton, nextButton
local scrollHeight
-- The entry of `Lore.Entries()` that the book shows.
local shown

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
	frame:Hide()
	-- Escape closes each frame in this list.
	table.insert(UISpecialFrames, NAME)

	local title = frame:CreateFontString(nil, "ARTWORK", "GameFontNormal")
	title:SetPoint("TOP", frame, "TOP", 0, -14)
	title:SetText("Lore")
	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("TOPRIGHT", frame, "TOPRIGHT", -4, -4)
end

local function Text(font)
	local text = page:CreateFontString(nil, "ARTWORK", font)
	text:SetJustifyH("LEFT")
	text:SetWidth(WIDTH - 2 * EDGE - SCROLL_BAR - 2 * MARGIN)
	return text
end

local function BuildPage()
	local sheet = CreateFrame("Frame", nil, frame)
	sheet:SetPoint("TOPLEFT", frame, "TOPLEFT", EDGE, SHEET_TOP)
	sheet:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", -EDGE, SHEET_BOTTOM)
	local parchment = sheet:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(sheet)
	parchment:SetAtlas("QuestBG-Parchment")
	scrollHeight = HEIGHT + SHEET_TOP - SHEET_BOTTOM - 16
	scroll = CreateFrame("ScrollFrame", NAME .. "Scroll", sheet, "UIPanelScrollFrameTemplate")
	scroll:SetSize(WIDTH - 2 * EDGE - SCROLL_BAR, scrollHeight)
	scroll:SetPoint("TOPLEFT", sheet, "TOPLEFT", 0, -8)
	page = CreateFrame("Frame", nil, scroll)
	page:SetSize(WIDTH - 2 * EDGE - SCROLL_BAR, 1)
	scroll:SetScrollChild(page)
	heading = Text("QuestTitleFont")
	heading:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN, -MARGIN)
	heading:SetTextColor(unpack(ns.Ink.title))
	body = Text("QuestFont")
end

local function Button(label, x, run)
	local button = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
	button:SetSize(BUTTON_WIDTH, BUTTON_HEIGHT)
	button:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", x, 14)
	button:SetText(label)
	button:SetScript("OnClick", run)
	return button
end

local function IndexOf(entry)
	for n, kept in ipairs(ns.Lore.Entries()) do
		if kept == entry then
			return n
		end
	end
end

local function Step(by)
	return function()
		local entries = ns.Lore.Entries()
		shown = entries[(IndexOf(shown) or #entries) + by] or shown
		LoreBook.Refresh()
	end
end

local function BuildFooter()
	counter = frame:CreateFontString(nil, "ARTWORK", "GameFontHighlightSmall")
	counter:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", EDGE + 10, 20)
	Button("Close", -EDGE - 4, function()
		frame:Hide()
	end)
	nextButton = Button("Next", -EDGE - 4 - (BUTTON_WIDTH + 4), Step(1))
	previousButton = Button("Previous", -EDGE - 4 - 2 * (BUTTON_WIDTH + 4), Step(-1))
end

local function BodyText(entry)
	return STATUS[entry.state] or entry.text or "Nobody here knows."
end

local function DrawEntry(entry)
	heading:SetText(entry.question and ns.Plain(entry.question) or "Your answer")
	body:ClearAllPoints()
	body:SetPoint("TOPLEFT", heading, "BOTTOMLEFT", 0, -GAP)
	body:SetText(BodyText(entry))
	local ink = entry.state == "answered" and entry.text and ns.Ink.text or ns.Ink.faded
	body:SetTextColor(unpack(ink))
	local height = MARGIN + heading:GetStringHeight() + GAP + body:GetStringHeight() + MARGIN
	page:SetHeight(height)
	local bar = scroll.ScrollBar
	if type(bar) == "table" then
		bar:SetShown(height > scrollHeight)
	end
end

function LoreBook.Refresh()
	if not frame or not frame:IsShown() then
		return
	end
	local entries = ns.Lore.Entries()
	local index = IndexOf(shown) or #entries
	shown = entries[index]
	if not shown then
		return
	end
	DrawEntry(shown)
	counter:SetText(string.format("%d of %d", index, #entries))
	previousButton:SetEnabled(index > 1)
	nextButton:SetEnabled(index < #entries)
end

-- Opens the book on this entry of `Lore.Entries()`, at the top of its page.
function LoreBook.Open(entry)
	if not frame then
		BuildFrame()
		BuildPage()
		BuildFooter()
	end
	shown = entry
	scroll:SetVerticalScroll(0)
	frame:Show()
	LoreBook.Refresh()
end

function LoreBook.IsShown()
	return frame ~= nil and frame:IsShown()
end
