-- Past talks (GAMEPLAY.md 3.5): what you said to each NPC and what it answered, with the
-- time, for each character. The talk window shows them above a new talk. No model reads
-- them: the NPC remembers you from your world on the desktop.
-- Other addons can read saved variables, so an entry holds only your own words and the
-- answer of the NPC, never a name of a player.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local TalkHistory = {}
ns.TalkHistory = TalkHistory

local GLOBAL = "TimewaysTalk"

-- The newest exchanges of one NPC that the window shows.
TalkHistory.MAX_PER_NPC = 20
-- The game reads the saved variables at each login and /reload, so they stay small. An
-- exchange holds about 400 bytes, so 200 take about 80 KB, and less than 700 KB when each
-- one is at its limits. 200 is 10 NPCs with a full history.
TalkHistory.MAX_TOTAL = 200
-- The limits of the bridge: the name of the NPC, the words of `talk_asked`, and the answer
-- of 1600 bytes, in which the bridge doubles each `|`.
local MAX_NPC_BYTES, MAX_WORDS_BYTES, MAX_ANSWER_BYTES = 64, 255, 2 * 1600

-- A text of the game keeps each `|` doubled. A lone `|` starts an escape, such as a
-- color or a fake link, so a text with one comes from a bug or another addon.
local function IsShownSafely(text, limit)
	return type(text) == "string"
		and text ~= ""
		and #text <= limit
		and not text:find("%c")
		and not text:gsub("||", ""):find("|", 1, true)
end

local function IsTime(at)
	return type(at) == "number" and at == at and at >= 0 and at < 2 ^ 53
end

local function IsExchange(exchange)
	return type(exchange) == "table"
		and IsTime(exchange.at)
		and IsShownSafely(exchange.said, MAX_WORDS_BYTES)
		and IsShownSafely(exchange.heard, MAX_ANSWER_BYTES)
end

-- Oldest first, and only the newest of the cap.
local function CleanList(list)
	local kept = {}
	for _, exchange in ipairs(type(list) == "table" and list or {}) do
		if IsExchange(exchange) then
			kept[#kept + 1] = { at = exchange.at, said = exchange.said, heard = exchange.heard }
		end
	end
	table.sort(kept, function(a, b)
		return a.at < b.at
	end)
	while #kept > TalkHistory.MAX_PER_NPC do
		table.remove(kept, 1)
	end
	return kept
end

local function Count(talks)
	local count = 0
	for _, list in pairs(talks) do
		count = count + #list
	end
	return count
end

-- The NPC whose oldest exchange is the oldest of all.
local function OldestNpc(talks)
	local oldest
	for npc, list in pairs(talks) do
		if not oldest or list[1].at < talks[oldest][1].at then
			oldest = npc
		end
	end
	return oldest
end

local function Trim(talks)
	local count = Count(talks)
	while count > TalkHistory.MAX_TOTAL do
		local npc = OldestNpc(talks)
		table.remove(talks[npc], 1)
		if #talks[npc] == 0 then
			talks[npc] = nil
		end
		count = count - 1
	end
end

-- Any addon can write the saved variables, so each entry is checked when the game loads
-- them. A broken one is dropped.
local function Checked(value)
	local talks = {}
	for npc, list in pairs(type(value) == "table" and value or {}) do
		if IsShownSafely(npc, MAX_NPC_BYTES) then
			local kept = CleanList(list)
			talks[npc] = #kept > 0 and kept or nil
		end
	end
	Trim(talks)
	return talks
end

local checked

-- The game loads the saved variables after the files of the addon run, so the table is
-- read only when a player acts, never while the file loads.
local function Talks()
	if not checked or _G[GLOBAL] ~= checked then
		checked = Checked(_G[GLOBAL])
		_G[GLOBAL] = checked
	end
	return checked
end

-- The past exchanges with this NPC, oldest first. Each is { at, said, heard }.
function TalkHistory.Of(npc)
	local list = Talks()[npc or ""] or {}
	local copy = {}
	for n, exchange in ipairs(list) do
		copy[n] = { at = exchange.at, said = exchange.said, heard = exchange.heard }
	end
	return copy
end

-- The NPCs that you talked to, by name.
function TalkHistory.Npcs()
	local names = {}
	for npc in pairs(Talks()) do
		names[#names + 1] = npc
	end
	table.sort(names)
	return names
end

-- An exchange that breaks a limit is not kept.
function TalkHistory.Add(npc, said, heard, at)
	local exchange = { at = at, said = said, heard = heard }
	if not IsShownSafely(npc, MAX_NPC_BYTES) or not IsExchange(exchange) then
		return
	end
	local talks = Talks()
	local list = talks[npc] or {}
	list[#list + 1] = exchange
	while #list > TalkHistory.MAX_PER_NPC do
		table.remove(list, 1)
	end
	talks[npc] = list
	Trim(talks)
end
