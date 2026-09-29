-- Text from the desktop, as it shows in the game.

local _, ns = ...

-- The bridge doubles each `|` of the text of a reply (Gnomish Relay SPEC.md 9.8, S10), so
-- no text from the desktop can start a WoW escape such as a color or a fake link. A second
-- escape here shows `||` to the player.
function ns.Plain(text)
	return tostring(text)
end
