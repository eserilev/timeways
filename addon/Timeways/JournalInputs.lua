-- The inputs that a line of the journal page can hold: a box to type in, the money boxes,
-- and item slots, in the look of the trade window. JournalFrame draws the rest of a line.
--   field: { value, letters, change = function(text), submit = function() or nil }
--   money: { coins = { gold, silver, copper }, change = function(gold, silver, copper) }
--   slots: { items, size, drop = function(), remove = function(index) }

local _, ns = ...

local JournalInputs = {}
ns.JournalInputs = JournalInputs

local BOX_HEIGHT = 22
local BOX_PADDING = 6
local COIN_BOX, COIN_ICON, COIN_GAP = 58, 14, 10
local SLOT, SLOT_GAP = 37, 5
local COINS = {
	"Interface\\MoneyFrame\\UI-GoldIcon",
	"Interface\\MoneyFrame\\UI-SilverIcon",
	"Interface\\MoneyFrame\\UI-CopperIcon",
}
local EMPTY_SLOT = "Interface\\PaperDoll\\UI-Backpack-EmptySlot"

-- Each kind keeps its widgets, and a page draws them in order from the first.
local pools = { field = {}, money = {}, slots = {} }
local used = { field = 0, money = 0, slots = 0 }

local function Next(kind, build, page)
	used[kind] = used[kind] + 1
	local pool = pools[kind]
	pool[used[kind]] = pool[used[kind]] or build(page)
	return pool[used[kind]]
end

-- The page draws again at each letter that the player types. A box changes its text only
-- when the task changed it, such as an Add that empties the step box.
local function Fill(box, text)
	if box:GetText() ~= text then
		box:SetText(text)
	end
end

-- Dark ink on a shade of the parchment, as the box of the editor.
local function Box(parent)
	local box = CreateFrame("EditBox", nil, parent)
	local ink = ns.Ink.text
	local shade = box:CreateTexture(nil, "BACKGROUND")
	shade:SetAllPoints(box)
	shade:SetColorTexture(ink[1], ink[2], ink[3], 0.12)
	box:SetAutoFocus(false)
	box:SetHeight(BOX_HEIGHT)
	box:SetFontObject("QuestFont")
	box:SetTextColor(ink[1], ink[2], ink[3])
	box:SetTextInsets(BOX_PADDING, BOX_PADDING, 0, 0)
	box:SetScript("OnEscapePressed", box.ClearFocus)
	return box
end

-- Fields -------------------------------------------------------------------------------------

local function ShowPlaceholder(box)
	box.placeholder:SetShown((box:GetText() or "") == "")
end

local function BuildField(page)
	local box = Box(page)
	local placeholder = box:CreateFontString(nil, "ARTWORK", "QuestFont")
	local faded = ns.Ink.faded
	placeholder:SetTextColor(faded[1], faded[2], faded[3])
	placeholder:SetPoint("LEFT", box, "LEFT", BOX_PADDING, 0)
	box.placeholder = placeholder
	box:SetScript("OnTextChanged", function(self, userInput)
		ShowPlaceholder(self)
		if userInput then
			self.field.change(self:GetText())
		end
	end)
	box:SetScript("OnEnterPressed", function(self)
		if self.field.submit then
			self.field.submit()
		else
			self:ClearFocus()
		end
	end)
	return box
end

local function Field(page, line, left, width, y)
	local box = Next("field", BuildField, page)
	box.field = line.field
	box:ClearAllPoints()
	box:SetPoint("TOPLEFT", page, "TOPLEFT", left, -y)
	box:SetWidth(width)
	box:SetMaxLetters(line.field.letters)
	box.placeholder:SetText(line.text)
	Fill(box, line.field.value)
	ShowPlaceholder(box)
	box:Show()
	return BOX_HEIGHT
end

-- Money --------------------------------------------------------------------------------------

local function BuildMoney(page)
	local money = CreateFrame("Frame", nil, page)
	money.boxes = {}
	local function Changed(_, userInput)
		if userInput then
			local boxes = money.boxes
			money.value.change(boxes[1]:GetText(), boxes[2]:GetText(), boxes[3]:GetText())
		end
	end
	for n, file in ipairs(COINS) do
		local box = Box(money)
		box:SetNumeric(true)
		box:SetSize(COIN_BOX, BOX_HEIGHT)
		box:SetPoint("LEFT", money, "LEFT", (n - 1) * (COIN_BOX + COIN_ICON + COIN_GAP), 0)
		box:SetScript("OnTextChanged", Changed)
		local coin = money:CreateTexture(nil, "ARTWORK")
		coin:SetTexture(file)
		coin:SetSize(COIN_ICON, COIN_ICON)
		coin:SetPoint("LEFT", box, "RIGHT", 2, 0)
		money.boxes[n] = box
	end
	return money
