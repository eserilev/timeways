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
	-- The count of each item in the bags, by name.
	bags = {},
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

wow.combat = false

function InCombatLockdown()
	return wow.combat
end

-- "none" outside, or "party", "raid", "pvp", "arena".
wow.instance = "none"

function IsInInstance()
	return wow.instance ~= "none", wow.instance
end

-- The table of a unit. The client refuses a hidden unit token as an argument, as it does in
-- restricted content.
local function Unit(unit)
	if issecretvalue(unit) then
		error("a secret value as the unit argument", 2)
	end
	return wow.units[unit]
end

-- A game object such as a Wanted poster has `object` in its table. It has a name, but the
-- tests take the worst case: the client calls it absent.
function UnitExists(unit)
	local u = Unit(unit)
	return u ~= nil and not u.object
end

-- A player of another realm has `realm` in its table, as the game shows it: "Argent Dawn".
function UnitName(unit)
	local u = Unit(unit)
	if not u then
		return nil
	end
	return u.name, u.realm
end

-- The realm without its spaces, as in the name of the sender of an addon message.
function UnitFullName(unit)
	local u = Unit(unit)
	if not u then
		return nil
	end
	return u.name, u.realm and u.realm:gsub("%s", "") or nil
end

function UnitIsPlayer(unit)
	local u = Unit(unit)
	return u ~= nil and u.player == true
end

function UnitGUID(unit)
	local u = Unit(unit)
	return u and u.guid
end

function UnitClassification(unit)
	local u = Unit(unit)
	return u and u.classification or "normal"
end

Enum = {
	TooltipDataType = { Unit = 2 },
	UIMapType = { Cosmic = 0, World = 1, Continent = 2, Zone = 3, Dungeon = 4, Micro = 5, Orphan = 6 },
	SendAddonMessageResult = {
		Success = 0,
		InvalidPrefix = 1,
		InvalidMessage = 2,
		AddonMessageThrottle = 3,
		InvalidChatType = 4,
		NotInGroup = 5,
		TargetRequired = 6,
		InvalidChannel = 7,
		ChannelThrottle = 8,
		GeneralError = 9,
		NotInGuild = 10,
		AddOnMessageLockdown = 11,
		TargetOffline = 12,
	},
}

-- The game's tooltip, and the hooks that run after it shows a unit.
wow.tooltipHooks = {}
wow.tooltip = { unit = nil, lines = {} }

TooltipDataProcessor = {
	AddTooltipPostCall = function(kind, hook)
		if kind == Enum.TooltipDataType.Unit then
			table.insert(wow.tooltipHooks, hook)
		end
	end,
}

GameTooltip = {
	SetOwner = function() end,
	SetItemByID = function() end,
	Show = function() end,
	Hide = function() end,
	GetUnit = function()
		local unit = wow.tooltip.unit
		return unit and wow.units[unit].name, unit
	end,
	AddLine = function(_, text)
		table.insert(wow.tooltip.lines, text)
	end,
}

-- Shows the tooltip of `unit`, as a mouseover does.
function wow.ShowTooltip(unit)
	wow.tooltip = { unit = unit, lines = {} }
	for _, hook in ipairs(wow.tooltipHooks) do
		hook(GameTooltip)
	end
	return wow.tooltip.lines
end

-- The name of the class, and its file name, as the client gives both.
wow.class = "Paladin"

function UnitClass(unit)
	if unit == "player" then
		return wow.class, wow.class:upper()
	end
	local known = wow.units[unit]
	if known and known.class then
		return known.class, known.class:upper()
	end
end

wow.race = "Human"

function UnitRace(unit)
	if unit == "player" then
		return wow.race, wow.race
	end
	local known = wow.units[unit]
	if known and known.race then
		return known.race, known.race
	end
end

-- 2 is male, 3 is female.
function UnitSex(unit)
	if unit == "player" then
		return 3
	end
end

-- Another player has `faction` in its table, or is of the Alliance, as the player is.
function UnitFactionGroup(unit)
	if unit == "player" then
		return "Alliance", "Alliance"
	end
	local u = Unit(unit)
	if u and u.player then
		local faction = u.faction or "Alliance"
		return faction, faction
	end
end

