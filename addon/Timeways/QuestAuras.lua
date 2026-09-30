-- Lasting buffs and debuffs that a quest of the game puts on you (GAMEPLAY.md 5.4). Only
-- an aura that comes just after an event of a quest counts, so the quest is the cause.

local _, ns = ...

local QuestAuras = {}
ns.QuestAuras = QuestAuras

-- Shorter auras are the buffs of a fight or a meal.
local LASTING_SECONDS = 600

-- Auras of the world that come near a quest by chance, by spell ID: Resurrection
-- Sickness, the world buffs, and the fortunes of the Darkmoon Faire.
local IGNORED = {
	[15007] = true,
	[15366] = true,
	[16609] = true,
	[22817] = true,
	[22818] = true,
	[22820] = true,
	[22888] = true,
	[23735] = true,
	[23736] = true,
	[23737] = true,
	[23738] = true,
	[23766] = true,
	[23767] = true,
	[23768] = true,
	[23769] = true,
	[24425] = true,
}

-- The spell IDs sent in this session. The desktop keeps each mark once for good.
local told = {}

-- The client can hide a value from addons. A hidden value is never compared or sent.
local function Readable(aura)
	local fields = { aura.name, aura.spellId, aura.duration, aura.sourceUnit, aura.isFromPlayerOrPlayerPet }
	for n = 1, 5 do
		if fields[n] ~= nil and issecretvalue(fields[n]) then
			return false
		end
	end
	return true
end

local function FromPlayer(aura)
	local source = aura.sourceUnit
	return aura.isFromPlayerOrPlayerPet == true or (type(source) == "string" and UnitIsPlayer(source))
end

local function Lasting(aura)
	local duration = aura.duration
	return type(duration) == "number" and (duration == 0 or duration >= LASTING_SECONDS)
end

local function IsMark(aura)
	if type(aura) ~= "table" or not Readable(aura) then
		return false
	end
	if type(aura.name) ~= "string" or type(aura.spellId) ~= "number" then
		return false
	end
	return not IGNORED[aura.spellId] and not told[aura.spellId] and not FromPlayer(aura) and Lasting(aura)
end

-- A full update is the state at login or after a load screen, not a new aura.
function QuestAuras.Changed(unit, info)
	if unit ~= "player" or type(info) ~= "table" or info.isFullUpdate or InCombatLockdown() then
		return
	end
	local quest = ns.GameQuests.Recent()
	if not quest or type(info.addedAuras) ~= "table" then
		return
	end
	for _, aura in ipairs(info.addedAuras) do
		if IsMark(aura) then
			told[aura.spellId] = true
			ns.Outbox.Add(ns.Inputs.QuestMarked(time(), quest, aura.name))
		end
	end
end
