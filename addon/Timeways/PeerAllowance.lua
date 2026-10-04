-- The parts that each sender may send to you, for player quests and for MSP (GAMEPLAY.md
-- 3.7.1, 4.7). A sender that floods loses its own parts, and nobody else's.

local _, ns = ...

local PeerAllowance = {}
ns.PeerAllowance = PeerAllowance

-- A sender gets this many parts in a burst, and one more each few seconds.
local BURST, SECONDS = 24, 2
local MAX_PEERS = 64

function PeerAllowance.New()
	return { peers = {}, count = 0 }
end

-- True while the sender has parts left. Takes one part.
function PeerAllowance.Take(allowance, sender, now)
	local peer = allowance.peers[sender]
	if not peer then
		if allowance.count >= MAX_PEERS then
			allowance.peers, allowance.count = {}, 0
		end
		peer = { parts = BURST, at = now }
		allowance.peers[sender], allowance.count = peer, allowance.count + 1
	end
	peer.parts = math.min(BURST, peer.parts + (now - peer.at) / SECONDS)
	peer.at = now
	if peer.parts < 1 then
		return false
	end
	peer.parts = peer.parts - 1
	return true
end
