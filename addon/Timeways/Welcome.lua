-- The setup window: how to install the desktop app, for a player whose game can't reach it.
-- CurseForge ships only the addon, so most players see this window at their first login.

local _, ns = ...

local Welcome = {}
ns.Welcome = Welcome

local NAME = "TimewaysWelcomeFrame"
local WIDTH, HEIGHT = 540, 360
local EDGE = 12
local SHEET_TOP, SHEET_BOTTOM = -34, 44
local INSET = 18
local BOX_HEIGHT = 24

-- The desktop app replies within seconds of login. A minute leaves room for a slow computer.
local WAIT_SECONDS = 60

-- TODO: These lines install Gnomish Relay, and its setup finds Timeways. When relay setup
-- also downloads timeways-story and timeways-pack from the Timeways release, check that
-- these lines still install both, and change them if the relay adds a Timeways command.
Welcome.COMMANDS = {
	windows = "irm https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.ps1 | iex",
	unix = "curl -fsSL https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.sh | sh",
}

local HEADINGS = {
	setup = "Install the desktop app",
	files = "Some Timeways files are missing",
	offline = "Can't reach the desktop app",
}

local frame, heading, restartHint
local checked = false

-- Why the game can't reach the desktop app: "setup", "files", "offline", or nil.
function Welcome.Reason()
	-- The key comes from the desktop app, so no key means no app.
	if not ns.key then
		return "setup"
	elseif ns.Messages.Problem() == "missing" then
		return "files"
	elseif not ns.Messages.Online() then
		return "offline"
	end
end

local function Text(parent, font, ink, top)
	local text = parent:CreateFontString(nil, "ARTWORK", font)
	text:SetPoint("TOPLEFT", parent, "TOPLEFT", INSET, top)
	text:SetPoint("TOPRIGHT", parent, "TOPRIGHT", -INSET, top)
	text:SetJustifyH("LEFT")
	text:SetTextColor(ink[1], ink[2], ink[3])
	return text
end

-- WoW can't write to the clipboard, so the box selects its whole line for Ctrl+C.
local function CommandBox(parent, command, top)
	local box = CreateFrame("EditBox", nil, parent, "InputBoxTemplate")
	box:SetAutoFocus(false)
	box:SetFontObject("ChatFontNormal")
	box:SetPoint("TOPLEFT", parent, "TOPLEFT", INSET + 6, top)
	box:SetPoint("TOPRIGHT", parent, "TOPRIGHT", -INSET, top)
	box:SetHeight(BOX_HEIGHT)
	box:SetText(command)
	box:SetCursorPosition(0)
	box:SetScript("OnEditFocusGained", box.HighlightText)
	-- A key press puts the line back, so the player always copies the real command.
	box:SetScript("OnTextChanged", function(self, typed)
		if typed then
			self:SetText(command)
			self:HighlightText()
		end
	end)
	box:SetScript("OnEscapePressed", box.ClearFocus)
	box:SetScript("OnEnterPressed", box.ClearFocus)
	return box
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
	frame:SetFrameStrata("DIALOG")
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
	title:SetText("Timeways Setup")
	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("TOPRIGHT", frame, "TOPRIGHT", -4, -4)
end

local function BuildSheet()
	local sheet = CreateFrame("Frame", nil, frame)
	sheet:SetPoint("TOPLEFT", frame, "TOPLEFT", EDGE, SHEET_TOP)
	sheet:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", -EDGE, SHEET_BOTTOM)
	local parchment = sheet:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(sheet)
	parchment:SetAtlas("QuestBG-Parchment")

	heading = Text(sheet, "QuestTitleFont", ns.Ink.title, -16)
	Text(sheet, "QuestFont", ns.Ink.text, -48):SetText(
		"Timeways needs its desktop app, Gnomish Relay, because addons can't save your story or talk to an AI on their own."
	)
	Text(sheet, "QuestFont", ns.Ink.text, -96):SetText("Windows (PowerShell):")
	CommandBox(sheet, Welcome.COMMANDS.windows, -114)
	Text(sheet, "QuestFont", ns.Ink.text, -146):SetText("macOS or Linux (Terminal):")
	CommandBox(sheet, Welcome.COMMANDS.unix, -164)
	Text(sheet, "QuestFontNormalSmall", ns.Ink.faded, -194):SetText(
		"Click a line and press Ctrl+C to copy it (Cmd+C on a Mac)."
	)
	Text(sheet, "QuestFont", ns.Ink.text, -220):SetText("Run it on your computer. Then restart WoW.")
	restartHint = Text(sheet, "QuestFontNormalSmall", ns.Ink.faded, -244)
	restartHint:SetText("Already installed? Run gnomish-relay restart on your computer.")
end

local function BuildFooter()
	local help = frame:CreateFontString(nil, "ARTWORK", "GameFontHighlightSmall")
	help:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", EDGE + 10, 20)
	help:SetText("Type /timeways help to see this again.")
	local okay = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
	okay:SetSize(90, 22)
	okay:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", -EDGE - 4, 14)
	okay:SetText("Close")
	okay:SetScript("OnClick", function()
		frame:Hide()
	end)
end

-- `reason` comes from Reason(). With nil, the window shows the install steps alone.
function Welcome.Open(reason)
	if not frame then
		BuildFrame()
		BuildSheet()
		BuildFooter()
	end
	heading:SetText(HEADINGS[reason] or HEADINGS.setup)
	restartHint:SetShown(reason == "offline")
	frame:Show()
end

function Welcome.IsShown()
	return frame ~= nil and frame:IsShown()
end

local function OpenIfNeeded()
	local reason = Welcome.Reason()
	if reason then
		Welcome.Open(reason)
	end
end

-- Runs at each loading screen. Only the first one checks, so the window shows once each session.
function Welcome.Login()
	if checked then
		return
	end
	checked = true
	-- With no key, nothing can reach the app, so there is nothing to wait for.
	C_Timer.After(ns.key and WAIT_SECONDS or 0, OpenIfNeeded)
end
