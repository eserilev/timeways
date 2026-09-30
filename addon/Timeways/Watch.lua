-- The game events that feed the world of the character (GAMEPLAY.md 5.4). Each one
-- becomes an input in the outbox.

local _, ns = ...

local Watch = {}
ns.Watch = Watch

local lastZone, lastSubzone
-- The time of the last meeting that went out, for each NPC.
local met = {}
-- A second meeting adds nothing to the world, but a quest step to meet an NPC needs a
-- meeting after the step became the next one (GAMEPLAY.md 3.4).
local MEET_AGAIN_SECONDS = 300

function Watch.Zone()
	local zone, subzone = GetRealZoneText(), GetSubZoneText()
	-- The zone text is empty for a moment during a loading screen.
	if zone == "" or (zone == lastZone and subzone == lastSubzone) then
		return
	end
	lastZone, lastSubzone = zone, subzone
	ns.Outbox.Add(ns.Inputs.Zone(time(), zone, subzone, ns.Position.Here()))
	local kind = Watch.InstanceKind()
	if kind then
		ns.Outbox.Add(ns.Inputs.Instance(time(), zone, kind))
	end
end

-- "party" for a dungeon and "raid" for a raid. A battleground or an arena is no story
-- place, so it counts as none.
local INSTANCE_KINDS = { party = true, raid = true }

function Watch.InstanceKind()
	local inside, kind = IsInInstance()
	if inside and INSTANCE_KINDS[kind] then
		return kind
	end
end

-- A party member who shares a quest is the "npc" unit too, and a player's name stays out
-- (5.11).
function Watch.Npc()
	local name = ns.Units.NpcName("npc")
	if not name or (met[name] and time() - met[name] < MEET_AGAIN_SECONDS) then
		return
	end
	met[name] = time()
	ns.Outbox.Add(ns.Inputs.Npc(time(), name, ns.Position.Here()))
end

function Watch.ForgetMet()
	met = {}
end

function Watch.Level(level)
	ns.Outbox.Add(ns.Inputs.Level(time(), level))
end

-- The world starts from what the game shows at login.
function Watch.Login()
	ns.Outbox.SetCharacter(ns.Inputs.Character(GetRealmName(), UnitName("player")))
	Watch.Level(UnitLevel("player"))
	Watch.Zone()
	-- The tooltips need the people before the book ever opens.
	ns.Journal.Request(0)
end
