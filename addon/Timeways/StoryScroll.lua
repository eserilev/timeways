-- The writing scroll of a story (GAMEPLAY.md 4.8): a title and a body on parchment, for a
-- player of your group. It asks the player's box first, so you know before you send. Save
-- keeps a draft, and Close saves a draft that changed.

local _, ns = ...

local StoryScroll = {}
ns.StoryScroll = StoryScroll

local NAME = "TimewaysStoryScroll"
local WIDTH, HEIGHT = 520, 560
local INSET = 22
local PORTRAIT = 64
local HEADER_TOP, TITLE_TOP, BODY_TOP = -40, -126, -168
local BODY_HEIGHT = 250
local SCROLL_BAR = 22
local BUTTON_WIDTH, BUTTON_HEIGHT, BUTTON_BOTTOM = 84, 22, 16
-- The count shows only when this few letters are left.
local COUNT_FROM = 100

StoryScroll.CHECKING = "Checking..."
StoryScroll.SENDING = "Sending..."
StoryScroll.LOGGED = "Like chat, Blizzard can read what you send."
StoryScroll.NO_ROOM = "No room left"

local frame, about, name, level, portrait, titleBox, bodyBox, bodyScroll
local titleHint, bodyHint, count, status, sendButton

-- The player of the open scroll, the room of the box, and the text when it opened.
local to, room, opened

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Short(player)
	return ns.TaskPeople.Short(player)
end

local function InGroup()
	return ns.TaskPeople.InGroup(to)
end

-- A typed `|` reads as "||" in a box, as the game keeps it.
local function Unescaped(text)
	return (text:gsub("||", "|"))
end

local function TypedTitle()
	return Unescaped(titleBox:GetText() or ""):match("^%s*(.-)%s*$")
end

local function TypedBody()
	return ns.StoryText.Body(Unescaped(bodyBox:GetText() or ""))
end

local function IsEmpty()
	return TypedTitle() == "" and TypedBody() == ""
end

local function Changed()
	return (titleBox:GetText() or "") ~= opened.title or (bodyBox:GetText() or "") ~= opened.text
end

local function SetStatus(text)
	status:SetText(text or "")
end

local function UpdateSend()
	sendButton:SetEnabled(room == "open" and TypedBody() ~= "")
end

