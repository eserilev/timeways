-- The talk window (GAMEPLAY.md 3.5): a talk with one NPC, in the look of the lore book.
-- Past talks show lighter above the talk of now. Work that the NPC offers comes as a quest
-- card with Accept and Decline, as the quest frame of the game shows a quest.

local _, ns = ...

local TalkWindow = {}
ns.TalkWindow = TalkWindow

local NAME = "TimewaysTalkFrame"
local WIDTH, HEIGHT = 440, 460
local EDGE = 12
local SHEET_TOP, SHEET_BOTTOM = -34, 78
-- The scroll bar of the template stands right of the scroll frame, on the parchment.
local SCROLL_BAR = 26
local MARGIN = 16
local GAP, TURN_GAP = 4, 12
local BUTTON_WIDTH, BUTTON_HEIGHT = 90, 22

-- A quest takes one model call, and one more when the first answer breaks a rule. The
-- bridge waits 60 seconds for each, so the window asks for the journal for 3 minutes.
local POLL_SECONDS, MAX_POLLS = 10, 18

local COPY = {
	thinking = "Thinking...",
	silent = "%s looks at you and says nothing.",
	failed = "No answer came back. Try again.",
	questThinking = "Thinking of a quest...",
	objectives = "Quest Objectives",
	accepted = "Quest accepted.",
	declined = "Quest declined.",
	hint = "Say something...",
	offerInChat = "%s has a quest for you: %s. Type /journal to read it.",
}

-- The font and the ink of each kind of row.
local STYLES = {
	day = { font = "QuestFontNormalSmall", ink = "past" },
	pastSaid = { font = "QuestFontNormalSmall", ink = "past" },
	pastHeard = { font = "QuestFontNormalSmall", ink = "past" },
	said = { font = "QuestFontNormalSmall", ink = "faded" },
	heard = { font = "QuestFont", ink = "text" },
	status = { font = "QuestFont", ink = "faded" },
	questTitle = { font = "QuestTitleFont", ink = "title" },
	questText = { font = "QuestFont", ink = "text" },
	objective = { font = "QuestFont", ink = "text" },
}

-- A row that starts a turn has more room above it.
local STARTS_A_TURN = { day = true, said = true, questTitle = true }

local frame, scroll, page, box, hint, acceptButton, declineButton
local scrollHeight
local texts = {}

-- The talk that the window shows: { npc, since, past, turns }. `since` is the time of its
-- first words, `past` the exchanges before it, and each turn { said, heard, state }, with
-- the state "thinking", "answered", "silent", or "failed". Nil while the window is closed.
local talk
-- True while combat hides the window. The talk stays.
local away = false
-- The quest that the window waits for: { npc, at, polls, planned }.
local waiting
-- The answer of the player to each card, by quest number.
local answered = {}

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Show()
	if InCombatLockdown() then
		away = true
		return
	end
	frame:Show()
end

local function DayOf(at)
	return date("%Y-%m-%d", at)
end

-- "Today", "Yesterday", or the date, as the journal writes it.
local function DayLine(at)
	if DayOf(at) == DayOf(time()) then
		return "Today"
	end
	if DayOf(at) == DayOf(time() - 86400) then
		return "Yesterday"
	end
	return ns.JournalRows.Day(at)
end

local function Row(style, text)
	return { style = style, text = text }
end

