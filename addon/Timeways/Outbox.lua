-- Game events wait here and go out in batches, as many lines as fit in one message
-- (GAMEPLAY.md 5.4). No moment costs a strip of its own.

local _, ns = ...

local Outbox = {}
ns.Outbox = Outbox

-- A few hours of play. Past this, the oldest events go first: the story loses a detail,
-- and the memory of the game stays bounded.
local MAX_WAITING = 500

-- The lines that get a reply. The bridge takes at most one in a batch, as its last line.
local REPLIES = { lore_asked = true, journal_asked = true }

local waiting = {}

function Outbox.Add(input)
	waiting[#waiting + 1] = { line = ns.Json.Encode(input), reply = REPLIES[input.type] == true }
	if #waiting > MAX_WAITING then
		table.remove(waiting, 1)
	end
end

function Outbox.Waiting()
	return #waiting
end

local function Batches(entries)
	local batches, current = {}, nil
	for _, entry in ipairs(entries) do
		if current and (current.ended or not ns.Link.Fits(current.text .. "\n" .. entry.line)) then
			batches[#batches + 1] = current
			current = nil
		end
		if current then
			current.text = current.text .. "\n" .. entry.line
			current.entries[#current.entries + 1] = entry
		else
			current = { text = entry.line, entries = { entry } }
		end
		current.ended = entry.reply
	end
	batches[#batches + 1] = current
	return batches
end

-- Lines that the link does not take stay, in order, for the next flush.
function Outbox.Flush()
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
