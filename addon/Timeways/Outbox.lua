-- Game events wait here and go out in batches, as many lines as fit in one message
-- (GAMEPLAY.md 5.4). Each batch costs a screenshot, so most events wait for the flush
-- timer. A big moment goes soon, because its narrator line belongs to that moment.

local _, ns = ...

local Outbox = {}
ns.Outbox = Outbox

-- A few hours of play. Past this, the oldest events go first: the story loses a detail,
-- and the memory of the game stays bounded.
local MAX_WAITING = 500

-- The lines that get a reply. The bridge takes at most one in a batch, as its last line.
local REPLIES = { lore_asked = true, journal_asked = true, talk_asked = true, draft_asked = true }

-- The burst of a big moment, such as a level up in a new zone, goes in one batch.
local SOON_SECONDS = 5
-- A new zone counts too, but a new subzone waits.
local BIG = {
	level_reached = true,
	died = true,
	npc_defeated = true,
	game_quest_done = true,
	instance_entered = true,
	bg_won = true,
	pvp_rank = true,
	mount_ridden = true,
	item_equipped = true,
	-- The prologue is due a few minutes after the first login, not after an hour.
	past_read = true,
}

local waiting = {}
local soonPlanned = false
local lastZone
-- The batch of game events on its way, until its reply comes. A bridge that is down never
-- answers, so one batch at a time keeps the strips few, and its events come back after the
-- transport gives up.
local eventsOnTheWay = false
-- The character line starts every batch, so the desktop always knows whose world a batch
-- changes, also after the story program restarts.
local character

local function Bound()
	while #waiting > MAX_WAITING do
		table.remove(waiting, 1)
	end
end

local function IsBig(input)
	if input.type ~= "zone_entered" then
		return BIG[input.type] == true
	end
	local newZone = input.zone ~= lastZone
	lastZone = input.zone
	return newZone
end

local function FlushSoon()
	if soonPlanned then
		return
	end
	soonPlanned = true
	C_Timer.After(SOON_SECONDS, function()
		soonPlanned = false
		Outbox.Flush()
	end)
end

-- `failed` runs with the error text when a question gets an error reply instead of its answer.
function Outbox.Add(input, failed)
	local reply = REPLIES[input.type] == true
	-- The bridge takes a line with a reply only in its fixed shape, so only an event line
	-- carries the mark of a fake line.
	if not reply then
		ns.Dev.Mark(input)
	end
	waiting[#waiting + 1] = { line = ns.Json.Encode(input), reply = reply, failed = failed }
	Bound()
	if IsBig(input) then
		FlushSoon()
	end
end

function Outbox.Waiting()
	return #waiting
end

function Outbox.SetCharacter(input)
	character = ns.Json.Encode(input)
end

-- The text of a batch that starts with this line.
local function Opening(line)
	return character and (character .. "\n" .. line) or line
end

-- A batch of this one input, which does not wait in the outbox. Nil before the login
-- names the character.
function Outbox.Alone(input)
	return character and Opening(ns.Json.Encode(input))
end

function Outbox.Fits(input)
	return ns.Link.Fits(Opening(ns.Json.Encode(input)))
end

-- An entry that does not fit even alone can never go, so it is dropped.
local function Batches(entries)
	local batches, current = {}, nil
	for _, entry in ipairs(entries) do
		local joined = current and (current.text .. "\n" .. entry.line)
		if current and (current.ended or not ns.Link.Fits(joined)) then
			batches[#batches + 1] = current
			current = nil
		end
		if current then
			current.text = joined
			current.entries[#current.entries + 1] = entry
			current.ended = entry.reply
		elseif ns.Link.Fits(Opening(entry.line)) then
			current = { text = Opening(entry.line), entries = { entry }, ended = entry.reply }
		end
	end
	batches[#batches + 1] = current
	return batches
end

-- The game events of a batch go back to the front, in order. A question does not: it
-- was for that moment, and its error tells the player.
local function Return(batch)
	for i = #batch.entries, 1, -1 do
		if not batch.entries[i].reply then
			table.insert(waiting, 1, batch.entries[i])
		end
	end
	Bound()
end

local function Answered(batch, status, text)
	if not batch.ended then
		eventsOnTheWay = false
	end
	if status == "done" then
		ns.OnReply(text)
		-- The bridge is up, so the next batch of a backlog goes now, not at the next tick.
		Outbox.Flush()
		return
	end
	Return(batch)
	if not batch.ended then
		return
	end
	ns.Link.ShowError(text)
	local question = batch.entries[#batch.entries]
	if question.failed then
		question.failed(text)
	end
end

-- A question always goes. A batch of game events waits while another one is on its way.
local function Send(batch)
	if not batch.ended and eventsOnTheWay then
		return false
	end
	local id = ns.Link.Send(batch.text)
	if not id then
		return false
	end
	eventsOnTheWay = eventsOnTheWay or not batch.ended
	ns.Link.Claim(id, function(status, text)
		Answered(batch, status, text)
	end)
	return true
end

-- Lines that do not go stay, in order, for the next flush. Before the character is known,
-- every line waits.
function Outbox.Flush()
	if not character then
		return
	end
	local kept = {}
	for _, batch in ipairs(Batches(waiting)) do
		if #kept > 0 or not Send(batch) then
			for _, entry in ipairs(batch.entries) do
				kept[#kept + 1] = entry
			end
		end
	end
	waiting = kept
end
