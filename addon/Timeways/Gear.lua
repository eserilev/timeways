-- The items that you put on (GAMEPLAY.md 5.4). Only an item of rare quality or better goes
-- out, with the item level of what its slot held. The story program decides what is a
-- first epic item and what is a big upgrade.

local _, ns = ...

local Gear = {}
ns.Gear = Gear

-- The item quality of the game: 3 is rare, 4 is epic.
local RARE = 3
-- The inventory slots of the game. The shirt (4) and the tabard (19) are only for show.
local SLOTS = { 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18 }
local FOR_SHOW = { [4] = true, [19] = true }
-- The story program reads a level as a whole number of 16 bits.
local MAX_LEVEL = 65535

-- What each slot held last: { item, quality, level }. A slot that you empty keeps its last
-- item, so an item taken off and another put on compare with each other.
local worn = {}

local function Whole(value)
	if type(value) == "number" and value >= 0 and value <= MAX_LEVEL and value % 1 == 0 then
		return value
	end
end

-- The item in the slot, or nil for an empty slot. Its fields are nil when the game has no
-- data on the item yet, or hides it.
local function Read(slot)
	local link = GetInventoryItemLink("player", slot)
	if type(link) ~= "string" or issecretvalue(link) then
		return nil
	end
	local name, _, quality, level = C_Item.GetItemInfo(link)
	for _, value in ipairs({ name, quality, level }) do
		if issecretvalue(value) then
			return {}
		end
	end
	return { item = type(name) == "string" and name or nil, quality = Whole(quality), level = Whole(level) }
end

-- The gear at login is what each later item compares with.
function Gear.Login()
	for _, slot in ipairs(SLOTS) do
		worn[slot] = Read(slot)
	end
end

local function IsNotable(item)
	return item.item and item.item ~= "" and item.quality and item.quality >= RARE
end

function Gear.Changed(slot)
	if type(slot) ~= "number" or FOR_SHOW[slot] or slot < 1 or slot > 19 then
		return
	end
	local before = worn[slot]
	local now = Read(slot)
	if not now then
		return
	end
	worn[slot] = now
	if not IsNotable(now) or (before and before.item == now.item) then
		return
	end
	ns.Outbox.Add(ns.Inputs.ItemEquipped(time(), slot, now, before))
end
