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

-- The win of the match in progress went out already. A new match clears it.
local wonSent = false
-- The rank that went out last, so the same rank goes out once.
local rankSent

-- UPDATE_BATTLEFIELD_STATUS: a match that you won ends with a winner of your faction.
function Pvp.BattlefieldStatus()
	local inside, kind = IsInInstance()
	if not inside or kind ~= "pvp" then
		wonSent = false
		return
	end
	local winner = GetBattlefieldWinner()
	if winner == nil then
		wonSent = false
		return
	end
	if wonSent or WINNERS[winner] ~= UnitFactionGroup("player") then
		return
	end
	wonSent = true
	ns.Outbox.Add(ns.Inputs.BgWon(time(), GetRealZoneText()))
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
