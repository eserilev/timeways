-- The writing page of the book: a title, a hint, and a box of several lines on the
-- parchment, with Save and Cancel under it.

local _, ns = ...

local Editor = {}
ns.Editor = Editor

-- Offsets in the parchment pane. Each part spans the pane from the left inset to the right
-- one, so the editor fits a pane of any width.
local INSET = 16
local TITLE_TOP, HINT_TOP = -16, -44
local BOX_TOP, BOX_HEIGHT = -100, 176
local BUTTON_BOTTOM = 12
local BUTTON_WIDTH, BUTTON_HEIGHT = 78, 22

local view, title, hint, box, count
local request, closed

-- Two anchors give the width of the pane, less the insets.
local function Span(region, top, inset)
	region:SetPoint("TOPLEFT", view, "TOPLEFT", inset, top)
	region:SetPoint("TOPRIGHT", view, "TOPRIGHT", -inset, top)
end

local function Label(font, color, y)
	local label = view:CreateFontString(nil, "ARTWORK", font)
	Span(label, y, INSET)
	label:SetJustifyH("LEFT")
	label:SetTextColor(color[1], color[2], color[3])
	return label
end

local function Button(label, point, x, run)
	local button = CreateFrame("Button", nil, view, "UIPanelButtonTemplate")
	button:SetSize(BUTTON_WIDTH, BUTTON_HEIGHT)
	button:SetPoint(point, view, point, x, BUTTON_BOTTOM)
	button:SetText(label)
	button:SetScript("OnClick", run)
	return button
end

local function ShowCount()
	count:SetText(string.format("%d / %d", box:GetNumLetters(), request.limit))
end

-- The box holds 300 letters of the quest font with room to spare, so it needs no scroll bar.
local function BuildBox()
	local ink = ns.Ink.text
	local shade = view:CreateTexture(nil, "BACKGROUND")
	shade:SetColorTexture(ink[1], ink[2], ink[3], 0.12)
	Span(shade, BOX_TOP, INSET - 6)
	shade:SetHeight(BOX_HEIGHT)
	box = CreateFrame("EditBox", nil, view)
	box:SetMultiLine(true)
	box:SetAutoFocus(false)
	box:SetFontObject("QuestFont")
	box:SetTextColor(ink[1], ink[2], ink[3])
	Span(box, BOX_TOP - 6, INSET)
	box:SetHeight(BOX_HEIGHT - 12)
	box:SetScript("OnEnterPressed", Editor.Save)
	box:SetScript("OnEscapePressed", Editor.Cancel)
	box:SetScript("OnTextChanged", ShowCount)
	count = view:CreateFontString(nil, "ARTWORK", "QuestFontNormalSmall")
	count:SetPoint("TOPRIGHT", view, "TOPRIGHT", -(INSET - 6), BOX_TOP - BOX_HEIGHT - 6)
	count:SetTextColor(ns.Ink.faded[1], ns.Ink.faded[2], ns.Ink.faded[3])
end

local function Build(parent)
	view = CreateFrame("Frame", nil, parent)
	view:SetAllPoints(parent)
	title = Label("QuestTitleFont", ns.Ink.title, TITLE_TOP)
	hint = Label("QuestFont", ns.Ink.faded, HINT_TOP)
	BuildBox()
	Button("Save", "BOTTOMLEFT", INSET, Editor.Save)
	Button("Cancel", "BOTTOMRIGHT", -INSET, Editor.Cancel)
end

local function Close()
	box:ClearFocus()
	view:Hide()
	closed()
end

-- `edit` is { title, hint, text, limit, save = function(text) }. `onClose` runs when the
-- player saves or cancels.
function Editor.Open(parent, edit, onClose)
	if not view then
		Build(parent)
	end
	request, closed = edit, onClose
	title:SetText(edit.title)
	hint:SetText(edit.hint)
	box:SetMaxLetters(edit.limit)
	box:SetText(edit.text or "")
	ShowCount()
	view:Show()
	box:SetFocus()
end

function Editor.Save()
	local text = box:GetText()
	Close()
	request.save(text)
end

function Editor.Cancel()
	Close()
end

function Editor.IsShown()
	return view ~= nil and view:IsShown()
end
