-- The on and off words of a setting of `/timeways`, such as "ratings on" or "ai off".

local _, ns = ...

local Switch = {}
ns.Switch = Switch

-- "on" or "off", "other" for the name of the setting with no word or a wrong one, or nil
-- for a message about something else. Case and extra spaces do not count.
function Switch.Read(message, name)
	local words = message:lower():match("^%s*(.-)%s*$"):gsub("%s+", " ")
	if words == name .. " on" then
		return "on"
	end
	if words == name .. " off" then
		return "off"
	end
	if words == name or words:sub(1, #name + 1) == name .. " " then
		return "other"
	end
	return nil
end
