-- The seam to the desktop: `ns.Link.Fits(text)` and `ns.Link.Send(text)` out, and each
-- final reply in. The shared `Messages.lua` of Gnomish Relay carries both (relay SPEC.md
-- 9.8 and 13.2). Timeways has one chat.

local _, ns = ...

local CHAT = { id = "story" }

ns.Link = {
	Fits = function(text)
		return ns.Messages.Fits(CHAT, text)
	end,
	Send = function(text)
		return ns.Messages.Send(CHAT, text) ~= nil
	end,
}

-- A done reply holds JSON lines. An error reply is plain text from the bridge, such as
-- "Timeways story program not running.", and the player sees it as it is.
ns.Messages.OnReply = function(_, _, status, text)
	if status == "done" then
		ns.OnReply(text)
	elseif type(text) == "string" and text ~= "" then
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
	end
end

ns.Messages.Init()
C_Timer.NewTicker(1, ns.Messages.Tick)
