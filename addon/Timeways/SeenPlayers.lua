-- The players that you saw lately: a target, a hover, a nameplate, and the players of your
-- group in this session (GAMEPLAY.md 5.11). A name that you saw gets a mark in a text for
-- the story program, also when the player is in neither your guild nor your friends list.
-- Only in memory, and only the newest ones.

local _, ns = ...

local SeenPlayers = {}
ns.SeenPlayers = SeenPlayers

SeenPlayers.MAX = 200

-- The keys of the game, oldest first.
local seen = {}

local function Remember(full)
	for index, known in ipairs(seen) do
		if known == full then
			table.remove(seen, index)
			break
		end
	end
	seen[#seen + 1] = full
	if #seen > SeenPlayers.MAX then
		table.remove(seen, 1)
	end
end

-- You are no player that you saw.
function SeenPlayers.See(unit)
	local full = ns.TaskPeople.OfUnit(unit)
	if full and full ~= ns.TaskPeople.Me() then
		Remember(full)
	end
end

function SeenPlayers.SeeTarget()
	SeenPlayers.See("target")
end

function SeenPlayers.SeeMouseover()
	SeenPlayers.See("mouseover")
end

function SeenPlayers.SeeGroup()
	for _, full in ipairs(ns.TaskPeople.GroupNames()) do
		Remember(full)
	end
end

function SeenPlayers.Names()
	local names = {}
	for index, full in ipairs(seen) do
		names[index] = full
	end
	return names
end
