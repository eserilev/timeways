-- Game events wait here and go out in batches, as many lines as fit in one message
-- (GAMEPLAY.md 5.4). No moment costs a strip of its own.

local _, ns = ...

local Outbox = {}
ns.Outbox = Outbox

-- A few hours of play. Past this, the oldest events go first: the story loses a detail,
-- and the memory of the game stays bounded.
local MAX_WAITING = 500

-- The lines that get a reply. The bridge takes at most one in a batch, as its last line.
local REPLIES = { lore_asked = true, journal_asked = true, talk_asked = true }

local waiting = {}
-- The character line starts every batch, so the desktop always knows whose world a batch
-- changes, also after the story program restarts.
local character

function Outbox.Add(input)
	waiting[#waiting + 1] = { line = ns.Json.Encode(input), reply = REPLIES[input.type] == true }
	if #waiting > MAX_WAITING then
		table.remove(waiting, 1)
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

-- Lines that the link does not take stay, in order, for the next flush. Before the
-- character is known, every line waits.
function Outbox.Flush()
	if not character then
		return
	end
	local kept = {}
	for _, batch in ipairs(Batches(waiting)) do
		if #kept > 0 or not ns.Link.Send(batch.text) then
			for _, entry in ipairs(batch.entries) do
				kept[#kept + 1] = entry
			end
		end
	end
	waiting = kept
end
