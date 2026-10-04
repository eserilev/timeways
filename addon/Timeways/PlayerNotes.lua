-- Remembered players (GAMEPLAY.md 4.9): your own mark and note on another player, from the
-- right-click menu of the player, shown in the tooltip of that player.
-- They hold real names, so they stay in the saved variables: never on the desktop, in a
-- model, or in a message to a player (rule 5 and 5.11). Other addons can read saved
-- variables, so the docs say so.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local PlayerNotes = {}
ns.PlayerNotes = PlayerNotes

local GLOBAL = "TimewaysPlayers"

PlayerNotes.MARKS = { "friendly", "neutral", "avoid" }
PlayerNotes.LABELS = { friendly = "Friendly", neutral = "Neutral", avoid = "Avoid" }
-- One line in a tooltip.
PlayerNotes.MAX_LETTERS = 100
PlayerNotes.MAX_BYTES = 120
-- The newest players stay.
PlayerNotes.MAX_PLAYERS = 300

-- The menus of the game that a player opens with a right click on another player: a unit
-- frame of a friend or a foe, a member of the group, and a name in the chat.
local MENUS =
	{ "MENU_UNIT_PLAYER", "MENU_UNIT_ENEMY_PLAYER", "MENU_UNIT_PARTY", "MENU_UNIT_RAID_PLAYER", "MENU_UNIT_FRIEND" }

local function IsMark(mark)
	return PlayerNotes.LABELS[mark] ~= nil
end

local function IsNote(note)
	return type(note) == "string" and note ~= "" and #note <= PlayerNotes.MAX_BYTES and not note:find("[%c|]")
end

-- Any addon can write the saved variables, so each entry is checked when the game loads
-- them. An entry with no mark and no note says nothing, and goes too.
local function CleanEntry(name, entry)
	if type(entry) ~= "table" or ns.TaskPeople.Full(name) ~= name or type(entry.at) ~= "number" then
		return nil
	end
	local mark = IsMark(entry.mark) and entry.mark or nil
	local note = IsNote(entry.note) and entry.note or nil
	if not mark and not note then
		return nil
	end
	return { mark = mark, note = note, at = entry.at }
end

local function Oldest(players)
	local oldest
	for name, entry in pairs(players) do
		if not oldest or entry.at < players[oldest].at then
			oldest = name
		end
	end
	return oldest
end

local function CountOf(players)
	local count = 0
	for _ in pairs(players) do
		count = count + 1
	end
	return count
end

local function Trim(players)
	local count = CountOf(players)
	while count > PlayerNotes.MAX_PLAYERS do
		players[Oldest(players)] = nil
		count = count - 1
	end
end

local function Checked(value)
	local players = {}
	for name, entry in pairs(type(value) == "table" and value or {}) do
		players[name] = CleanEntry(name, entry)
	end
	Trim(players)
	return players
end

local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
local function Players()
	if not checked or _G[GLOBAL] ~= checked then
		checked = Checked(_G[GLOBAL])
		_G[GLOBAL] = checked
	end
	return checked
end

function PlayerNotes.Of(name)
	return Players()[name or ""]
end

-- `change` holds the new mark or note. An entry left with neither is forgotten.
local function Change(name, change)
	local players = Players()
	local entry = players[name] or {}
	entry.mark, entry.note = change.mark, change.note
	entry.at = time()
	players[name] = (entry.mark or entry.note) and entry or nil
	Trim(players)
end

function PlayerNotes.SetMark(name, mark)
	local entry = Players()[name] or {}
	Change(name, { mark = IsMark(mark) and mark or nil, note = entry.note })
end

-- An empty note clears it.
function PlayerNotes.SetNote(name, note)
	local entry = Players()[name] or {}
	Change(name, { mark = entry.mark, note = IsNote(note) and note or nil })
end

function PlayerNotes.Forget(name)
	Players()[name] = nil
end

