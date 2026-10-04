-- Your own roleplay profile for MSP (GAMEPLAY.md 3.7.1): the switch, the copy of the
-- shared fields that answers while the desktop is off, and the import from another
-- roleplay addon.
-- Other addons can read saved variables, so the copy holds only what MSP shares anyway, and
-- only while the switch is on.

-- The globals come from the TOC and from other roleplay addons, so only _G can reach them.
--# selene: allow(global_usage)

local _, ns = ...

local MspProfile = {}
ns.MspProfile = MspProfile

local GLOBAL = "TimewaysProfile"

-- The field of MSP for each field of the sheet that goes out. Goal, bond, flaw, and traits
-- have none, and stay private.
MspProfile.CODES = {
	origin = "HB",
	background = "HI",
	name = "NA",
	title = "NT",
	currently = "CU",
	appearance = "DE",
	age = "AG",
	motto = "MO",
}

-- The texts of the sheet in the newest journal, by field.
local sheet = {}
-- The text of each field that this session imported, so a refused import never loops.
local imported = {}

local function FieldOf(code)
	for field, fieldCode in pairs(MspProfile.CODES) do
		if fieldCode == code then
			return field
		end
	end
end

-- Any addon can write the saved variables, so each value is checked when the game loads them.
local function Checked(value)
	local data = type(value) == "table" and value or {}
	local fields = type(data.fields) == "table" and data.fields or {}
	local clean = {}
	for code, text in pairs(fields) do
		local field = FieldOf(code)
		local fits = field and type(text) == "string" and #text <= ns.Hero.LIMITS[field].bytes
		if fits and not text:find("%c") then
			clean[code] = text
		end
	end
	return { share = data.share == true, fields = clean }
end

local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
local function Data()
	if not checked or _G[GLOBAL] ~= checked then
		checked = Checked(_G[GLOBAL])
		_G[GLOBAL] = checked
	end
	return checked
end

-- The name of the roleplay addon that owns MSP, or nil when Timeways is alone. LibMSP keeps
-- its data in the global `msp`, and each roleplay addon sets `msp_RPAddOn` to its name.
function MspProfile.Owner()
	if type(_G.msp) ~= "table" then
		return nil
	end
	local name = _G.msp_RPAddOn
	if type(name) == "string" and name ~= "" and not name:find("[%c|]") then
		return name
	end
	return "your roleplay addon"
end

function MspProfile.IsSharing()
	return Data().share and not MspProfile.Owner()
end

local function CopySheet()
	local fields = {}
	for field, code in pairs(MspProfile.CODES) do
		fields[code] = sheet[field]
	end
	Data().fields = fields
end

-- Turning the switch off clears the copy.
function MspProfile.SetSharing(on)
	local data = Data()
	data.share = on == true
	data.fields = {}
	if data.share then
		CopySheet()
	end
	ns.JournalFrame.Refresh()
end

-- A field that the player saved. An empty text clears it.
function MspProfile.Edited(field, text)
	sheet[field] = text ~= "" and text or nil
	if Data().share then
		CopySheet()
	end
end

-- The fields of MSP that Timeways shares, by code. The name of the game stands in for an
-- empty name, as in every roleplay addon.
function MspProfile.Fields()
	local fields = {}
	for code, text in pairs(Data().fields) do
		fields[code] = text
	end
	fields.NA = fields.NA or UnitName("player")
	return fields
end

-- Color codes, TRP3 tags such as "{h1}", and line breaks make no sense on the sheet.
local function Plain(text)
	text = text:gsub("|c%x%x%x%x%x%x%x%x", ""):gsub("|r", ""):gsub("{.-}", "")
	text = text:gsub("[%c%s]+", " ")
	return (text:match("^%s*(.-)%s*$"))
end

-- The start of a text that fits a limit, cut between letters.
local function Cut(text, limit)
	local letters, last = 0, 0
	for start, letter in text:gmatch("()([%z\1-\127\194-\244][\128-\191]*)") do
		local stop = start + #letter - 1
		if letters >= limit.letters or stop > limit.bytes then
			break
		end
		letters, last = letters + 1, stop
	end
	return text:sub(1, last)
end

local function Imported(field, value)
	local text = type(value) == "string" and Plain(value) or ""
	return Cut(text, ns.Hero.LIMITS[field])
end

-- The profile of the other roleplay addon goes to the desktop when it changed. The six
-- questions stay the player's own, so only the roleplay fields come in.
local function Import()
	local my = type(_G.msp.my) == "table" and _G.msp.my or {}
	for _, field in ipairs(ns.Hero.PROFILE) do
		local text = Imported(field, my[MspProfile.CODES[field]])
		if text ~= (sheet[field] or "") and text ~= imported[field] then
			imported[field] = text
			ns.Hero.Set(field, text)
		end
	end
end

-- The sheet of each whole journal: the copy follows it, and an import compares with it.
function MspProfile.JournalCame(fields)
	sheet = {}
	for _, field in ipairs(type(fields) == "table" and fields or {}) do
		if type(field) == "table" and type(field.field) == "string" and type(field.text) == "string" then
			sheet[field.field] = field.text
		end
	end
	if MspProfile.Owner() then
		Import()
	elseif Data().share then
		CopySheet()
	end
end

-- The short state of the profile, for the list of the Hero page.
function MspProfile.State()
	local owner = MspProfile.Owner()
	if owner then
		return "From " .. owner
	end
	return Data().share and "Shared" or "Not shared"
end
