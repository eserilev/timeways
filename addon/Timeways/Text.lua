-- Text from the desktop, as it shows in the game.

local _, ns = ...

-- The bridge doubles each `|` of the text of a reply (Gnomish Relay SPEC.md 9.8, S10), so
-- no text from the desktop can start a WoW escape such as a color or a fake link. A second
-- escape here shows `||` to the player.
function ns.Plain(text)
	return tostring(text)
end

-- The desktop writes `$N` in place of the name of your character, so no model ever sees
-- the name (GAMEPLAY.md 5.11). Only your own screen shows it.
function ns.WithName(text)
	local name = UnitName("player")
	local shown = type(name) == "string" and not issecretvalue(name) and name:gsub("%%", "%%%%") or "you"
	return (ns.Plain(text):gsub("%$N", shown))
end
