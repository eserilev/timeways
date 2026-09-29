-- The seam to the desktop: `ns.Link.Fits(text)` and `ns.Link.Send(text)` out, and each
-- final reply in. The shared `Messages.lua` of Gnomish Relay carries both (relay SPEC.md
-- 9.8 and 13.2). Timeways has one chat.

local _, ns = ...

local CHAT = { id = "story" }

-- The replies that go to a handler of their own, not to the story code: the self-test.
local claimed = {}

ns.Link = {
	Fits = function(text)
		return ns.Messages.Fits(CHAT, text)
	end,
	-- The id of the message, or nil when the link did not take it.
	Send = function(text)
		local message = ns.Messages.Send(CHAT, text)
		return message and message.id
	end,
	Claim = function(id, handler)
		claimed[id] = handler
	end,
}

-- A done reply holds JSON lines. An error reply is plain text from the bridge, such as
-- "Timeways story program not running.", and the player sees it as it is.
function ns.Link.Receive(id, status, text)
	local handler = claimed[id]
	if handler then
		claimed[id] = nil
		handler(status, text)
	elseif status == "done" then
		ns.OnReply(text)
	elseif type(text) == "string" and text ~= "" then
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
	end
end

ns.Messages.OnReply = function(_, id, status, text)
	ns.Link.Receive(id, status, text)
end

ns.Messages.Init()
C_Timer.NewTicker(1, ns.Messages.Tick)
