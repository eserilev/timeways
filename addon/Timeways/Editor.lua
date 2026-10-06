-- The writing page of the book: a title, a hint or a box for a heading, and a box of
-- several lines on the parchment, with Save and Cancel under it.

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
-- Room on the right for the scroll bar of the template.
local SCROLL_BAR = 22

local view, title, hint, heading, scroll, box, count
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

local function SetCountInk(ink)
	count:SetTextColor(ink[1], ink[2], ink[3])
end

-- "16 / 1000". A letter outside ASCII takes more than one byte, so with a byte limit the room
-- left is a guess in letters.
function Editor.Count(text, letters, limit, bytes)
	if not bytes or #text == letters then
		return string.format("%d / %d", letters, limit)
	end
	local left = math.min(limit - letters, bytes - #text)
	return left > 0 and string.format("About %d left", left) or "None left"
end

local function ShowCount()
	SetCountInk(ns.Ink.faded)
	local text = box:GetText() or ""
	count:SetText(Editor.Count(text, box:GetNumLetters(), request.limit, request.bytes))
end

-- The scroll frame moves with the cursor, so the line that the player writes stays in view.
local function FollowCursor(_, _, y, _, height)
	local top, shown = -y, scroll:GetHeight()
	local offset = scroll:GetVerticalScroll()
	if top < offset then
		scroll:SetVerticalScroll(top)
	elseif top + height > offset + shown then
		scroll:SetVerticalScroll(top + height - shown)
	end
end

-- A long text scrolls inside the box. A scroll child needs its own width.
local function BuildScroll()
	scroll = CreateFrame("ScrollFrame", "TimewaysEditorScroll", view, "UIPanelScrollFrameTemplate")
	scroll:SetPoint("TOPLEFT", view, "TOPLEFT", INSET, BOX_TOP - 6)
	scroll:SetPoint("TOPRIGHT", view, "TOPRIGHT", -(INSET + SCROLL_BAR), BOX_TOP - 6)
	scroll:SetHeight(BOX_HEIGHT - 12)
	box = CreateFrame("EditBox", nil, scroll)
	scroll:SetScrollChild(box)
	scroll:SetScript("OnSizeChanged", function(_, width)
		box:SetWidth(width)
	end)
	ns.Focus.OnAreaClick(scroll, box)
end

local function BuildBox()
	local ink = ns.Ink.text
	local shade = view:CreateTexture(nil, "BACKGROUND")
	shade:SetColorTexture(ink[1], ink[2], ink[3], 0.12)
	Span(shade, BOX_TOP, INSET - 6)
	shade:SetHeight(BOX_HEIGHT)
	BuildScroll()
	box:SetMultiLine(true)
	box:SetAutoFocus(false)
	box:SetFontObject("QuestFont")
	box:SetTextColor(ink[1], ink[2], ink[3])
	box:SetScript("OnCursorChanged", FollowCursor)
	box:SetScript("OnEnterPressed", Editor.Save)
	box:SetScript("OnEscapePressed", Editor.Cancel)
	box:SetScript("OnTextChanged", ShowCount)
	ns.Focus.ReleaseOnHide(box)
	count = view:CreateFontString(nil, "ARTWORK", "QuestFontNormalSmall")
	count:SetPoint("TOPRIGHT", view, "TOPRIGHT", -(INSET - 6), BOX_TOP - BOX_HEIGHT - 6)
end

-- One line for a heading that the player can change, in place of the hint.
local function BuildHeading()
	heading = CreateFrame("EditBox", nil, view, "InputBoxTemplate")
	heading:SetPoint("TOPLEFT", view, "TOPLEFT", INSET + 6, HINT_TOP + 4)
	heading:SetPoint("TOPRIGHT", view, "TOPRIGHT", -INSET, HINT_TOP + 4)
	heading:SetHeight(BUTTON_HEIGHT)
	heading:SetAutoFocus(false)
	heading:SetFontObject("QuestFont")
	heading:SetScript("OnEnterPressed", function()
		box:SetFocus()
	end)
	heading:SetScript("OnEscapePressed", Editor.Cancel)
	ns.Focus.ReleaseOnHide(heading)
end

local function Build(parent)
	view = CreateFrame("Frame", nil, parent)
	view:SetAllPoints(parent)
	title = Label("QuestTitleFont", ns.Ink.title, TITLE_TOP)
	hint = Label("QuestFont", ns.Ink.faded, HINT_TOP)
	BuildBox()
	BuildHeading()
	Button("Save", "BOTTOMLEFT", INSET, Editor.Save)
	Button("Cancel", "BOTTOMRIGHT", -INSET, Editor.Cancel)
end

local function Close()
	box:ClearFocus()
	heading:ClearFocus()
	view:Hide()
	closed()
end

-- `edit` is { title, hint, text, limit, save = function(text, heading) }, and optionally
-- `problem = function(text, heading)`, which gives the reason that the text can't be saved,
-- or nil, `bytes`, the limit of the text in bytes, and `heading` = { text, letters }, a
-- heading that the player can change in place of the hint.
-- `onClose` runs when the player saves or cancels.
function Editor.Open(parent, edit, onClose)
	if not view then
		Build(parent)
	end
	request, closed = edit, onClose
	title:SetText(edit.title)
	hint:SetText(edit.hint)
	hint:SetShown(edit.heading == nil)
	heading:SetShown(edit.heading ~= nil)
	if edit.heading then
		heading:SetMaxLetters(edit.heading.letters)
		heading:SetText(edit.heading.text or "")
	end
	box:SetMaxLetters(edit.limit)
	box:SetText(edit.text or "")
	scroll:SetVerticalScroll(0)
	ShowCount()
	view:Show()
	ns.Focus.AtEnd(box)
end

-- A text that can't be saved stays in the box, with the reason under it. The reason takes
-- the ink of the text, so it does not read like the faded count.
function Editor.Save()
	local text = box:GetText()
	local typed = request.heading and heading:GetText() or nil
	local problem = request.problem and request.problem(text, typed)
	if problem then
		SetCountInk(ns.Ink.text)
		count:SetText(problem)
		return
	end
	Close()
	request.save(text, typed)
end

function Editor.Cancel()
	Close()
end

function Editor.IsShown()
	return view ~= nil and view:IsShown()
end