-- "Avoid: Ninja-looted the chest.", or nil for a player with no mark and no note.
function PlayerNotes.Line(name)
	local entry = PlayerNotes.Of(name)
	if not entry then
		return nil
	end
	local mark = entry.mark and PlayerNotes.LABELS[entry.mark]
	if mark and entry.note then
		return mark .. ": " .. entry.note
	end
	return mark or entry.note
end

local NO_PIPE = "Notes can't hold the | sign. Take it out to save."
local TOO_LONG = "Too long to save. Try a shorter version."

local function Clean(text)
	local flat = tostring(text or ""):gsub("%s*[\r\n]+%s*", " ")
	return (flat:match("^%s*(.-)%s*$"))
end

function PlayerNotes.Problem(text)
	text = Clean(text)
	if text:find("|", 1, true) then
		return NO_PIPE
	end
	if #text > PlayerNotes.MAX_BYTES then
		return TOO_LONG
	end
end

function PlayerNotes.EditNote(name)
	local entry = PlayerNotes.Of(name)
	ns.JournalFrame.Edit({
		title = ns.TaskPeople.Short(name),
		hint = "A note about this player. Only you can see it.",
		text = entry and entry.note or "",
		limit = PlayerNotes.MAX_LETTERS,
		bytes = PlayerNotes.MAX_BYTES,
		problem = PlayerNotes.Problem,
		save = function(text)
			PlayerNotes.SetNote(name, Clean(text))
		end,
	})
end

-- The player of a menu: its unit when it has one, else the name and the realm.
local function MenuPlayer(contextData)
	if type(contextData) ~= "table" then
		return nil
	end
	if type(contextData.unit) == "string" and not issecretvalue(contextData.unit) then
		local name = ns.TaskPeople.OfUnit(contextData.unit)
		if name then
			return name
		end
	end
	local name, server = contextData.name, contextData.server
	if type(name) ~= "string" or issecretvalue(name) then
		return nil
	end
	if type(server) == "string" and server ~= "" and not issecretvalue(server) and not name:find("-", 1, true) then
		name = name .. "-" .. server
	end
	return ns.TaskPeople.Full(name)
end

local function AddMarks(submenu, name)
	for _, mark in ipairs(PlayerNotes.MARKS) do
		submenu:CreateRadio(PlayerNotes.LABELS[mark], function()
			local entry = PlayerNotes.Of(name)
			return entry ~= nil and entry.mark == mark
		end, function()
			PlayerNotes.SetMark(name, mark)
		end)
	end
end

-- "Remember" in the menu of a player: a mark, a note, and Forget.
function PlayerNotes.AddToMenu(_, rootDescription, contextData)
	local name = MenuPlayer(contextData)
	if not name or name == ns.TaskPeople.Me() then
		return
	end
	local entry = PlayerNotes.Of(name)
	rootDescription:CreateDivider()
	local submenu = rootDescription:CreateButton("Remember")
	AddMarks(submenu, name)
	submenu:CreateDivider()
	submenu:CreateButton(entry and entry.note and "Edit note" or "Add a note", function()
		PlayerNotes.EditNote(name)
	end)
	if entry then
		submenu:CreateButton("Forget", function()
			PlayerNotes.Forget(name)
		end)
	end
end

-- Only the game's own tooltip gets the line, and only for another player.
function PlayerNotes.OnTooltip(tooltip)
	if tooltip ~= GameTooltip then
		return
	end
	local _, unit = tooltip:GetUnit()
	if not unit or issecretvalue(unit) then
		return
	end
	local line = PlayerNotes.Line(ns.TaskPeople.OfUnit(unit))
	if line then
		tooltip:AddLine(line, 0.78, 0.63, 0.39)
	end
end

for _, tag in ipairs(MENUS) do
	Menu.ModifyMenu(tag, PlayerNotes.AddToMenu)
end

TooltipDataProcessor.AddTooltipPostCall(Enum.TooltipDataType.Unit, PlayerNotes.OnTooltip)