-- A player of a realm that is not connected to yours has `farRealm` in its table.
function UnitIsSameServer(unit)
	local u = Unit(unit)
	return u ~= nil and not u.farRealm
end

-- The quest log: each entry is a table of `C_QuestLog.GetInfo`, headers included.
wow.questLog = {}

C_QuestLog = {
	GetNumQuestLogEntries = function()
		return #wow.questLog, #wow.questLog
	end,
	GetInfo = function(index)
		return wow.questLog[index]
	end,
}

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

-- Each addon message that the addon sent waits in `wow.addonSent`, as { prefix, text,
-- channel, target, event }, until a test delivers it. `event` is the event that the
-- receiver gets: CHAT_MSG_ADDON, or CHAT_MSG_ADDON_LOGGED for a logged message.
wow.addonSent = {}
wow.prefixes = {}
-- The game refuses a message with this result, for a test of the throttle.
wow.sendResult = 0
-- The results of the next sends, first in first out. Past its end, `sendResult` holds.
wow.sendResults = {}
-- The players who are offline, by full name: a whisper to one of them fails.
wow.offline = {}
-- The real client can give nil for a logged message that it took. With this true, the fake
-- does that too.
wow.loggedGivesNil = false

local function SendResult(channel, target)
	if channel == "WHISPER" and wow.offline[target] then
		return Enum.SendAddonMessageResult.TargetOffline
	end
	if #wow.sendResults > 0 then
		return table.remove(wow.sendResults, 1)
	end
	return wow.sendResult
end

-- The length of the UTF-8 letter that starts with `byte`, or nil for a byte that starts none.
local function LetterLength(byte)
	if byte < 128 then
		return 1
	end
	if byte >= 194 and byte <= 223 then
		return 2
	end
	if byte >= 224 and byte <= 239 then
		return 3
	end
	if byte >= 240 and byte <= 244 then
		return 4
	end
end

-- The code point of the letter at `at`, or nil for a broken letter.
local function CodePoint(text, at)
	local first = text:byte(at)
	local length = LetterLength(first)
	if not length then
		return nil
	end
	local point = length == 1 and first or first % (2 ^ (7 - length))
	for n = 1, length - 1 do
		local byte = text:byte(at + n)
		if not byte or byte < 128 or byte > 191 then
			return nil
		end
		point = point * 64 + byte % 64
	end
	local least = ({ 0, 128, 2048, 65536 })[length]
	if point < least or (point >= 0xD800 and point <= 0xDFFF) or point > 0x10FFFF then
		return nil
	end
	return point, length
end

