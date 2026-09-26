-- The game events that feed the world of the character (GAMEPLAY.md 5.4). Each one
-- becomes an input in the outbox.

local _, ns = ...

local Watch = {}
ns.Watch = Watch

local lastZone, lastSubzone
local met = {}

function Watch.Zone()
	local zone, subzone = GetRealZoneText(), GetSubZoneText()
	-- The zone text is empty for a moment during a loading screen.
	if zone == "" or (zone == lastZone and subzone == lastSubzone) then
		return
	end
	lastZone, lastSubzone = zone, subzone
	ns.Outbox.Add(ns.Inputs.Zone(time(), zone, subzone))
end

-- The world adds nothing for a second meeting, so one per UI session is enough. A party
-- member who shares a quest is the "npc" unit too, and a player's name stays out (5.11).
function Watch.Npc()
	local name = UnitName("npc")
	if not name or met[name] or UnitIsPlayer("npc") then
		return
	end
	met[name] = true
	ns.Outbox.Add(ns.Inputs.Npc(time(), name))
end

function Watch.Level(level)
	ns.Outbox.Add(ns.Inputs.Level(time(), level))
end

-- The world starts from what the game shows at login.
function Watch.Login()
	ns.Outbox.SetCharacter(ns.Inputs.Character(GetRealmName(), UnitName("player")))
	Watch.Level(UnitLevel("player"))
	Watch.Zone()
end
