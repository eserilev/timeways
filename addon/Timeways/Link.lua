-- The seam to the desktop: `Fits(text)`, `Send(text)`, and a call of ns.OnReply(text)
-- for each reply. Tests put a fake here.

local _, ns = ...

-- The payload of one strip, less the room of the transport flags.
local MAX_TEXT = 3000

-- TODO: send through the shared Messages.lua when relay SPEC.md 9.7, step 5b, adds it.
ns.Link = {
	Fits = function(text)
		return #text <= MAX_TEXT
	end,
	Send = function()
		return false
	end,
}
