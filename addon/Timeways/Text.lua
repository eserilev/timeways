-- Text from the desktop, made safe to show.

local _, ns = ...

-- A `|` starts a WoW escape, such as a color or a link. Text from the desktop shows as it
-- is, so a passage can never draw a fake link.
function ns.Plain(text)
	return (tostring(text):gsub("|", "||"))
end
