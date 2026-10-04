-- The count of the item of an open carry step, at a meeting with its NPC
-- (docs/plans/quest-variety.md 4.9). Only that item, only then: Timeways never watches your
-- bags.

local _, ns = ...

local Carry = {}
ns.Carry = Carry

-- A gossip window that opens and closes fast sends one count.
local SAME_COUNT_SECONDS = 10
local sentAt = {}

-- The count of the item in your bags, not the bank. A hidden count or one that is not a
-- whole number is no count.
function Carry.InBags(item)
	local count = C_Item.GetItemCount(item)
	if issecretvalue(count) or type(count) ~= "number" or count < 0 or count % 1 ~= 0 then
		return nil
	end
	return count
end

-- You meet this NPC: a gossip window or /talk.
function Carry.Met(npc)
	local now = time()
	for _, item in ipairs(ns.QuestSteps.CarriesFor(npc)) do
		local key = npc .. "\0" .. item
		local count = Carry.InBags(item)
		if count and not (sentAt[key] and now - sentAt[key] < SAME_COUNT_SECONDS) then
			sentAt[key] = now
			ns.Outbox.Add(ns.Inputs.ItemsHeld(now, npc, item, count))
		end
	end
end
