-- The one way out to other players. While dev mode is on, nothing goes: a dev world holds
-- fake data, and no real player gets any of it (GAMEPLAY.md 5.15). Fake players never come
-- here, because their messages stay in the game. A test fails when another file sends.

local _, ns = ...

local ToPlayers = {}
ns.ToPlayers = ToPlayers

-- The result of a send that dev mode stopped. The game has no result for it.
ToPlayers.DEV_MODE = "dev mode"

function ToPlayers.Send(prefix, text, channel, target)
	if not ns.Dev.CanTalkToPlayers() then
		return ToPlayers.DEV_MODE
	end
	return C_ChatInfo.SendAddonMessage(prefix, text, channel, target)
end

-- On the logged channel, which Blizzard support can read.
function ToPlayers.SendLogged(prefix, text, channel, target)
	if not ns.Dev.CanTalkToPlayers() then
		return ToPlayers.DEV_MODE
	end
	return C_ChatInfo.SendAddonMessageLogged(prefix, text, channel, target)
end

-- While dev mode is on, only a fake player is heard.
function ToPlayers.Hears(sender)
	return ns.Dev.CanTalkToPlayers() or ns.DevPeer.Has(sender)
end
