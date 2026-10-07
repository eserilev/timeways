-- Battleground wins and the PvP rank (docs/plans/chapters.md 9). A first win and a new rank
-- give weight to a tale or a chapter. This client has no UnitPVPRank: the rank is the
-- renown of one faction, as the client's own PVPRankFrame reads it.

local _, ns = ...

local Pvp = {}
ns.Pvp = Pvp

-- The faction whose renown is the PvP rank, in PVPRankFrame.lua of the client.
local RANK_FACTION = 2800

-- GetBattlefieldWinner gives 0 for the Horde and 1 for the Alliance.
local WINNERS = { [0] = "Horde", [1] = "Alliance" }

-- A /reload on the score screen fires the win again, so the wins live in the saved variables
-- as { zone, won }, newest last.
local MAX_WINS = 10
-- Two wins in one battleground this close are one match. It covers the score screen of two
-- minutes. The next match takes longer: the queue, the preparation, and the match itself.
-- The run time of the battleground is no help: the game hides it in a fight.
local SAME_MATCH_SECONDS = 150

-- The saved wins, less any entry that is broken. Any addon can write saved variables.
local function SavedWins()
	local wins = {}
	local saved = ns.Saved().bgWins
	for _, win in ipairs(type(saved) == "table" and saved or {}) do
		if type(win) == "table" and type(win.zone) == "string" and type(win.won) == "number" then
			wins[#wins + 1] = win
		end
	end
	return wins
end

local function WasSent(wins, zone, won)
	for _, win in ipairs(wins) do
		if win.zone == zone and math.abs(win.won - won) <= SAME_MATCH_SECONDS then
			return true
		end
	end
	return false
end

local function Remember(wins, zone, won)
	wins[#wins + 1] = { zone = zone, won = won }
	while #wins > MAX_WINS do
		table.remove(wins, 1)
	end
	ns.Saved().bgWins = wins
end

-- The rank that went out last, so the same rank goes out once.
local rankSent

-- UPDATE_BATTLEFIELD_STATUS: a match that you won ends with a winner of your faction.
function Pvp.BattlefieldStatus()
	local inside, kind = IsInInstance()
	if not inside or kind ~= "pvp" then
		return
	end
	local winner = GetBattlefieldWinner()
	if winner == nil or WINNERS[winner] ~= UnitFactionGroup("player") then
		return
	end
	local zone, won, wins = GetRealZoneText(), time(), SavedWins()
	if WasSent(wins, zone, won) then
		return
	end
	Remember(wins, zone, won)
	ns.Outbox.Add(ns.Inputs.BgWon(won, zone))
end

-- The renown level of the rank faction, or nil while the client does not know it.
local function Rank()
	local info = C_MajorFactions.GetMajorFactionProgressionInfo(RANK_FACTION)
	local rank = type(info) == "table" and info.renownLevel
	if type(rank) == "number" and rank >= 0 then
		return rank
	end
end

-- At login and at MAJOR_FACTION_RENOWN_LEVEL_CHANGED. A rank of 0 goes out too, so the first
-- rank that you earn counts as new.
function Pvp.RankChanged()
	local rank = Rank()
	if rank == nil or rank == rankSent then
		return
	end
	rankSent = rank
	ns.Outbox.Add(ns.Inputs.PvpRank(time(), rank))
end
