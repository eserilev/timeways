-- How much each NPC that you dealt with trusts you (GAMEPLAY.md 3.5). It shows on the
-- tooltip of the NPC, and a change of feeling shows in the chat.

local _, ns = ...

local Trust = {}
ns.Trust = Trust

local PREFIX = "|cffc8a064Timeways|r: "

-- Enum.TooltipDataType.Unit of the client.
-- TODO: use the Enum name once the API gate of Gnomish Relay knows the Enum table.
local UNIT_TOOLTIP = 2

-- From the most trust to the least: the tooltip word, and the chat line of a change.
local BANDS = {
	{ min = 50, word = "Trusts you", change = "now trusts you." },
	{ min = 10, word = "Likes you", change = "now likes you." },
	{ min = -9, word = "Neutral", change = "feels neutral about you now." },
	{ min = -49, word = "Wary of you", change = "is now wary of you." },
	{ min = -100, word = "Distrusts you", change = "no longer trusts you." },
}

-- The people of the newest journal, by name. Nil until the first journal came, so the
-- first one prints no changes.
local people

local function Band(trust)
	for _, band in ipairs(BANDS) do
		if trust >= band.min then
			return band
		end
	end
	return BANDS[#BANDS]
end

-- A trust that nothing changed yet counts as neutral.
local function TrustOf(person)
	return person and type(person.trust) == "number" and person.trust or 0
end

-- "Likes you. Slapped 2 times." Empty for someone that you only met.
function Trust.Line(person)
	local parts = {}
	if type(person.trust) == "number" then
		parts[#parts + 1] = Band(person.trust).word .. "."
	end
	if type(person.slapped) == "number" then
		local times = person.slapped == 1 and "time" or "times"
		parts[#parts + 1] = string.format("Slapped %d %s.", person.slapped, times)
	end
	return table.concat(parts, " ")
end

local function Changed(before, now)
	local band = Band(TrustOf(now))
	if before ~= nil and band ~= Band(TrustOf(before)) then
		DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. ns.Plain(now.name) .. " " .. band.change)
	end
end

function Trust.Update(list)
	local newest = {}
	for _, person in ipairs(type(list) == "table" and list or {}) do
		if type(person) == "table" and type(person.name) == "string" then
			newest[person.name] = person
			if people then
				Changed(people[person.name] or {}, person)
			end
		end
	end
	people = newest
end

-- Only the game's own tooltip gets the line, and only for an NPC that you dealt with.
function Trust.OnTooltip(tooltip)
	if tooltip ~= GameTooltip or not people then
		return
	end
	local _, unit = tooltip:GetUnit()
	local name = unit and ns.Units.NpcName(unit)
	local line = name and people[name] and Trust.Line(people[name]) or ""
	if line ~= "" then
		tooltip:AddLine("Timeways: " .. line, 0.78, 0.63, 0.39)
	end
end

TooltipDataProcessor.AddTooltipPostCall(UNIT_TOOLTIP, Trust.OnTooltip)
