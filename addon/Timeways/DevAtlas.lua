-- The commands of `/twdev` for Knowledge (TESTING.md, "Dev mode"): a short tour of the
-- zone where you stand, with points on its real map, so the atlas shows full with no hours
-- of play, and a page of a place that you never visited.

local _, ns = ...

local Dev = ns.Dev

-- The places of the tour, each with its point on the map of the zone in thousandths, and
-- the person that you meet there.
local TOUR = {
	{ place = "Dev Camp", x = 300, y = 400, npc = "Dev Scout" },
	{ place = "Dev Ruins", x = 650, y = 300, npc = "Dev Keeper" },
	{ place = "Dev Tower", x = 500, y = 700, npc = "Dev Hermit" },
}

local function Add(input)
	ns.Outbox.Add(input)
end

local function Spot(map, x, y)
	return map and { map = map, x = x, y = y } or nil
end

-- In each place: you walk in and meet its person. Then what the zone holds: a talk in
-- passing, a quest text and its turn-in, a book, a death, and a rare that you defeat.
local function Tour(zone)
	local map = ns.MapPane.MapFor(zone)
	for _, stop in ipairs(TOUR) do
		Add(ns.Inputs.Zone(time(), zone, stop.place, Spot(map, stop.x, stop.y), nil))
		Add(ns.Inputs.Npc(time(), stop.npc, Spot(map, stop.x + 20, stop.y + 20)))
	end
	local scout = TOUR[1].npc
	Add(ns.Inputs.Seen(time(), "gossip", nil, scout, zone, "Keep to the road after dark, $N."))
	Add(ns.Inputs.Seen(time(), "quest", "Dev Errand", scout, zone, "Bring word to the camp, $N."))
	Add(ns.Inputs.GameQuestDone(time(), "Dev Errand", "normal"))
	Add(ns.Inputs.Seen(time(), "book", "Dev Chronicle", nil, zone, "The old tower once watched the road."))
	Add(ns.Inputs.Died(time(), "Dev Wolf", nil, nil, ns.Inputs.Hour()))
	Add(ns.Inputs.Defeated(time(), "Dev Rare", "rare"))
end

Dev.Add("atlas", "atlas [empty]: tour three places of your zone, or open a place you never saw.", function(rest)
	if rest == "empty" then
		ns.JournalKnowledge.Show("place", "Dev Wilds")
		return
	end
	Tour(GetRealZoneText())
	ns.Journal.Request(0)
	ns.JournalFrame.Open("knowledge")
end)
