-- MSP, the protocol of roleplay addons such as Total RP 3 and MyRolePlay (GAMEPLAY.md
-- 3.7.1). With the switch on and no other roleplay addon, Timeways answers their requests
-- for your profile, and asks other players for their name and title. With the switch off,
-- it sends no MSP message at all, and reads none.

local _, ns = ...

local Msp = {}
ns.Msp = Msp

-- MSP gets half of the budget of Timeways, so a quest part never waits long for it.
local OWN_BURST, OWN_SECONDS = 4, 2
-- The limits of LibMSP: a field is asked again only after 30 seconds, a player who never
-- answered only after 300, and the same request again within 5 seconds gets no answer.
local ASK_SECONDS, PROBE_SECONDS, REPEAT_SECONDS = 30, 300, 5
-- A sender gets this many parts in a burst, and one more each few seconds.
local PEER_BURST, PEER_SECONDS = 24, 2
local MAX_PEERS = 64
local MAX_PLAYERS = 200
local MAX_WAITING = 50
-- The longest name or title that the tooltip shows.
local TOOLTIP_BYTES = 60

local RESULT = Enum.SendAddonMessageResult

-- The fields of other players that Timeways keeps: the eight that it shares too.
local KEPT = { NA = true, NT = true, CU = true, DE = true, AG = true, MO = true, HB = true, HI = true }

local own, ownAt = OWN_BURST, nil
local session = math.random(0, 4095)
local collector = ns.MspParts.NewCollector()
local allowance, allowanceCount = {}, 0
-- The messages that wait: { to, logged, parts }.
local waiting = {}
-- What other players shared, in memory only: { fields, versions, asked, answered, probedAt }.
local players, playerCount = {}, 0

C_ChatInfo.RegisterAddonMessagePrefix(ns.MspWire.PREFIX)

local function Active()
	return ns.MspProfile.IsSharing()
end

local function OwnHas(now)
	ownAt = ownAt or now
	own = math.min(OWN_BURST, own + (now - ownAt) / OWN_SECONDS)
	ownAt = now
	return own >= 1
end

-- The logged channel can answer nil. Nil gives no reason to wait, so it counts as Success.
local function Send(part, entry)
	local send = entry.logged and C_ChatInfo.SendAddonMessageLogged or C_ChatInfo.SendAddonMessage
	return send(ns.MspWire.PREFIX, part, "WHISPER", entry.to) or RESULT.Success
end

-- Sends while both budgets allow. MSP is best effort: a part that the game refuses drops
-- its message.
function Msp.Flush()
	local now = GetTime()
	while waiting[1] and OwnHas(now) and ns.AddonBudget.Has() do
		local entry = waiting[1]
		local result = Send(entry.parts[1], entry)
		table.remove(entry.parts, 1)
		if result == RESULT.Success then
			ns.AddonBudget.Spend()
			own = own - 1
		end
		if result ~= RESULT.Success or #entry.parts == 0 then
			table.remove(waiting, 1)
		end
	end
end

