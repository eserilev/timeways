-- The past of the character before Timeways saw it (GAMEPLAY.md 3.3, the prologue). The
-- journal says "wanted" until the desktop keeps a past, so the addon reads it once in a
-- life. The client can hide any value from addons, so each one passes `issecretvalue`.

local _, ns = ...

local Past = {}
ns.Past = Past

-- The limits of the line (crates/story/src/past.rs).
local MAX_QUEST_TITLES = 8
local MAX_ZONES = 48
local MAX_FACTIONS = 8
local MAX_PROFESSIONS = 6
local MAX_MOUNTS = 3

-- The server sends the quest titles and the played time a moment after the ask.
local WAIT_SECONDS = 3

-- The standings of the game run from 1 (hated) to 8 (exalted).
local NEUTRAL = 4
local EXALTED = 8

-- The category of a skill line: 9 is a secondary skill, such as Fishing, and 11 is a
-- profession.
local PROFESSION_CATEGORIES = { [9] = true, [11] = true }

local RARE = 3
local EPIC = 4
-- The shirt (4) and the tabard (19) are only for show.
local SLOTS = { 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18 }

local asked = false
local played

local function Name(value)
	if type(value) == "string" and value ~= "" and not issecretvalue(value) then
		return value
	end
end

local function Whole(value)
	if type(value) == "number" and not issecretvalue(value) and value >= 0 and value % 1 == 0 then
		return value
	end
end

local function Yes(value)
	return value == true and not issecretvalue(value)
end

-- An empty Lua table goes out as a JSON object, so an empty list goes as nothing.
local function List(items)
	if #items > 0 then
		return items
	end
end

-- The ids of the finished quests, the newest first: a higher id is a later quest of the
-- game, most of the time.
local function CompletedQuests()
	local ids = {}
	for _, id in ipairs(C_QuestLog.GetAllCompletedQuestIDs() or {}) do
		if Whole(id) then
			ids[#ids + 1] = id
		end
	end
	table.sort(ids, function(a, b)
		return a > b
	end)
	return ids
end

local function LoadTitles(ids)
	for index = 1, math.min(#ids, MAX_QUEST_TITLES) do
		C_QuestLog.RequestLoadQuestByID(ids[index])
	end
end

local function Titles(ids)
	local titles = {}
	for index = 1, math.min(#ids, MAX_QUEST_TITLES) do
		titles[#titles + 1] = Name(C_QuestLog.GetTitleForQuestID(ids[index]))
	end
	return titles
end

-- A zone of the map with a part that the character explored.
local function Discovered(map)
	local info = C_Map.GetMapInfo(map.mapID)
	if not info or info.mapType ~= Enum.UIMapType.Zone then
		return nil
	end
	local explored = C_MapExplorationInfo.GetExploredMapTextures(map.mapID)
	if type(explored) == "table" and #explored > 0 then
		return Name(map.name)
	end
end

local function Zones()
	local zones, seen = {}, {}
	local world = C_Map.GetFallbackWorldMapID()
	for _, map in ipairs(C_Map.GetMapChildrenInfo(world, Enum.UIMapType.Zone, true) or {}) do
		local name = Discovered(map)
		if name and not seen[name] and #zones < MAX_ZONES then
			seen[name] = true
			zones[#zones + 1] = name
		end
	end
	return zones
end

local function Standing(data)
	local standing = Whole(data.reaction)
	local shown = not Yes(data.isHeader) or Yes(data.isHeaderWithRep)
	if shown and standing and standing >= 1 and standing <= EXALTED and standing ~= NEUTRAL then
		return standing
	end
end

-- The factions furthest from neutral first, then by name.
local function Furthest(a, b)
	local da, db = math.abs(a.standing - NEUTRAL), math.abs(b.standing - NEUTRAL)
	if da ~= db then
		return da > db
	end
	return a.name < b.name
end

local function Factions()
	local factions = {}
	for index = 1, Whole(C_Reputation.GetNumFactions()) or 0 do
		local data = C_Reputation.GetFactionDataByIndex(index)
		local name = data and Name(data.name)
		local standing = data and Standing(data)
		if name and standing then
			factions[#factions + 1] = { name = name, standing = standing }
		end
	end
	table.sort(factions, Furthest)
	for index = #factions, MAX_FACTIONS + 1, -1 do
		factions[index] = nil
	end
	return factions
end

local function Professions()
	local professions = {}
	for index = 1, Whole(C_SkillInfo.GetNumSkillLines()) or 0 do
		local info = C_SkillInfo.GetSkillLineInfo(index)
		local name = info and Name(info.name)
		local rank = info and Whole(info.rank)
		local profession = info and PROFESSION_CATEGORIES[Whole(info.skillLineCategoryID) or 0]
		if name and rank and profession and not Yes(info.isHeader) and #professions < MAX_PROFESSIONS then
			professions[#professions + 1] = { name = name, rank = rank }
		end
	end
	return professions
end

local function Mounts()
	local mounts = {}
	for _, id in ipairs(C_MountJournal.GetMountIDs() or {}) do
		local name, _, _, _, _, _, _, _, _, hidden, collected = C_MountJournal.GetMountInfoByID(id)
		name = Name(name)
		if name and Yes(collected) and not Yes(hidden) and #mounts < MAX_MOUNTS then
			mounts[#mounts + 1] = name
		end
	end
	return mounts
end

local function Gear()
	local gear = { rare = 0, epic = 0 }
	for _, slot in ipairs(SLOTS) do
		local quality = Whole(GetInventoryItemQuality("player", slot))
		if quality and quality >= EPIC then
			gear.epic = gear.epic + 1
		elseif quality == RARE then
			gear.rare = gear.rare + 1
		end
	end
	return gear
end

local function Send(ids)
	ns.Outbox.Add(ns.Inputs.Past(time(), {
		level = Whole(UnitLevel("player")),
		zone = Name(GetRealZoneText()),
		played = played,
		quests = #ids,
		quest_titles = List(Titles(ids)),
		zones = List(Zones()),
		factions = List(Factions()),
		professions = List(Professions()),
		mounts = List(Mounts()),
		gear = Gear(),
	}))
end

-- The journal says "wanted" until the desktop keeps a past. It asks the server for the
-- quest titles and the played time once, and the line goes a moment later.
function Past.JournalSays(value)
	if value ~= "wanted" or asked then
		return
	end
	asked = true
	local ids = CompletedQuests()
	LoadTitles(ids)
	RequestTimePlayed()
	C_Timer.After(WAIT_SECONDS, function()
		Send(ids)
	end)
end

-- TIME_PLAYED_MSG: the time played in all, in seconds.
function Past.TimePlayed(total)
	played = Whole(total)
end
