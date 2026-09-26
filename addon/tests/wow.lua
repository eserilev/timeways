-- A small fake of the WoW API: only the calls that Timeways makes. It returns a `wow`
-- table that tests use to set the game state, fire events, and read the chat.

local wow = {
	now = 1790000000,
	startedAt = 1790000000 - 1000,
	after = {},
	loaded = {},
	cvars = {},
	shots = 0,
	realm = "Stormrage",
	zone = "",
	subzone = "",
	units = {},
	printed = {},
	widgets = {},
	tickers = {},
}

function time()
	return wow.now
end

function GetRealmName()
	return wow.realm
end

function GetRealZoneText()
	return wow.zone
end

function GetSubZoneText()
	return wow.subzone
end

function UnitExists(unit)
	return wow.units[unit] ~= nil
end

function UnitName(unit)
	local u = wow.units[unit]
	return u and u.name
end

function UnitIsPlayer(unit)
	local u = wow.units[unit]
	return u ~= nil and u.player == true
end

function UnitGUID(unit)
	local u = wow.units[unit]
	return u and u.guid
end

function UnitClassification(unit)
	local u = wow.units[unit]
	return u and u.classification or "normal"
end

-- The text of the open quest, gossip, or book window.
wow.text = {}

C_GossipInfo = {
	GetText = function()
		return wow.text.gossip
	end,
}

function GetGreetingText()
	return wow.text.greeting
end

function GetTitleText()
	return wow.text.title
end

function GetQuestText()
	return wow.text.quest
end

function GetObjectiveText()
	return wow.text.objectives
end

function GetProgressText()
	return wow.text.progress
end

function GetRewardText()
	return wow.text.reward
end

function ItemTextGetItem()
	return wow.text.item
end

function ItemTextGetText()
	return wow.text.page
end

function ItemTextGetCreator()
	return wow.text.creator
end

-- A test marks a value as hidden by putting it in this set.
wow.secrets = {}
function issecretvalue(value)
	return value ~= nil and wow.secrets[value] == true
end

-- Runs `hook` after the function, as the game does. The function stays the same.
function hooksecurefunc(owner, name, hook)
	local original = owner[name]
	owner[name] = function(...)
		local results = { original(...) }
		hook(...)
		return unpack(results)
	end
end

C_ChatInfo = {
	PerformEmote = function() end,
}

