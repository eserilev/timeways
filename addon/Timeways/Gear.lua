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

-- What each slot held last: { link, item, quality, level }. A slot that you empty keeps its
-- last item, so an item taken off and another put on compare with each other.
local worn = {}
-- The equips that wait for the data of an item, by slot: { before, at }.
local waiting = {}
-- The game can never send the data of an item. An equip that waits longer goes, so it
-- never goes out late as news.
local WAIT_SECONDS = 60

local function Whole(value)
	if type(value) == "number" and value >= 0 and value <= MAX_LEVEL and value % 1 == 0 then
		return value
	end
end

-- The item of the link. Its fields are nil while the game has no data on it. A hidden item
-- has no fields and no link, so nothing waits for it.
local function ReadLink(link)
	local name, _, quality, level = C_Item.GetItemInfo(link)
	if issecretvalue(name) or issecretvalue(quality) or issecretvalue(level) then
		return {}
	end
	local item = type(name) == "string" and name or nil
	return { link = link, item = item, quality = Whole(quality), level = Whole(level) }
end

-- The item in the slot, or nil for an empty slot.
local function Read(slot)
	local link = GetInventoryItemLink("player", slot)
	if type(link) ~= "string" or issecretvalue(link) then
		return nil
	end
	return ReadLink(link)
end

-- GET_ITEM_INFO_RECEIVED comes once the game has the data.
local function IsLoading(item)
	return item ~= nil and item.link ~= nil and item.item == nil
end

-- In place, so each table that holds the item gets the data.
local function Fill(item)
	if not IsLoading(item) then
		return
	end
	local read = ReadLink(item.link)
	item.link, item.item, item.quality, item.level = read.link, read.item, read.quality, read.level
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

local function Equipped(slot, now, before)
	if not IsNotable(now) or (before and before.item == now.item) then
		return
	end
	ns.Outbox.Add(ns.Inputs.ItemEquipped(time(), slot, now, before))
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
	if IsLoading(now) or IsLoading(before) then
		waiting[slot] = { before = before, at = time() }
		return
	end
	waiting[slot] = nil
	Equipped(slot, now, before)
end

-- GET_ITEM_INFO_RECEIVED: the items with no data get it, and an equip that waited goes out,
-- unless it waited too long.
-- A slot that changed again waits for its newest item only.
function Gear.ItemLoaded()
	for _, item in pairs(worn) do
		Fill(item)
	end
	for slot, wait in pairs(waiting) do
		Fill(wait.before)
		local now = worn[slot]
		if time() - wait.at > WAIT_SECONDS then
			waiting[slot] = nil
		elseif not IsLoading(now) and not IsLoading(wait.before) then
			waiting[slot] = nil
			Equipped(slot, now, wait.before)
		end
	end
end
