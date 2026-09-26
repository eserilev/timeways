-- A small fake of the WoW API: only the calls that Timeways makes. It returns a `wow`
-- table that tests use to set the game state, fire events, and read the chat.

local wow = {
	now = 1790000000,
	zone = "",
	subzone = "",
	units = {},
	printed = {},
	widgets = {},
	tickers = {},
}

function time()
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

	return wow.now
end

function GetRealZoneText()
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

	return wow.zone
end

function GetSubZoneText()
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

	return wow.subzone
end

function UnitExists(unit)
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
date = os.date

C_Timer = {
	NewTicker = function(seconds, callback)
		wow.tickers[#wow.tickers + 1] = { seconds = seconds, callback = callback }
	end,
}

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
