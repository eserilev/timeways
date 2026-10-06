-- The cursor of the player in an edit box. A box takes it only while it shows, and gives
-- it back when it hides, so a typed key never goes to a box that the player can't see.

local _, ns = ...

local Focus = {}
ns.Focus = Focus

-- The box gives the cursor back when it or a parent hides.
function Focus.ReleaseOnHide(box)
	box:SetScript("OnHide", box.ClearFocus)
end

-- A press anywhere on `area` puts the cursor in `box`. A box of several lines is only as
-- tall as its text, so a press below the text misses it.
function Focus.OnAreaClick(area, box)
	area:EnableMouse(true)
	area:SetScript("OnMouseDown", function()
		box:SetFocus()
	end)
end

-- The cursor blinks at the end of the text, ready to type. Call it after the box and its
-- parents show: the game drops a SetFocus on a hidden box.
function Focus.AtEnd(box)
	box:SetFocus()
	box:SetCursorPosition(#(box:GetText() or ""))
end

-- A moving player steers with the keys, and a box with the cursor takes them.
local function Busy()
	return InCombatLockdown() or GetUnitSpeed("player") > 0
end

-- For a box that opens by itself, not at a click of the player: it takes the cursor only
-- while the player stands still out of combat.
function Focus.AtEndIfIdle(box)
	if not Busy() then
		Focus.AtEnd(box)
	end
end