local function PastRows(rows)
	local lastDay
	for _, exchange in ipairs(talk.past) do
		local day = DayLine(exchange.at)
		if day ~= lastDay then
			rows[#rows + 1] = Row("day", day)
			lastDay = day
		end
		rows[#rows + 1] = Row("pastSaid", "You: " .. exchange.said)
		rows[#rows + 1] = Row("pastHeard", ns.WithName(exchange.heard))
	end
end

local function AnswerRow(turn)
	if turn.state == "answered" then
		return Row("heard", ns.WithName(turn.heard))
	end
	if turn.state == "silent" then
		return Row("status", string.format(COPY.silent, ns.Plain(talk.npc)))
	end
	return Row("status", COPY[turn.state])
end

local function TurnRows(rows)
	for _, turn in ipairs(talk.turns) do
		rows[#rows + 1] = Row("said", "You: " .. turn.said)
		rows[#rows + 1] = AnswerRow(turn)
	end
end

-- The quest of the journal, when a talk of this window asked for it.
local function OwnQuest()
	local quest = ns.Journal.TalkQuest()
	if talk and quest and quest.npc == talk.npc and quest.at >= talk.since then
		return quest
	end
	return nil
end

-- The offer that waits for an answer, or nil.
local function CardOffer(quest)
	if not quest or quest.state ~= "offered" or answered[quest.number] then
		return nil
	end
	local offer = ns.Journal.Quest(quest.number)
	return offer and offer.status == "offered" and offer or nil
end

local function CardRows(rows, offer)
	rows[#rows + 1] = Row("questTitle", ns.JournalRows.Name(offer.title))
	if type(offer.text) == "string" then
		rows[#rows + 1] = Row("questText", ns.Plain(offer.text))
	end
	rows[#rows + 1] = Row("questTitle", COPY.objectives)
	for _, line in ipairs(ns.JournalQuests.StepLines(offer)) do
		rows[#rows + 1] = Row("objective", line.text)
	end
end

local function QuestRows(rows, quest)
	if not quest then
		return
	end
	if quest.state == "writing" then
		rows[#rows + 1] = Row("status", COPY.questThinking)
	elseif quest.state == "refused" then
		rows[#rows + 1] = Row("status", ns.Plain(quest.line))
	elseif answered[quest.number] then
		rows[#rows + 1] = Row("status", COPY[answered[quest.number]])
	elseif CardOffer(quest) then
		CardRows(rows, CardOffer(quest))
	end
end

local function Rows()
	local rows = {}
	PastRows(rows)
	TurnRows(rows)
	QuestRows(rows, OwnQuest())
	return rows
end

local function Text(n)
	if not texts[n] then
		local text = page:CreateFontString(nil, "ARTWORK", "QuestFont")
		text:SetJustifyH("LEFT")
		text:SetWidth(WIDTH - 2 * EDGE - SCROLL_BAR - 2 * MARGIN)
		texts[n] = text
	end
	return texts[n]
end

-- Each row under the one before it. The page scrolls to the newest row.
local function Layout(rows)
	local height = MARGIN
	for n, row in ipairs(rows) do
		local text, style = Text(n), STYLES[row.style]
		local gap = n == 1 and 0 or (STARTS_A_TURN[row.style] and TURN_GAP or GAP)
		text:SetFontObject(style.font)
		text:SetText(row.text)
		text:SetTextColor(unpack(ns.Ink[style.ink]))
		text:ClearAllPoints()
		text:SetPoint("TOPLEFT", page, "TOPLEFT", MARGIN, -(height + gap))
		text:Show()
		height = height + gap + text:GetStringHeight()
	end
	for n = #rows + 1, #texts do
		texts[n]:Hide()
	end
	height = height + MARGIN
	page:SetHeight(height)
	local bar = scroll.ScrollBar
	if type(bar) == "table" then
		bar:SetShown(height > scrollHeight)
	end
	scroll:SetVerticalScroll(math.max(0, height - scrollHeight))
end

local function LastTurn()
	return talk.turns[#talk.turns]
end

-- The box shows once an answer came, so the player answers the NPC, not the void. The
-- answer comes by itself, so the box takes the cursor only from a player who stands still.
local function ShowBox()
	local last = LastTurn()
	local was = box:IsShown()
	box:SetShown(last ~= nil and last.state ~= "thinking")
	hint:SetShown(box:IsShown() and (box:GetText() or "") == "")
	if box:IsShown() and not was and frame:IsShown() then
		ns.Focus.AtEndIfIdle(box)
	end
end

local function ShowCardButtons()
	local offer = CardOffer(OwnQuest())
	acceptButton:SetShown(offer ~= nil)
	declineButton:SetShown(offer ~= nil)
end

local function Draw()
	if not frame or not talk then
		return
	end
	frame.title:SetText(ns.Plain(talk.npc))
	Layout(Rows())
	ShowBox()
	ShowCardButtons()
end

local function Send()
	local words = (box:GetText() or ""):match("^%s*(.-)%s*$")
	box:ClearFocus()
	if words == "" then
		return
	end
	box:SetText("")
	ns.Talk.Reply(talk.npc, words)
end

local function AnswerCard(answer, run)
	return function()
		local quest = OwnQuest()
		if not quest or quest.state ~= "offered" then
			return
		end
		answered[quest.number] = answer
		run(quest.number)
		Draw()
	end
end

local function BuildFrame()
	frame = CreateFrame("Frame", NAME, UIParent, "BackdropTemplate")
	frame:SetSize(WIDTH, HEIGHT)
	frame:SetPoint("CENTER", UIParent, "CENTER", -260, 40)
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
	-- Combat hides the window too, and then the talk stays.
	frame:SetScript("OnHide", function()
		if not away then
			talk = nil
		end
	end)
	frame:Hide()
	-- Escape closes each frame in this list.
	table.insert(UISpecialFrames, NAME)

	frame.title = frame:CreateFontString(nil, "ARTWORK", "GameFontNormal")
	frame.title:SetPoint("TOP", frame, "TOP", 0, -14)
	local close = CreateFrame("Button", nil, frame, "UIPanelCloseButton")
	close:SetPoint("TOPRIGHT", frame, "TOPRIGHT", -4, -4)
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
end

local function BuildBox()
	box = CreateFrame("EditBox", NAME .. "Reply", frame, "InputBoxTemplate")
	box:SetSize(WIDTH - 2 * EDGE - 16, 24)
	box:SetPoint("BOTTOMLEFT", frame, "BOTTOMLEFT", EDGE + 10, 44)
	box:SetAutoFocus(false)
	-- One chat line of the game, as `/talk` takes it.
	box:SetMaxBytes(255)
	box:SetScript("OnEnterPressed", Send)
	box:SetScript("OnEscapePressed", function()
		box:ClearFocus()
		frame:Hide()
	end)
	box:SetScript("OnTextChanged", function()
		hint:SetShown((box:GetText() or "") == "")
	end)
	ns.Focus.ReleaseOnHide(box)
	hint = box:CreateFontString(nil, "ARTWORK", "GameFontDisable")
	hint:SetPoint("LEFT", box, "LEFT", 4, 0)
	hint:SetText(COPY.hint)
	box:Hide()
end

local function Button(label, point, x, run)
	local button = CreateFrame("Button", nil, frame, "UIPanelButtonTemplate")
	button:SetSize(BUTTON_WIDTH, BUTTON_HEIGHT)
	button:SetPoint(point, frame, point, x, 14)
	button:SetText(label)
	button:SetScript("OnClick", run)
	return button
end

local function BuildFooter()
	Button("Goodbye", "BOTTOMRIGHT", -EDGE - 4, function()
		frame:Hide()
	end)
	acceptButton = Button("Accept", "BOTTOMLEFT", EDGE + 4, AnswerCard("accepted", ns.Quest.Accept))
	declineButton = Button("Decline", "BOTTOMLEFT", EDGE + 8 + BUTTON_WIDTH, AnswerCard("declined", ns.Quest.Decline))
	acceptButton:Hide()
	declineButton:Hide()
end

local function Build()
	BuildFrame()
	BuildPage()
	BuildBox()
	BuildFooter()
end

-- The window opens on this NPC. Another NPC starts a new talk.
function TalkWindow.Open(npc)
	if not frame then
		Build()
	end
	if not talk or talk.npc ~= npc then
		away = false
		frame:Hide()
		talk = { npc = npc, since = time(), past = ns.TalkHistory.Of(npc), turns = {} }
	end
	if not frame:IsShown() and not away then
		Show()
	end
	Draw()
end

-- The words of the player are on their way.
function TalkWindow.Asked(npc, words)
	TalkWindow.Open(npc)
	talk.turns[#talk.turns + 1] = { said = words, state = "thinking" }
	Draw()
end

local function Thinking(npc)
	if not talk or talk.npc ~= npc then
		return nil
	end
	for _, turn in ipairs(talk.turns) do
		if turn.state == "thinking" then
			return turn
		end
	end
	return nil
end

-- True when the window takes the answer. A closed window takes none: it goes to the chat.
function TalkWindow.Takes(npc, text)
	local turn = Thinking(npc)
	if not turn then
		return false
	end
	turn.heard = text
	turn.state = text and "answered" or "silent"
	if not text then
		box:SetText(turn.said)
	end
	Draw()
	return true
end

-- The words stay in the box, so the player can send them again.
function TalkWindow.Failed(npc, words)
	local turn = Thinking(npc)
	if not turn then
		return
	end
	turn.state = "failed"
	box:SetText(words)
	Draw()
end

local function Planned()
	if not waiting then
		return
	end
	waiting.planned = false
	ns.Journal.Request(0)
end

-- One request at a time waits, also when other journals come in between.
local function PlanPoll()
	if waiting.planned or waiting.polls >= MAX_POLLS then
		return
	end
	waiting.polls = waiting.polls + 1
	waiting.planned = true
	C_Timer.After(POLL_SECONDS, Planned)
end

local function IsWaitedFor(quest)
	return waiting ~= nil and quest ~= nil and quest.npc == waiting.npc and quest.at == waiting.at
end

-- A quest that comes while the window is closed gets one line in the chat.
local function SayInChat(quest)
	if quest.state == "refused" then
		Say(ns.Plain(quest.line))
		return
	end
	local offer = ns.Journal.Quest(quest.number)
	if offer then
		Say(string.format(COPY.offerInChat, ns.Plain(quest.npc), ns.JournalRows.Name(offer.title)))
	end
end

-- The window waits for a quest of its own talk, and goes on waiting when it closes.
local function Follow(quest)
	if not quest then
		return
	end
	if quest.state == "writing" then
		if OwnQuest() and not IsWaitedFor(quest) then
			waiting = { npc = quest.npc, at = quest.at, polls = 0, planned = false }
		end
		if IsWaitedFor(quest) then
			PlanPoll()
		end
		return
	end
	if IsWaitedFor(quest) then
		waiting = nil
		if not OwnQuest() then
			SayInChat(quest)
		end
	end
end

-- A new journal can carry the quest of a talk of the window.
function TalkWindow.JournalCame()
	Follow(ns.Journal.TalkQuest())
	Draw()
end

function TalkWindow.CombatStarted()
	if frame and frame:IsShown() then
		away = true
		frame:Hide()
	end
end

function TalkWindow.CombatEnded()
	if away then
		away = false
		frame:Show()
		Draw()
	end
end

function TalkWindow.IsShown()
	return frame ~= nil and frame:IsShown()
end
