-- The text of quests, gossip, and books, as the player reads it (GAMEPLAY.md 5.10).
-- `/lore` searches it. The name of your character becomes `$N`, so no model sees it (5.11).

local _, ns = ...

local Seen = {}
ns.Seen = Seen

-- The story program takes 2400 bytes. The rest is room for the JSON escapes.
local MAX_BYTES = 2000
local MAX_NAME_BYTES = 96

-- The story program keeps a text once, so one send for each UI session is enough.
local sent = {}

local function Readable(value)
	return type(value) == "string" and not issecretvalue(value) and value ~= ""
end

-- The bridge drops a line with a control character, and game text holds line breaks. The
-- search needs no line breaks, and the Learned page shows one line anyway.
local function OneLine(text)
	return (text:gsub("%c+", " "))
end

local function Name(value)
	if Readable(value) and #value <= MAX_NAME_BYTES then
		return OneLine(value)
	end
end

-- Small letters for ASCII and for the capitals of Latin-1, such as É and Ö: the letters of
-- names on US and EU realms. Each capital of Latin-1 is 0xC3 0x80 to 0xC3 0x9E, and its small
-- letter adds 0x20 to the second byte, so the positions in the text stay the same. 0xC3 0x97
-- is the sign ×.
local function Fold(text)
	local folded = text:lower():gsub("\195([\128-\158])", function(byte)
		if byte == "\151" then
			return nil
		end
		return "\195" .. string.char(byte:byte() + 32)
	end)
	return folded
end

-- A letter of ASCII. A character past ASCII can be a quote mark, as in "«Ada»", and Lua 5.1
-- cannot tell, so it counts as no letter: a name next to it becomes a mark. Replacing too
-- much costs a word, and replacing too little sends the name.
local function IsLetter(byte)
	return byte ~= nil and byte < 128 and string.char(byte):match("%a") ~= nil
end

-- The name counts only as a whole word: "Ed" is not the end of "killed".
local function WithoutName(text)
	local name = UnitName("player")
	if not Readable(name) then
		return text
	end
	local lower, needle = Fold(text), Fold(name)
	local parts, start, from = {}, 1, 1
	while true do
		local to
		from, to = lower:find(needle, from, true)
		if not from then
			break
		end
		-- Right after a mark, the name starts a word: the letter before it is gone.
		local before = lower:byte(from - 1)
		if from == start and start > 1 then
			before = nil
		end
		if IsLetter(before) or IsLetter(lower:byte(to + 1)) then
			from = from + 1
		else
			parts[#parts + 1] = text:sub(start, from - 1) .. "$N"
			start, from = to + 1, to + 1
		end
	end
	parts[#parts + 1] = text:sub(start)
	return table.concat(parts)
end

-- A byte from 0x80 to 0xBF continues a UTF-8 character, so the cut never lands on one.
local function Cut(text)
	local stop = math.min(#text, MAX_BYTES)
	while stop < #text and stop > 0 and text:byte(stop + 1) >= 0x80 and text:byte(stop + 1) < 0xC0 do
		stop = stop - 1
	end
	return text:sub(1, stop)
end

local function Add(kind, title, npc, text)
	if not Readable(text) then
		return
	end
	text = Cut(WithoutName(OneLine(text)))
	if strtrim(text) == "" then
		return
	end
	local input = ns.Inputs.Seen(time(), kind, Name(title), Name(npc), Name(GetRealZoneText()), text)
	local key = kind .. "\n" .. (input.title or "") .. "\n" .. (input.npc or "") .. "\n" .. text
	if sent[key] or not ns.Outbox.Fits(input) then
		return
	end
	sent[key] = true
	ns.Outbox.Add(input)
end

local function Speaker()
	return ns.Units.NpcName("npc")
end

function Seen.Gossip()
	Add("gossip", nil, Speaker(), C_GossipInfo.GetText())
end

function Seen.QuestGreeting()
	Add("gossip", nil, Speaker(), GetGreetingText())
end

function Seen.QuestDetail()
	local text, objectives = GetQuestText(), GetObjectiveText()
	if Readable(text) and Readable(objectives) then
		text = text .. " " .. objectives
	end
	Add("quest", GetTitleText(), Speaker(), text)
end

function Seen.QuestProgress()
	Add("quest", GetTitleText(), Speaker(), GetProgressText())
end

function Seen.QuestComplete()
	Add("quest", GetTitleText(), Speaker(), GetRewardText())
end

-- A letter that a player wrote has a creator. Its words are theirs, so they stay out.
function Seen.Book()
	if ItemTextGetCreator() then
		return
	end
	Add("book", ItemTextGetItem(), nil, ItemTextGetText())
end