-- The code points that the logged channel refuses: control characters, `|`, `\`, two
-- non-letters, and two signs that Blizzard bans (rules of the Chomp library).
local REFUSED_POINTS = { [124] = true, [92] = true, [127] = true, [0xFFFE] = true, [0xFFFF] = true }
REFUSED_POINTS[0x534D], REFUSED_POINTS[0x5350] = true, true

local function IsLoggable(text)
	local at = 1
	while at <= #text do
		local point, length = CodePoint(text, at)
		if not point or point < 32 or REFUSED_POINTS[point] then
			return false
		end
		at = at + length
	end
	return true
end

local function Sent(prefix, text, channel, target, event)
	assert(#text <= 255, "an addon message holds at most 255 bytes")
	local result = SendResult(channel, target)
	if result == 0 then
		local message = { prefix = prefix, text = text, channel = channel, target = target, event = event }
		wow.addonSent[#wow.addonSent + 1] = message
	end
	return result
end

C_ChatInfo = {
	PerformEmote = function() end,
	RegisterAddonMessagePrefix = function(prefix)
		wow.prefixes[prefix] = true
		return 0
	end,
	SendAddonMessage = function(prefix, text, channel, target)
		return Sent(prefix, text, channel, target, "CHAT_MSG_ADDON")
	end,
	-- The game refuses a text that is not plain, with InvalidMessage.
	SendAddonMessageLogged = function(prefix, text, channel, target)
		if not IsLoggable(text) then
			return Enum.SendAddonMessageResult.InvalidMessage
		end
		local result = Sent(prefix, text, channel, target, "CHAT_MSG_ADDON_LOGGED")
		if result == 0 and wow.loggedGivesNil then
			return nil
		end
		return result
	end,
}

-- Dialogs of the game: each shown one waits in `wow.popups`, with its data.
StaticPopupDialogs = {}
wow.popups = {}
function StaticPopup_Show(which, text, _, data)
	wow.popups[#wow.popups + 1] = { which = which, text = text, data = data }
end

-- Clicks Accept in the last dialog.
function wow.AcceptPopup()
	local popup = wow.popups[#wow.popups]
	StaticPopupDialogs[popup.which].OnAccept(nil, popup.data)
end

-- The right-click menus of the game. A test opens one with wow.OpenMenu, and reads or
-- clicks its items.
wow.menus = {}
Menu = {
	ModifyMenu = function(tag, callback)
		wow.menus[tag] = wow.menus[tag] or {}
		table.insert(wow.menus[tag], callback)
	end,
}

local MenuItem = {}
MenuItem.__index = MenuItem

local function NewMenuItem(parent, kind, text)
	local item = setmetatable({ kind = kind, text = text, items = {} }, MenuItem)
	if parent then
		table.insert(parent.items, item)
	end
	return item
end

function MenuItem:CreateButton(text, callback)
	local item = NewMenuItem(self, "button", text)
	item.callback = callback
	return item
end

function MenuItem:CreateRadio(text, isSelected, setSelected)
	local item = NewMenuItem(self, "radio", text)
	item.isSelected, item.callback = isSelected, setSelected
	return item
end

function MenuItem:CreateDivider()
	return NewMenuItem(self, "divider")
end

-- The menu with this tag, as the game builds it for `contextData`.
function wow.OpenMenu(tag, contextData)
	local root = NewMenuItem(nil, "root")
	for _, callback in ipairs(wow.menus[tag] or {}) do
		callback(nil, root, contextData)
	end
	return root
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
	local u = Unit(unit)
	return u ~= nil and (u.player == true or u.controlled == true)
end

-- A unit that you can attack has `hostile` in its table: a bat, a boar, an enemy.
function UnitCanAttack(_, unit)
	local u = Unit(unit)
	return u ~= nil and u.hostile == true
end

-- `creature` in the table of a unit is its creature type: { name, id }.
function UnitCreatureType(unit)
	local u = Unit(unit)
	if u and u.creature then
		return u.creature[1], u.creature[2]
	end
end

function UnitLevel(unit)
	local u = Unit(unit)
	return u and u.level or 0
end

-- The maps of the world by id: each one a { name, type, parent, layers, textures, player,
-- explored } table. `type` is its `Enum.UIMapType`, `parent` is the map that holds it,
-- `player` is the position of the player on it, and `explored` is the list of the parts that
-- the character explored. `wow.playerMap` is the map where the player stands. With no maps,
-- the pane of the journal has nothing to draw.
wow.maps = {}
wow.playerMap = nil

local function Map(id)
	return wow.maps[id]
end

C_Map = {
	GetFallbackWorldMapID = function()
		return 947
	end,
	GetMapChildrenInfo = function()
		local children = {}
		for id, map in pairs(wow.maps) do
			children[#children + 1] = { mapID = id, name = map.name }
		end
		table.sort(children, function(a, b)
			return a.mapID < b.mapID
		end)
		return children
	end,
	GetBestMapForUnit = function()
		return wow.playerMap
	end,
	GetMapInfo = function(id)
		local map = Map(id)
		return map and { mapID = id, name = map.name, mapType = map.type, parentMapID = map.parent or 0 }
	end,
	GetMapArtLayers = function(id)
		return Map(id) and Map(id).layers
	end,
	GetMapArtLayerTextures = function(id)
		return Map(id) and Map(id).textures
	end,
	GetPlayerMapPosition = function(id)
		return Map(id) and Map(id).player
	end,
}

C_MapExplorationInfo = {
	GetExploredMapTextures = function(id)
		return Map(id) and Map(id).explored
	end,
}

-- A widget keeps what the tests read: its text, scripts, events, size, anchor, and whether
-- it shows.
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

-- A new size runs OnSizeChanged, as the game does after its layout. A test calls SetSize
-- or SetWidth to stand in for a layout that gives an anchored frame its size.
function Widget:SetSize(width, height)
	local changed = width ~= self.width or height ~= self.height
	self.width, self.height = width, height
	if changed and self.scripts.OnSizeChanged then
		self.scripts.OnSizeChanged(self, width, height)
	end
end

function Widget:SetWidth(width)
	self:SetSize(width, self.height)
end

function Widget:SetHeight(height)
	self:SetSize(self.width, height)
end

function Widget:SetPoint(...)
	self.point = { ... }
end

function Widget:RegisterEvent(event)
	self.events[event] = true
end

function Widget:UnregisterEvent(event)
	self.events[event] = nil
end

-- The fake fires a unit event for every unit; a test fires it only for the unit it wants.
function Widget:RegisterUnitEvent(event)
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
	local was = self.shown
	self.shown = false
	if was and self.scripts.OnHide then
		self.scripts.OnHide(self)
	end
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

-- The first `count` letters of a UTF-8 text. A letter starts at each byte that does not
-- continue a letter.
local function FirstLetters(text, count)
	local seen = 0
	for at in text:gmatch("()[^\128-\191]") do
		if seen == count then
			return text:sub(1, at - 1)
		end
		seen = seen + 1
	end
	return text
end

-- An edit box cuts a longer text at its limit in letters, as the game does.
function Widget:SetText(text)
	if self.maxLetters and self.maxLetters > 0 then
		text = FirstLetters(text, self.maxLetters)
	end
	self.text = text
end

function Widget:SetTextColor(r, g, b)
	self.textColor = { r, g, b }
end

function Widget:SetTexture(file)
	self.file = file
end

function Widget:SetAtlas(atlas)
	self.atlas = atlas
end

function Widget:SetAlpha(alpha)
	self.alpha = alpha
end

-- The player copies the selected text of an edit box with Ctrl+C.
function Widget:HighlightText()
	self.highlighted = true
end

function Widget:SetNumeric(numeric)
	self.numeric = numeric
end

function Widget:SetMultiLine(multiLine)
	self.multiLine = multiLine
end

function Widget:SetMaxLetters(letters)
	self.maxLetters = letters
end

function Widget:SetMaxBytes(bytes)
	self.maxBytes = bytes
end

function Widget:GetNumLetters()
	local _, letters = (self.text or ""):gsub("[^\128-\191]", "")
	return letters
end

-- A button stands in for its own label.
function Widget:GetFontString()
	return self
end

-- About the width of the small quest font, and a little wider, so a test of the bounds
-- errs on the safe side.
function Widget:GetStringWidth()
	return 6 * #(self.text or "")
end

function Widget:GetText()
	return self.text
end

-- The text as the game draws it: "||" shows as one "|", and color codes show nothing. The
-- game also takes this text for Ctrl+C, as a typed "|" is kept as "||" and copies as one.
function Widget:GetDisplayText()
	local text = (self.text or ""):gsub("||", "\1"):gsub("|c%x%x%x%x%x%x%x%x", ""):gsub("|r", "")
	return (text:gsub("\1", "|"))
end

-- A line of the quest font is 14 high. A text wraps at the width of its font string, with
-- letters as wide as GetStringWidth gives them, and each line break starts a line.
local LINE_HEIGHT = 14

function Widget:GetStringHeight()
	local lines = 0
	for paragraph in ((self.text or "") .. "\n"):gmatch("(.-)\n") do
		local wide = 6 * #paragraph
		local width = self.width or 0
		lines = lines + ((width > 0 and wide > width) and math.ceil(wide / width) or 1)
	end
	return lines * LINE_HEIGHT
end

function Widget:SetEnabled(enabled)
	self.enabled = enabled
end

function Widget:IsEnabled()
	return self.enabled
end

function Widget:GetHeight()
	return self.height or 0
end

function Widget:GetVerticalScroll()
	return self.verticalScroll or 0
end

function Widget:SetVerticalScroll(offset)
	self.verticalScroll = offset
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

-- The draw layer and its sublevel decide which region draws on top. Equal ones draw in no
-- fixed order.
local function Layered(widget, layer, sublevel)
	widget.layer, widget.sublevel = layer or "ARTWORK", sublevel or 0
	return widget
end

function Widget:CreateFontString(_, layer, font)
	local text = Layered(NewWidget("FontString", nil, self), layer)
	text.font = font
	return text
end

function Widget:SetFontObject(font)
	self.font = font
end

function Widget:SetShadowOffset(x, y)
	self.shadow = { x, y }
end

function Widget:CreateTexture(_, layer, _, sublevel)
	return Layered(NewWidget("Texture", nil, self), layer, sublevel)
end

function Widget:SetColorTexture(r, g, b, a)
	self.color = { r, g, b, a }
end

local LAYERS = { BACKGROUND = 1, BORDER = 2, ARTWORK = 3, OVERLAY = 4, HIGHLIGHT = 5 }

-- A number that grows with the order of drawing, for regions of one frame.
function wow.DrawOrder(widget)
	return LAYERS[widget.layer] * 100 + widget.sublevel
end

-- The scroll frame template of the game comes with its scroll bar.
function CreateFrame(kind, name, parent, template)
	local frame = NewWidget(kind, name, parent, template)
	if template == "UIPanelScrollFrameTemplate" then
		frame.ScrollBar = NewWidget("Slider", nil, frame)
	end
	return frame
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
		wow.after[#wow.after + 1] = { seconds = seconds, callback = callback, at = wow.now + seconds }
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

wow.reloads = 0
function ReloadUI()
	wow.reloads = wow.reloads + 1
end

function GetPhysicalScreenSize()
	return 1920, 1080
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

-- The button with this label, for a click.
function wow.Button(label)
	for _, widget in ipairs(wow.widgets) do
		if widget.kind == "Button" and widget.text == label then
			return widget
		end
	end
	error("no button " .. label)
end

-- The one edit box of the book, where the player writes.
function wow.EditBox()
	for _, widget in ipairs(wow.widgets) do
		if widget.kind == "EditBox" then
			return widget
		end
	end
	error("no edit box")
end

-- The box of several lines of the editor, on a page that holds other boxes too.
function wow.MultiLineBox()
	for _, widget in ipairs(wow.widgets) do
		if widget.kind == "EditBox" and widget.multiLine then
			return widget
		end
	end
	error("no box of several lines")
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

function GetNormalizedRealmName()
	return (wow.realm:gsub("[%s-]", ""))
end

-- "none" drops the realm of a player of your own realm, as the game does.
function Ambiguate(name, _)
	local short, realm = name:match("^(.-)%-(.+)$")
	if short and realm == GetNormalizedRealmName() then
		return short
	end
	return name
end

-- True in a group of the group finder: then the group channel is INSTANCE_CHAT.
wow.instanceGroup = false
-- LE_PARTY_CATEGORY_INSTANCE of the client.
local INSTANCE_GROUP = 2

-- The group is the units "party1" to "party4" and "raid1" to "raid40" in `wow.units`.
function IsInGroup(category)
	if category == INSTANCE_GROUP then
		return wow.instanceGroup
	end
	for unit in pairs(wow.units) do
		if unit:match("^party%d") or unit:match("^raid%d") then
			return true
		end
	end
	return false
end

function IsInRaid()
	for unit in pairs(wow.units) do
		if unit:match("^raid%d") then
			return true
		end
	end
	return false
end

-- A unit with `offline` in its table is a member of the group who logged out.
function UnitIsConnected(unit)
	local u = Unit(unit)
	return u ~= nil and not u.offline
end

-- The yards of each distance of CheckInteractDistance: inspect, trade, duel, follow.
local INTERACT_YARDS = { 28, 11.11, 9.9, 28 }

-- A unit with `near` in its table stands close enough to trade. A unit with `yards` stands
-- that far away.
function CheckInteractDistance(unit, distance)
	local u = Unit(unit)
	if u == nil then
		return false
	end
	if u.near then
		return true
	end
	return u.yards ~= nil and u.yards <= INTERACT_YARDS[distance]
end

-- The guild: nil for none, or a list of { name = "Name-Realm", online }. The roster of the
-- client shows `listed` in place of `online` when a member has it: the client learns who is
-- online only when it asks the server with C_GuildInfo.GuildRoster.
wow.guild = nil
wow.rosterRequests = 0

function IsInGuild()
	return wow.guild ~= nil
end

function GetNumGuildMembers()
	return wow.guild and #wow.guild or 0
end

function GetGuildRosterInfo(index)
	local member = wow.guild[index]
	local online = member.online
	if member.listed ~= nil then
		online = member.listed
	end
	return member.name, "Member", 1, 60, "Warlock", "Brill", "", "", online
end

C_GuildInfo = {
	GuildRoster = function()
		wow.rosterRequests = wow.rosterRequests + 1
		for _, member in ipairs(wow.guild or {}) do
			member.listed = member.online
		end
		wow.Fire("GUILD_ROSTER_UPDATE", false)
	end,
}

-- The friends list: each one { name, connected }, with the name as the game shows it.
wow.friends = {}

local function FriendInfo(friend)
	return { name = friend.name, connected = friend.connected == true, guid = "Player-1-" .. friend.name, level = 60 }
end

C_FriendList = {
	GetNumFriends = function()
		return #wow.friends
	end,
	GetFriendInfoByIndex = function(index)
		return wow.friends[index] and FriendInfo(wow.friends[index])
	end,
	GetFriendInfo = function(name)
		for _, friend in ipairs(wow.friends) do
			if friend.name == name then
				return FriendInfo(friend)
			end
		end
	end,
}

-- The open trade window: `wow.trade.gave` and `wow.trade.got` list { name, count } by slot,
-- and `money` and `moneyGot` hold copper.
wow.trade = { gave = {}, got = {}, money = 0, moneyGot = 0 }

function GetTradePlayerItemInfo(slot)
	local item = wow.trade.gave[slot]
	if item then
		return item.name, 134400, item.count, 1, nil, false, false, 2589
	end
end

function GetTradeTargetItemInfo(slot)
	local item = wow.trade.got[slot]
	if item then
		return item.name, 134400, item.count, 1, true, nil, 2589
	end
end

function GetPlayerTradeMoney()
	return tostring(wow.trade.money)
end

function GetTargetTradeMoney()
	return tostring(wow.trade.moneyGot)
end

-- The names of the messages of UI_INFO_MESSAGE by index, as GetGameMessageInfo gives them.
wow.TRADE_CANCELLED, wow.TRADE_COMPLETE = 1, 2
local GAME_MESSAGES = { "ERR_TRADE_CANCELLED", "ERR_TRADE_COMPLETE" }

function GetGameMessageInfo(index)
	return GAME_MESSAGES[index]
end

-- A whole trade with the player of the "npc" unit: the window opens, both accept, and the
-- game makes the trade, empties and closes the window, and says so.
function wow.Trade(partner, trade)
	wow.units.npc = { name = partner, player = true }
	wow.trade = { gave = {}, got = {}, money = 0, moneyGot = 0 }
	wow.Fire("TRADE_SHOW")
	wow.trade = trade
	wow.Fire("TRADE_PLAYER_ITEM_CHANGED", 1)
	wow.Fire("TRADE_TARGET_ITEM_CHANGED", 1)
	wow.Fire("TRADE_MONEY_CHANGED")
	wow.Fire("TRADE_ACCEPT_UPDATE", 1, 0)
	wow.Fire("TRADE_ACCEPT_UPDATE", 1, 1)
	wow.trade = { gave = {}, got = {}, money = 0, moneyGot = 0 }
	wow.Fire("TRADE_CLOSED")
	wow.Fire("UI_INFO_MESSAGE", wow.TRADE_COMPLETE, "Trade complete.")
	wow.units.npc = nil
end

-- The item that the player holds on the cursor: { id, name, count }, or nil.
wow.cursor = nil

function GetCursorInfo()
	local item = wow.cursor
	if item then
		return "item", item.id, "|cffffffff|Hitem:" .. item.id .. "::::::::|h[" .. item.name .. "]|h|r"
	end
end

function ClearCursor()
	wow.cursor = nil
end

-- The location of an item stands in for the item on the cursor.
C_Cursor = {
	GetCursorItem = function()
		return wow.cursor and { cursor = true }
	end,
}

C_Item = {
	GetItemCount = function(item)
		return wow.bags[item] or 0
	end,
	GetStackCount = function(location)
		return location and location.cursor and wow.cursor and wow.cursor.count
	end,
	GetItemIconByID = function()
		return 134400
	end,
}

return wow
