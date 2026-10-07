-- The mounts that you ride (GAMEPLAY.md 5.4). The game names no mount when you mount up,
-- so the name comes from the aura that you got just before, whose spell the mount journal
-- knows as a mount. A paladin aura or a hunter aspect has no end either, so only the
-- journal tells them apart. The story program keeps only the first mount and the first epic
-- mount.

local _, ns = ...

local Mounts = {}
ns.Mounts = Mounts

-- The mount aura and the mounted state come within this many seconds of each other.
local AURA_SECONDS = 3
-- The run speed takes a moment to change after you mount up.
local SETTLE_SECONDS = 1
-- The run speed on foot, in yards a second. A mount of level 40 runs at 160 percent of it,
-- and an epic mount at 200.
local RUN_SPEED = 7

-- The newest mount aura that you got: { name, at }.
local lastMountAura
-- The mounts sent in this session. The desktop keeps the first ones for good.
local told = {}

-- The client can hide a value from addons. A hidden value is never compared or sent.
local function Hidden(aura)
	return issecretvalue(aura.name) or issecretvalue(aura.spellId)
end

local function IsMount(spell)
	local mount = C_MountJournal.GetMountFromSpell(spell)
	return type(mount) == "number" and not issecretvalue(mount)
end

local function IsMountAura(aura)
	if type(aura) ~= "table" or Hidden(aura) then
		return false
	end
	local named = type(aura.name) == "string" and aura.name ~= ""
	return named and type(aura.spellId) == "number" and IsMount(aura.spellId)
end

-- A full update is the state at login or after a load screen, not a new aura. While the game
-- restricts auras, it hides the whole update, and a test of a hidden field is a Lua error.
function Mounts.AuraChanged(unit, info)
	if unit ~= "player" or type(info) ~= "table" then
		return
	end
	local full, added = info.isFullUpdate, info.addedAuras
	if issecretvalue(full) or issecretvalue(added) or full or type(added) ~= "table" then
		return
	end
	for _, aura in ipairs(added) do
		if IsMountAura(aura) then
			lastMountAura = { name = aura.name, at = time() }
		end
	end
end

-- The run speed in percent of a run on foot, or nil when the game hides it.
local function SpeedPercent()
	local _, run = GetUnitSpeed("player")
	if type(run) ~= "number" or issecretvalue(run) then
		return nil
	end
	return math.floor(run / RUN_SPEED * 100 + 0.5)
end

function Mounts.Check()
	if not IsMounted() or not lastMountAura or time() - lastMountAura.at > AURA_SECONDS then
		return
	end
	local mount = lastMountAura.name
	if told[mount] then
		return
	end
	told[mount] = true
	ns.Outbox.Add(ns.Inputs.MountRidden(time(), mount, SpeedPercent()))
end

-- The event fires when you mount up and when you get down.
function Mounts.DisplayChanged()
	if IsMounted() then
		C_Timer.After(SETTLE_SECONDS, Mounts.Check)
	end
end