-- The letters left in the body, as the story counts them after its blank lines go.
local function Left()
	local body = TypedBody()
	local letters = ns.StoryText.BODY_LETTERS - ns.StoryText.Letters(body)
	return math.min(letters, ns.StoryText.BODY_BYTES - #body)
end

local function ShowCount()
	local left = Left()
	if left <= 0 then
		count:SetText(StoryScroll.NO_ROOM)
	elseif left <= COUNT_FROM then
		count:SetText(string.format("%d left", left))
	else
		count:SetText("")
	end
end

local function TextChanged()
	titleHint:SetShown((titleBox:GetText() or "") == "")
	bodyHint:SetShown((bodyBox:GetText() or "") == "")
	ShowCount()
	UpdateSend()
end

local function Answered(answer)
	room = answer
	SetStatus(answer ~= "open" and ns.PlayerStories.RoomLine(answer, to) or nil)
	UpdateSend()
end

-- Asks the box of the player, and the status line follows the answer.
local function Check()
	if not InGroup() then
		room = nil
		SetStatus("Invite " .. Short(to) .. " to your group to send it.")
		UpdateSend()
		return
	end
	room = "checking"
	SetStatus(StoryScroll.CHECKING)
	UpdateSend()
	local asked = to
	ns.PlayerStories.Ask(asked, function(answer)
		if frame:IsShown() and to == asked then
			Answered(answer)
		end
	end)
end

-- The first bad sign of a box, selected. A typed "|" is two bytes in the box.
local function SelectBadSign()
	for _, box in ipairs({ titleBox, bodyBox }) do
		local text = box:GetText() or ""
		local first, last = ns.StoryText.BadSign(text)
		if first then
			if text:sub(first, first + 1) == "||" then
				last = first + 1
			end
			box:SetFocus()
			box:HighlightText(first - 1, last)
			return
		end
	end
end

local function Sent(title, body)
	ns.PlayerStories.Send(to, title, body)
	titleBox:SetText("")
	bodyBox:SetText("")
	opened = { title = "", text = "" }
	room = nil
	SetStatus("Sent to " .. Short(to) .. ". They'll decide if it's part of their story.")
	TextChanged()
end

-- Send asks again, and sends only on "open".
function StoryScroll.Send()
	if not InGroup() then
		SetStatus(Short(to) .. " left your group. Invite them back to send it.")
		return
	end
	local title, body = TypedTitle(), TypedBody()
	local problem = ns.StoryText.Problem(title, body)
	if problem then
		SetStatus(problem)
		SelectBadSign()
		return
	end
	room = "checking"
	SetStatus(StoryScroll.SENDING)
	UpdateSend()
	local asked = to
	ns.PlayerStories.Ask(asked, function(answer)
		if to ~= asked then
			return
		end
		if answer == "open" then
			Sent(title, body)
		else
			Answered(answer)
		end
	end)
end

-- Keeps the text as it shows in the boxes. False when 10 other drafts wait.
local function SaveDraft()
	if IsEmpty() then
		ns.StoryDrafts.Delete(to)
		return true
	end
	return ns.StoryDrafts.Save(to, titleBox:GetText() or "", bodyBox:GetText() or "")
end

local function Hide()
	titleBox:ClearFocus()
	bodyBox:ClearFocus()
	frame:Hide()
	to = nil
	ns.JournalFrame.Refresh()
end

function StoryScroll.Save()
	if not SaveDraft() then
		SetStatus(ns.StoryDrafts.FULL)
		return
	end
	Hide()
	Say("Saved. Find it under Drafts in Stories.")
end

StaticPopupDialogs.TIMEWAYS_STORY_DISCARD = {
	text = "Discard this story?",
	button1 = "Discard",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function()
		Hide()
	end,
}

-- No text is lost: a changed text becomes a draft, and with 10 drafts the player decides.
function StoryScroll.Close()
	if not Changed() then
		Hide()
		return
	end
	if SaveDraft() then
		Hide()
		return
	end
	StaticPopup_Show("TIMEWAYS_STORY_DISCARD")
end

-- Building ------------------------------------------------------------------------------------

local function Text(font, ink, point, x, y)
	local text = frame:CreateFontString(nil, "ARTWORK", font)
	text:SetPoint(point, frame, point, x, y)
	text:SetJustifyH("LEFT")
	text:SetTextColor(ink[1], ink[2], ink[3])
	return text
end

local function Button(label, x, run)
	local button = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
	button:SetSize(BUTTON_WIDTH, BUTTON_HEIGHT)
	button:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", x, BUTTON_BOTTOM)
	button:SetText(label)
	button:SetScript("OnClick", run)
	return button
end

local function Ink(box)
	local ink = ns.Ink.text
	box:SetAutoFocus(false)
	box:SetTextColor(ink[1], ink[2], ink[3])
	box:SetScript("OnTextChanged", TextChanged)
	box:SetScript("OnEscapePressed", StoryScroll.Close)
	ns.Focus.ReleaseOnHide(box)
end

-- A hint in faded ink stands in the box while it is empty.
local function Hint(box, text)
	local hint = box:CreateFontString(nil, "ARTWORK", box == titleBox and "QuestTitleFont" or "QuestFont")
	hint:SetPoint("TOPLEFT", box, "TOPLEFT", 0, 0)
	hint:SetTextColor(ns.Ink.faded[1], ns.Ink.faded[2], ns.Ink.faded[3])
	hint:SetText(text)
	return hint
end

local function BuildTitle()
	titleBox = CreateFrame("EditBox", nil, frame)
	titleBox:SetPoint("TOPLEFT", frame, "TOPLEFT", INSET, TITLE_TOP)
	titleBox:SetSize(WIDTH - 2 * INSET, 28)
	titleBox:SetFontObject("QuestTitleFont")
	titleBox:SetMaxLetters(ns.StoryText.TITLE_LETTERS)
	titleBox:SetMaxBytes(ns.StoryText.TITLE_BYTES + 1)
	Ink(titleBox)
	local function ToBody()
		bodyBox:SetFocus()
	end
	titleBox:SetScript("OnEnterPressed", ToBody)
	titleBox:SetScript("OnTabPressed", ToBody)
	titleHint = Hint(titleBox, "Title")
end

-- Enter starts a new paragraph, and Ctrl+Enter sends. The box inserts the break itself, so
-- the key works whether or not the game adds one: a blank line makes the same story.
local function EnterPressed()
	if IsControlKeyDown() then
		StoryScroll.Send()
	else
		bodyBox:Insert("\n")
	end
end

local function BuildBody()
	bodyScroll = CreateFrame("ScrollFrame", NAME .. "Body", frame, "UIPanelScrollFrameTemplate")
	bodyScroll:SetPoint("TOPLEFT", frame, "TOPLEFT", INSET, BODY_TOP)
	bodyScroll:SetSize(WIDTH - 2 * INSET - SCROLL_BAR, BODY_HEIGHT)
	bodyBox = CreateFrame("EditBox", nil, bodyScroll)
	bodyScroll:SetScrollChild(bodyBox)
	bodyBox:SetWidth(WIDTH - 2 * INSET - SCROLL_BAR)
	bodyBox:SetMultiLine(true)
	bodyBox:SetFontObject("QuestFont")
	bodyBox:SetMaxLetters(ns.StoryText.BODY_LETTERS)
	bodyBox:SetMaxBytes(ns.StoryText.BODY_BYTES + 1)
	Ink(bodyBox)
	bodyBox:SetScript("OnEnterPressed", EnterPressed)
	bodyBox:SetScript("OnTabPressed", function()
		titleBox:SetFocus()
	end)
	ns.Focus.OnAreaClick(bodyScroll, bodyBox)
	bodyHint = Hint(bodyBox, "")
end

local function BuildHeader()
	portrait = CreateFrame("PlayerModel", nil, frame)
	portrait:SetSize(PORTRAIT, PORTRAIT)
	portrait:SetPoint("TOPLEFT", frame, "TOPLEFT", INSET, HEADER_TOP)
	local left = INSET + PORTRAIT + 12
	about = Text("QuestFontNormalSmall", ns.Ink.faded, "TOPLEFT", left, HEADER_TOP)
	about:SetText("A story about")
	name = Text("QuestTitleFont", ns.Ink.title, "TOPLEFT", left, HEADER_TOP - 16)
	level = Text("QuestFontNormalSmall", ns.Ink.faded, "TOPLEFT", left, HEADER_TOP - 40)
end

local function Build()
	frame = CreateFrame("Frame", NAME, UIParent)
	frame:SetSize(WIDTH, HEIGHT)
	frame:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
	frame:SetFrameStrata("HIGH")
	frame:SetToplevel(true)
	frame:EnableMouse(true)
	frame:SetMovable(true)
	frame:RegisterForDrag("LeftButton")
	frame:SetScript("OnDragStart", frame.StartMoving)
	frame:SetScript("OnDragStop", frame.StopMovingOrSizing)
	local parchment = frame:CreateTexture(nil, "BACKGROUND")
	parchment:SetAllPoints(frame)
	parchment:SetAtlas("QuestBG-Parchment")
	local heading = Text("GameFontNormal", ns.Ink.title, "TOP", 0, -14)
	heading:SetText("Tell a Story")
	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("TOPRIGHT", frame, "TOPRIGHT", -4, -4)
	close:SetScript("OnClick", StoryScroll.Close)
	BuildHeader()
	BuildTitle()
	BuildBody()
	count = Text("QuestFontNormalSmall", ns.Ink.faded, "TOPRIGHT", -INSET, BODY_TOP - BODY_HEIGHT - 4)
	count:SetJustifyH("RIGHT")
	status = Text("QuestFont", ns.Ink.text, "BOTTOMLEFT", INSET, BUTTON_BOTTOM + BUTTON_HEIGHT + 28)
	status:SetWidth(WIDTH - 2 * INSET)
	local footer = Text("QuestFontNormalSmall", ns.Ink.faded, "BOTTOMLEFT", INSET, BUTTON_BOTTOM + BUTTON_HEIGHT + 8)
	footer:SetText(StoryScroll.LOGGED)
	sendButton = Button("Send", -INSET, StoryScroll.Send)
	Button("Save", -(INSET + BUTTON_WIDTH + 4), StoryScroll.Save)
	Button("Close", -(INSET + 2 * (BUTTON_WIDTH + 4)), StoryScroll.Close)
	frame:Hide()
end

-- "Level 11 Undead Mage", from the game, while the player is your target.
local function LevelLine(unit)
	local race = UnitRace(unit)
	local class = UnitClass(unit)
	return string.format("Level %d %s %s", UnitLevel(unit) or 0, race or "", class or "")
end

-- A draft for a player who is not your target shows the name only.
local function ShowHeader()
	name:SetText(Short(to))
	local targeted = ns.TaskPeople.OfUnit("target") == to
	portrait:SetShown(targeted)
	if targeted then
		portrait:SetUnit("target")
	end
	level:SetText(targeted and LevelLine("target") or "")
	bodyHint:SetText("What happened with " .. Short(to) .. "?")
end

local function Open(player)
	if not frame then
		Build()
	end
	-- The text for another player becomes its draft first, so no text is lost.
	if frame:IsShown() and to ~= player and Changed() and not SaveDraft() then
		SetStatus(ns.StoryDrafts.FULL)
		return
	end
	to = player
	local draft = ns.StoryDrafts.For(player)
	opened = { title = draft and draft.title or "", text = draft and draft.text or "" }
	titleBox:SetText(opened.title)
	bodyBox:SetText(opened.text)
	ShowHeader()
	frame:Show()
	TextChanged()
	Check()
	-- A blank scroll starts at the title, and a draft with a title goes on with the body.
	ns.Focus.AtEnd(opened.title == "" and titleBox or bodyBox)
end

-- `/story` opens the scroll for your target, with its draft.
function StoryScroll.OpenFor(player)
	if not player or not ns.TaskPeople.InGroup(player) then
		Say("Target a player in your group first.")
		return
	end
	Open(player)
end

-- Continue on a draft opens it for its player, also one who is not your target.
function StoryScroll.OpenDraft(player)
	Open(player)
end

function StoryScroll.IsShown()
	return frame ~= nil and frame:IsShown()
end

function StoryScroll.Player()
	return to
end

-- The status line: the room of the box, a problem, or what happened.
function StoryScroll.Status()
	return status and status:GetText() or ""
end