end

local function Money(page, line, left, width, y)
	local money = Next("money", BuildMoney, page)
	money.value = line.money
	money:ClearAllPoints()
	money:SetPoint("TOPLEFT", page, "TOPLEFT", left, -y)
	money:SetSize(width, BOX_HEIGHT)
	for n, coins in ipairs(line.money.coins) do
		Fill(money.boxes[n], coins > 0 and tostring(coins) or "")
	end
	money:Show()
	return BOX_HEIGHT
end

-- Item slots ---------------------------------------------------------------------------------

local function ShowTooltip(slot)
	if not slot.item then
		return
	end
	GameTooltip:SetOwner(slot, "ANCHOR_RIGHT")
	GameTooltip:SetItemByID(slot.item.id)
	GameTooltip:Show()
end

local function BuildSlot(row, n)
	local slot = CreateFrame("Button", nil, row)
	slot:SetSize(SLOT, SLOT)
	slot:SetPoint("LEFT", row, "LEFT", (n - 1) * (SLOT + SLOT_GAP), 0)
	slot:RegisterForClicks("LeftButtonUp", "RightButtonUp")
	slot.icon = slot:CreateTexture(nil, "ARTWORK")
	slot.icon:SetAllPoints(slot)
	slot:SetHighlightTexture("Interface\\Buttons\\ButtonHilight-Square", "ADD")
	slot.count = slot:CreateFontString(nil, "OVERLAY", "NumberFontNormal")
	slot.count:SetPoint("BOTTOMRIGHT", slot, "BOTTOMRIGHT", -2, 2)
	-- A right click takes an item out, as in the trade window.
	slot:SetScript("OnClick", function(self, button)
		if button == "RightButton" and self.item then
			row.value.remove(self.index)
		else
			row.value.drop()
		end
	end)
	slot:SetScript("OnReceiveDrag", function()
		row.value.drop()
	end)
	slot:SetScript("OnEnter", ShowTooltip)
	slot:SetScript("OnLeave", function()
		GameTooltip:Hide()
	end)
	return slot
end

local function BuildSlots(page)
	local row = CreateFrame("Frame", nil, page)
	row.slots = {}
	return row
end

local function DrawSlot(slot, index, item)
	slot.index, slot.item = index, item
	slot.icon:SetTexture(item and C_Item.GetItemIconByID(item.id) or EMPTY_SLOT)
	slot.count:SetText(item and item.count > 1 and tostring(item.count) or "")
	slot:Show()
end

local function Slots(page, line, left, width, y)
	local row = Next("slots", BuildSlots, page)
	row.value = line.slots
	row:ClearAllPoints()
	row:SetPoint("TOPLEFT", page, "TOPLEFT", left, -y)
	row:SetSize(width, SLOT)
	for n = 1, line.slots.size do
		row.slots[n] = row.slots[n] or BuildSlot(row, n)
		DrawSlot(row.slots[n], n, line.slots.items[n])
	end
	row:Show()
	return SLOT
end

-- Drawing ------------------------------------------------------------------------------------

local DRAWERS = { field = Field, money = Money, slots = Slots }

function JournalInputs.Holds(line)
	return DRAWERS[line.style] ~= nil
end

-- A page draws its inputs between Begin and Finish. A box that stays on the page never
-- hides, because a hidden box loses the focus of the player who types in it.
function JournalInputs.Begin()
	for kind in pairs(pools) do
		used[kind] = 0
	end
end

-- Hides the inputs that the page did not draw.
function JournalInputs.Finish()
	for kind, pool in pairs(pools) do
		for n = used[kind] + 1, #pool do
			pool[n]:Hide()
		end
	end
end

-- Draws the input of `line` at `y`, `width` wide, and returns its height.
function JournalInputs.Draw(page, line, left, width, y)
	return DRAWERS[line.style](page, line, left, width, y)
end