-- Dialogs of the game: each shown one waits in `wow.popups`, with its data.
StaticPopupDialogs = {}
wow.popups = {}
function StaticPopup_Show(which, text, _, data)
	wow.popups[#wow.popups + 1] = { which = which, text = text, data = data }
end

-- Saves a text in the last dialog, as a click on Save does.
function wow.SavePopup(text)
	local popup = wow.popups[#wow.popups]
	local dialog = StaticPopupDialogs[popup.which]
	local editBox = {
		GetText = function()
			return text
		end,
	}
	dialog.OnAccept({
		GetEditBox = function()
			return editBox
		end,
	}, popup.data)
end

-- The recap of the last death: a list of events, the killing blow first.
wow.recap = nil
C_DeathRecap = {
	HasRecapEvents = function()
		return wow.recap ~= nil
	end,
	GetRecapEvents = function()
		return wow.recap
	end,
}

-- A pet or another unit of a player: `player` or `controlled` in its table.
function UnitPlayerControlled(unit)
	local u = wow.units[unit]
	return u ~= nil and (u.player == true or u.controlled == true)
end

function UnitLevel(unit)
	local u = wow.units[unit]
	return u and u.level or 0
end

-- A widget keeps what the tests read: its text, scripts, events, and whether it shows.
-- Any other capitalized method is a no-op, like the layout calls.
local Widget = {}
local function Nothing() end
local WidgetMeta = {
	__index = function(_, key)
		return Widget[key] or (type(key) == "string" and key:match("^%u") and Nothing or nil)
	end,
}

local function NewWidget(kind, name, parent, template)
	local widget = setmetatable({
		kind = kind,
		parent = parent,
		template = template,
		events = {},
		scripts = {},
		shown = true,
		enabled = true,
	}, WidgetMeta)
	wow.widgets[#wow.widgets + 1] = widget
	if name then
		_G[name] = widget
	end
	return widget
end

function Widget:RegisterEvent(event)
	self.events[event] = true
end

function Widget:SetScript(name, handler)
	self.scripts[name] = handler
end

function Widget:Show()
	local was = self.shown
	self.shown = true
	if not was and self.scripts.OnShow then
		self.scripts.OnShow(self)
	end
end

function Widget:Hide()
	self.shown = false
end

function Widget:IsShown()
	return self.shown
end

function Widget:SetShown(shown)
	if shown then
		self:Show()
	else
		self:Hide()
	end
end

function Widget:SetText(text)
	self.text = text
end

function Widget:GetText()
	return self.text
end

function Widget:GetStringHeight()
	return 14
end

function Widget:SetEnabled(enabled)
	self.enabled = enabled
end

function Widget:IsEnabled()
	return self.enabled
end

function Widget:SetScrollChild(child)
	self.scrollChild = child
end

function Widget:GetScrollChild()
	return self.scrollChild
end

function Widget:Click()
	self.scripts.OnClick(self)
end

function Widget:CreateFontString()
	return NewWidget("FontString", nil, self)
end

function Widget:CreateTexture()
	return NewWidget("Texture", nil, self)
end

function CreateFrame(kind, name, parent, template)
	return NewWidget(kind, name, parent, template)
end

UIParent = NewWidget("Frame", "UIParent")
UISpecialFrames = {}
-- In UTC on the clock of the fake game, so a test never depends on the zone of its machine.
function date(format, at)
	return os.date("!" .. format, at or wow.now)
end

C_Timer = {
	NewTicker = function(seconds, callback)
		wow.tickers[#wow.tickers + 1] = { seconds = seconds, callback = callback }
	end,
	-- One-shot timers wait in `wow.after` until a test runs them.
	After = function(seconds, callback)
		wow.after[#wow.after + 1] = { seconds = seconds, callback = callback }
	end,
}

-- The seconds since the client started, as WoW gives them.
function GetTime()
	return wow.now - wow.startedAt
end

function GetBuildInfo()
	return "1.60.1", "70009", "Sep 1 2026", 16001
end

-- No slot addon is installed, so every slot counts as free and loads nothing.
C_AddOns = {
	IsAddOnLoaded = function(name)
		return wow.loaded[name] == true
	end,
	EnableAddOn = function() end,
	LoadAddOn = function(name)
		wow.loaded[name] = true
		return false, "MISSING"
	end,
}

C_CVar = {
	GetCVar = function(name)
		return wow.cvars[name]
	end,
	SetCVar = function(name, value)
		wow.cvars[name] = tostring(value)
	end,
}

function SetCVar(name, value)
	wow.cvars[name] = tostring(value)
end

-- The game saves the picture and reports success, as it does for each screenshot.
function Screenshot()
	wow.shots = wow.shots + 1
	wow.Fire("SCREENSHOT_SUCCEEDED")
end

function GetPhysicalScreenSize()
	return 1920, 1080
end

function InCombatLockdown()
	return false
end

function PlaySound() end

SOUNDKIT = {}

function strtrim(text)
	return (text:gsub("^%s+", ""):gsub("%s+$", ""))
end

DEFAULT_CHAT_FRAME = {
	AddMessage = function(_, text)
		wow.printed[#wow.printed + 1] = text
	end,
}

SlashCmdList = {}

function wow.Fire(event, ...)
	for _, frame in ipairs(wow.widgets) do
		if frame.events[event] then
			frame.scripts.OnEvent(frame, event, ...)
		end
	end
end

function wow.RunTickers()
	for _, ticker in ipairs(wow.tickers) do
		ticker.callback()
	end
end

-- Runs a slash command as the chat box does: through its SLASH_<NAME><n> global.
function wow.Slash(command, message)
	for key, value in pairs(_G) do
		local name = type(key) == "string" and key:match("^SLASH_(.-)%d+$")
		if name and value == command then
			return SlashCmdList[name](message)
		end
	end
	error("no slash command " .. command)
end

-- The font strings of a frame that show, in the order of creation.
function wow.ShownTexts(parent)
	local texts = {}
	for _, widget in ipairs(wow.widgets) do
		if widget.kind == "FontString" and widget.parent == parent and widget.shown then
			texts[#texts + 1] = widget.text
		end
	end
	return texts
end

return wow
