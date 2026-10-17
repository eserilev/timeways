-- The names of players. On WoW Forever a character has a first name and, often, a surname,
-- and no realm. The unit functions give the surname where other clients give the realm, so
-- the key of a character is "First-Surname", and the game shows "First Surname".

local _, ns = ...

local PlayerNames = {}
ns.PlayerNames = PlayerNames

local function Readable(value)
	return type(value) == "string" and value ~= "" and not issecretvalue(value)
end

function PlayerNames.Regional()
	local ok, on = pcall(RegionalUniqueNamesEnabled)
	return ok and on == true
end

-- The first name and the surname of a key, or the key and nil. "Name-Realm" of your own
-- realm has no surname: a character with no surname gets your realm in its key.
function PlayerNames.Parts(key)
	local first, rest = key:match("^([^-]+)%-(.+)$")
	if first and PlayerNames.Regional() and rest ~= GetNormalizedRealmName() then
		return first, rest
	end
	return key, nil
end

-- The name as the game shows it: "First Surname", or the name with no realm of your own.
function PlayerNames.Shown(key)
	local first, surname = PlayerNames.Parts(key)
	if surname then
		return first .. " " .. surname
	end
	return Ambiguate(key, "none")
end

-- Your first name and your surname, or nil for a character with none.
function PlayerNames.MyParts()
	local first, surname = UnitName("player")
	if not Readable(first) then
		return nil
	end
	if PlayerNames.Regional() and Readable(surname) then
		return first, surname
	end
	return first, nil
end

function PlayerNames.Mine()
	local first, surname = PlayerNames.MyParts()
	if surname then
		return first .. " " .. surname
	end
	return first
end