local function Queue(to, commands, logged)
	if #commands == 0 then
		return
	end
	local text = ns.MspWire.Join(commands)
	session = ns.MspParts.NextSession(session)
	local parts = ns.MspParts.Split(logged and ns.MspWire.Escape(text) or text, session)
	if not parts then
		return
	end
	waiting[#waiting + 1] = { to = to, logged = logged, parts = parts }
	while #waiting > MAX_WAITING do
		table.remove(waiting, 1)
	end
	Msp.Flush()
end

function Msp.Waiting()
	return #waiting
end

local function ForgetOldestPlayer()
	local oldest
	for name, player in pairs(players) do
		if not oldest or player.at < players[oldest].at then
			oldest = name
		end
	end
	players[oldest] = nil
	playerCount = playerCount - 1
end

-- A full list forgets only its oldest player, so the timers of the others hold.
local function Player(name, now)
	if not players[name] then
		if playerCount >= MAX_PLAYERS then
			ForgetOldestPlayer()
		end
		players[name] = { fields = {}, versions = {}, asked = {}, answered = false, repeats = {} }
		playerCount = playerCount + 1
	end
	players[name].at = now
	return players[name]
end

-- The fields of the game that LibMSP sends too, so a roleplay addon shows the right race
-- and class.
local function GameFields(fields)
	fields.VP = ns.MspWire.PROTOCOL
	fields.VA = "Timeways/" .. ns.App.version
	fields.GU = UnitGUID("player")
	fields.GC = select(2, UnitClass("player"))
	fields.GR = select(2, UnitRace("player"))
	fields.GS = tostring(UnitSex("player"))
	fields.GF = UnitFactionGroup("player")
	return fields
end

-- The reply to one request: logged when it carries a text, plain when it says "not changed".
local function Answer(request, fields, safe, plain)
	if request.field == "TT" then
		local block, version = ns.MspWire.Tooltip(fields)
		if request.version == version then
			plain[#plain + 1] = ns.MspWire.Same("TT", version)
		else
			safe[#safe + 1] = block
		end
		return
	end
	local version = ns.MspWire.Version(fields[request.field])
	if request.version == version then
		plain[#plain + 1] = ns.MspWire.Same(request.field, version)
	else
		safe[#safe + 1] = ns.MspWire.Field(request.field, fields[request.field])
	end
end

-- A request for the same field within a few seconds gets no answer, and waits longer.
local function Repeated(player, field, now)
	local quietUntil = player.repeats[field]
	player.repeats[field] = now + REPEAT_SECONDS
	return quietUntil ~= nil and now < quietUntil
end

local function AnswerAll(sender, requests, now)
	local player = Player(sender, now)
	local fields = GameFields(ns.MspProfile.Fields())
	local safe, plain = {}, {}
	for _, request in ipairs(requests) do
		if ns.MspWire.InTooltip(request.field) then
			request.field = "TT"
		end
		if not Repeated(player, request.field, now) then
			Answer(request, fields, safe, plain)
		end
	end
	Queue(sender, safe, true)
	Queue(sender, plain, false)
end

-- Only a logged message carries a text, as in LibMSP: support can read it when a player
-- reports it.
local function Keep(player, command, logged)
	if command.kind ~= "field" or not logged then
		return
	end
	if command.field == "TT" then
		player.versions.TT = command.version
	elseif KEPT[command.field] then
		player.fields[command.field] = command.text
		player.versions[command.field] = command.version
	end
end

local function Allowed(sender, now)
	local peer = allowance[sender]
	if not peer then
		if allowanceCount >= MAX_PEERS then
			allowance, allowanceCount = {}, 0
		end
		peer = { parts = PEER_BURST, at = now }
		allowance[sender], allowanceCount = peer, allowanceCount + 1
	end
	peer.parts = math.min(PEER_BURST, peer.parts + (now - peer.at) / PEER_SECONDS)
	peer.at = now
	if peer.parts < 1 then
		return false
	end
	peer.parts = peer.parts - 1
	return true
end

local function Handle(sender, message, logged, now)
	local player = Player(sender, now)
	local requests = {}
	for _, command in ipairs(ns.MspWire.Commands(message)) do
		player.answered = true
		if command.kind == "request" then
			requests[#requests + 1] = command
		end
		Keep(player, command, logged)
	end
	AnswerAll(sender, requests, now)
end

-- The payload of CHAT_MSG_ADDON and CHAT_MSG_ADDON_LOGGED.
function Msp.Received(prefix, text, sender, logged)
	if prefix ~= ns.MspWire.PREFIX or type(text) ~= "string" or issecretvalue(text) or not Active() then
		return
	end
	sender = ns.TaskPeople.Full(sender)
	local now = GetTime()
	if not sender or sender == ns.TaskPeople.Me() or not Allowed(sender, now) then
		return
	end
	local whole, allLogged = ns.MspParts.Add(collector, sender, text, logged, now)
	if whole then
		Handle(sender, whole, allLogged, now)
	end
end

-- Asks a player for these fields of MSP, under the limits of LibMSP.
function Msp.Ask(name, fields)
	name = ns.TaskPeople.Full(name)
	if not Active() or not name or name == ns.TaskPeople.Me() then
		return
	end
	local now = GetTime()
	local player = Player(name, now)
	if not player.answered then
		if player.probedAt and now < player.probedAt + PROBE_SECONDS then
			return
		end
		player.probedAt = now
	end
	local requests = {}
	for _, field in ipairs(fields) do
		field = ns.MspWire.InTooltip(field) and "TT" or field
		if now >= (player.asked[field] or -math.huge) + ASK_SECONDS then
			player.asked[field] = now
			requests[#requests + 1] = ns.MspWire.Request(field, player.versions[field])
		end
	end
	Queue(name, requests, false)
end

-- The fields that a player shared, in memory only.
function Msp.FieldsOf(name)
	local player = players[ns.TaskPeople.Full(name) or ""]
	return player and player.fields or {}
end

-- Text from another player: no escape of the game, on one line, cut between letters.
local function Shown(text)
	text = text:gsub("|c%x%x%x%x%x%x%x%x", ""):gsub("|r", ""):gsub("%c+", " ")
	text = ns.Utf8.Cut(text, TOOLTIP_BYTES, TOOLTIP_BYTES)
	return (text:gsub("|", "||"))
end

-- "Ada Brightwater, Keeper of the Flame", or nil with no name and no title.
function Msp.TooltipLine(name)
	local fields = Msp.FieldsOf(name)
	local parts = {}
	for _, code in ipairs({ "NA", "NT" }) do
		if fields[code] then
			parts[#parts + 1] = Shown(fields[code])
		end
	end
	return #parts > 0 and table.concat(parts, ", ") or nil
end

-- A whisper to an offline player, to a realm that is not connected, or to the other faction
-- fails with an error in the chat.
local function CanWhisper(unit)
	return UnitIsConnected(unit) and UnitIsSameServer(unit) and UnitFactionGroup(unit) == UnitFactionGroup("player")
end

-- The tooltip of another player shows the name and the title, and asks for them again.
function Msp.OnTooltip(tooltip)
	if tooltip ~= GameTooltip or not Active() then
		return
	end
	local _, unit = tooltip:GetUnit()
	if not unit or issecretvalue(unit) then
		return
	end
	local name = ns.TaskPeople.OfUnit(unit)
	if not name or name == ns.TaskPeople.Me() then
		return
	end
	if CanWhisper(unit) then
		Msp.Ask(name, { "TT" })
	end
	local line = Msp.TooltipLine(name)
	if line then
		tooltip:AddLine(line, 1, 0.82, 0)
	end
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("CHAT_MSG_ADDON")
frame:RegisterEvent("CHAT_MSG_ADDON_LOGGED")
frame:SetScript("OnEvent", function(_, event, prefix, text, _, sender)
	Msp.Received(prefix, text, sender, event == "CHAT_MSG_ADDON_LOGGED")
end)

TooltipDataProcessor.AddTooltipPostCall(Enum.TooltipDataType.Unit, Msp.OnTooltip)

C_Timer.NewTicker(1, Msp.Flush)
