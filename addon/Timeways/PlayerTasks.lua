-- Tasks that one player gives another (GAMEPLAY.md 4.7). The giver writes a task and sends
-- it. The doer accepts it, does the steps, and turns it in face to face. The giver decides.
-- A task never goes to the desktop: it holds the names of real players (5.11).

local _, ns = ...

local PlayerTasks = {}
ns.PlayerTasks = PlayerTasks

-- A peer can have this many offers waiting for you. You hold this many open tasks that
-- you got, and as many that you gave.
local OFFERS_PER_GIVER = 3
local MAX_OPEN = 20

-- The players who answered a call: { [name] = true }.
local MAX_ANSWERED = 64
local answered, answeredCount = {}, 0

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Short(name)
	return ns.TaskPeople.Short(name)
end

local function Changed()
	ns.TaskTracker.TasksChanged()
	ns.JournalFrame.Refresh()
end

-- Where you stand: the subzone, such as Brill, or the zone outside a subzone.
local function WhereYouStand()
	local subzone = GetSubZoneText()
	return subzone ~= "" and subzone or GetRealZoneText()
end

local function Close(task, status)
	task.status = status
	task.closedAt = time()
	task.turnInAt = nil
end

local function CountOffersFrom(tasks, giver)
	local count = 0
	for _, task in pairs(tasks) do
		if task.giver == giver and task.status == "offered" then
			count = count + 1
		end
	end
	return count
end

-- The task that you gave, when the sender is its doer.
local function GivenTo(sender, id)
	local task = ns.TaskStore.Data().given[id]
	if task and task.doer == sender then
		return task
	end
end

-- The task that you got, when the sender is its giver.
local function ReceivedFrom(sender, id)
	return ns.TaskStore.Data().received[ns.TaskStore.ReceivedKey(sender, id)]
end

local function Copy(steps)
	local copy = {}
	for n, step in ipairs(steps) do
		copy[n] = { kind = step.kind, target = step.target, count = step.count }
	end
	return copy
end

-- Giving -----------------------------------------------------------------------------------

-- Asks the party, the guild, and friends online who has Timeways. Each one who has it
-- answers, and shows in the list of who can get a task.
function PlayerTasks.Call()
	ns.TaskChannel.Broadcast({ type = "hello" })
	for _, friend in ipairs(ns.TaskPeople.OnlineFriends()) do
		ns.TaskChannel.Whisper(friend, { type = "hello" })
	end
end

-- The players who can get a task now, sorted by name: { name, relation }.
function PlayerTasks.Recipients()
	local list = {}
	for name in pairs(answered) do
		local relation = ns.TaskPeople.Relation(name)
		if relation and ns.TaskPeople.IsOnline(name) then
			list[#list + 1] = { name = name, relation = relation }
		end
	end
	table.sort(list, function(a, b)
		return a.name < b.name
	end)
	return list
end

-- `draft` is { title, text, reward, steps }, checked by the form. Returns the id, or nil.
function PlayerTasks.Give(draft, doer)
	local data = ns.TaskStore.Data()
	if data.refusedBy[doer] then
		Say(Short(doer) .. " doesn't take tasks from you.")
		return nil
	end
	if ns.TaskStore.CountOpen(data.given) >= MAX_OPEN then
		Say(string.format("You have %d open tasks. Cancel one to give another.", MAX_OPEN))
		return nil
	end
	local id = ns.TaskStore.NewId(time())
	local task = {
		id = id,
		giver = ns.TaskPeople.Me(),
		doer = doer,
		title = draft.title,
		text = draft.text,
		reward = draft.reward or "",
		steps = Copy(draft.steps),
		status = "offered",
		sentAt = time(),
		claims = {},
	}
	data.given[id] = task
	ns.TaskStore.Trim(data.given)
	Changed()
	ns.TaskTracker.Watch()
	ns.TaskChannel.Whisper(doer, {
		type = "offer",
		id = id,
		title = task.title,
		text = task.text,
		reward = task.reward,
		steps = task.steps,
	})
	Say("You sent " .. task.title .. " to " .. Short(doer) .. ".")
	Changed()
	return id
end

function PlayerTasks.Cancel(id)
	local task = ns.TaskStore.Data().given[id]
	if not task or not ns.TaskStore.IsOpen(task) then
		return
	end
	Close(task, "cancelled")
	ns.TaskChannel.Whisper(task.doer, { type = "cancel", id = id })
	Changed()
end

-- The giver decides, face to face.
function PlayerTasks.Complete(id)
	local task = ns.TaskStore.Data().given[id]
	if not task or task.status ~= "accepted" or not ns.TaskPeople.IsNear(task.doer) then
		return false
	end
	Close(task, "done")
	task.place = WhereYouStand()
	ns.TaskChannel.Whisper(task.doer, { type = "result", id = id, verdict = "done" })
	Say(Short(task.doer) .. " finished " .. task.title .. ".")
	Changed()
	return true
end

function PlayerTasks.NotYet(id)
	local task = ns.TaskStore.Data().given[id]
	if not task or task.status ~= "accepted" then
		return
	end
	task.turnInAt = nil
	ns.TaskChannel.Whisper(task.doer, { type = "result", id = id, verdict = "notyet" })
	Changed()
end

-- Doing ------------------------------------------------------------------------------------

local function Answer(key, status, type)
	local task = ns.TaskStore.Data().received[key]
	if not task or task.status ~= "offered" then
		return nil
	end
	task.status = status
	task.answeredAt = time()
	ns.TaskChannel.Whisper(task.giver, { type = type, id = task.id })
	Changed()
	return task
end

function PlayerTasks.Accept(key)
	local task = Answer(key, "accepted", "accept")
	if task then
		task.progress = {}
		Say("Task accepted: " .. task.title .. ".")
		ns.TaskTracker.Accepted()
	end
end

function PlayerTasks.Decline(key)
	local task = Answer(key, "declined", "decline")
	if task then
		task.closedAt = time()
	end
end

-- No more tasks from this player. The giver's addon learns it, so it stops asking.
function PlayerTasks.Block(key)
	local data = ns.TaskStore.Data()
	local task = data.received[key]
	if not task then
		return
	end
	ns.TaskStore.AddName(data.blocked, task.giver)
	if ns.TaskStore.IsOpen(task) then
		Close(task, "declined")
	end
	ns.TaskChannel.Whisper(task.giver, { type = "block", id = task.id })
	Say("You won't get tasks from " .. Short(task.giver) .. " anymore.")
	Changed()
end

function PlayerTasks.Unblock(name)
	local blocked = ns.TaskStore.Data().blocked
	if not blocked[name] then
		return
	end
	blocked[name] = nil
	Say(Short(name) .. " can send you tasks again.")
	Changed()
end

-- The players that you blocked, sorted by name.
function PlayerTasks.Blocked()
	local names = {}
	for name in pairs(ns.TaskStore.Data().blocked) do
		names[#names + 1] = name
	end
	table.sort(names)
	return names
end

function PlayerTasks.GiveUp(key)
	local task = ns.TaskStore.Data().received[key]
	if not task or task.status ~= "accepted" then
		return
	end
	Close(task, "declined")
	ns.TaskChannel.Whisper(task.giver, { type = "decline", id = task.id })
	Changed()
end

local function ClaimList(task)
	local claims = {}
	for index = 1, #task.steps do
		local claim = task.claims[index]
		if claim then
			claims[#claims + 1] = { index = index, at = claim.at, zone = claim.zone }
		end
	end
	return claims
end

-- The turn-in carries every claim again, so a step message that got lost costs nothing.
function PlayerTasks.AskTurnIn(key)
	local task = ns.TaskStore.Data().received[key]
	if not task or task.status ~= "accepted" then
		return
	end
	task.turnInAt = time()
	ns.TaskChannel.Whisper(task.giver, { type = "turnin", id = task.id, claims = ClaimList(task) })
	Say("You asked " .. Short(task.giver) .. " to check your task. Stand next to them.")
	Changed()
end

-- A step of a task that you got is done. Your addon saw it, and tells the giver.
function PlayerTasks.Claim(task, index)
	if task.claims[index] then
		return
	end
	local claim = { at = time(), zone = GetRealZoneText() }
	task.claims[index] = claim
	ns.TaskChannel.Whisper(task.giver, { type = "step", id = task.id, index = index, at = claim.at, zone = claim.zone })
	Say(string.format("%s: step %d of %d done.", task.title, index, #task.steps))
	Changed()
end

-- Receiving --------------------------------------------------------------------------------

local function Offered(sender, message)
	local data = ns.TaskStore.Data()
	local key = ns.TaskStore.ReceivedKey(sender, message.id)
	local full = ns.TaskStore.CountOpen(data.received) >= MAX_OPEN
		or CountOffersFrom(data.received, sender) >= OFFERS_PER_GIVER
	if data.blocked[sender] or data.received[key] or full then
		return
	end
	data.received[key] = {
		id = message.id,
		giver = sender,
		doer = ns.TaskPeople.Me(),
		title = message.title,
		text = message.text,
		reward = message.reward,
		steps = message.steps,
		status = "offered",
		sentAt = time(),
		claims = {},
	}
	ns.TaskStore.Trim(data.received)
	Say(Short(sender) .. " sent you a task: " .. message.title .. ". Open your journal to read it.")
	Changed()
end

-- A step or a turn-in means that the doer took the task, also when the accept got lost.
local function Took(task)
	if task.status == "offered" then
		task.status = "accepted"
		task.answeredAt = time()
		-- A party that began before the accept counts from now.
		ns.TaskTracker.Watch()
	end
end

-- The giver's addon judges a claim as it comes, while its own records still reach back.
local function Settle(task, index)
	local claim = task.claims[index]
	claim.level = ns.TaskProof.Level(task.steps[index], claim, task, ns.TaskStore.Data())
end

-- The time and the zone of a step message are the doer's word. A step that comes now
-- happened now, so the giver's addon takes its own clock, and its own range check.
local function Stepped(task, message)
	local index = message.index
	if not task.steps[index] or task.claims[index] then
		return
	end
	-- Only the giver's addon can see that the doer stands next to the giver.
	if ns.TaskPeople.IsNear(task.doer) then
		ns.TaskStore.Record("near", { at = time(), name = task.doer })
	end
	local close = ns.TaskPeople.IsClose(task.doer)
	local zone = close and GetRealZoneText() or message.zone
	task.claims[index] = { at = time(), zone = zone, near = close }
	Settle(task, index)
end

-- A claim that came live stays: the turn-in only fills the claims that got lost.
local function TurnInAsked(task, message)
	for _, claim in ipairs(message.claims) do
		if task.steps[claim.index] and not task.claims[claim.index] then
			task.claims[claim.index] = { at = claim.at, zone = claim.zone }
			Settle(task, claim.index)
		end
	end
	if not task.turnInAt then
		Say(Short(task.doer) .. " wants to turn in " .. task.title .. ". Meet face to face, then open your journal.")
	end
	task.turnInAt = time()
end

-- A doer who still sends steps of a finished task missed the end of it, so it goes again.
local function AnswerAgain(task)
	if task.status == "done" then
		ns.TaskChannel.Whisper(task.doer, { type = "result", id = task.id, verdict = "done" })
	elseif task.status == "cancelled" then
		ns.TaskChannel.Whisper(task.doer, { type = "cancel", id = task.id })
	end
end

-- Each type from a doer of a task that you gave.
local FROM_DOER = {
	accept = function(task)
		if task.status == "offered" then
			Took(task)
			Say(Short(task.doer) .. " accepted " .. task.title .. ".")
		end
	end,
	decline = function(task)
		if ns.TaskStore.IsOpen(task) then
			Close(task, "declined")
			Say(Short(task.doer) .. " declined " .. task.title .. ".")
		end
	end,
	block = function(task)
		local data = ns.TaskStore.Data()
		ns.TaskStore.AddName(data.refusedBy, task.doer)
		if ns.TaskStore.IsOpen(task) then
			Close(task, "declined")
		end
	end,
	step = function(task, message)
		if not ns.TaskStore.IsOpen(task) then
			return AnswerAgain(task)
		end
		Took(task)
		Stepped(task, message)
	end,
	turnin = function(task, message)
		if not ns.TaskStore.IsOpen(task) then
			return AnswerAgain(task)
		end
		Took(task)
		TurnInAsked(task, message)
	end,
}

local function Verdict(task, message)
	if message.verdict == "done" then
		Close(task, "done")
		task.place = WhereYouStand()
		Say("Task complete: " .. task.title .. ".")
		return
	end
	task.turnInAt = nil
	Say(Short(task.giver) .. " says " .. task.title .. " isn't done yet.")
end

-- Each type from a giver of a task that you got.
local FROM_GIVER = {
	cancel = function(task)
		if ns.TaskStore.IsOpen(task) then
			Close(task, "cancelled")
			Say(Short(task.giver) .. " canceled " .. task.title .. ".")
		end
	end,
	result = function(task, message)
		if task.status == "accepted" then
			Verdict(task, message)
		end
	end,
}

local function Hello(sender)
	if ns.TaskPeople.Relation(sender) and not ns.TaskStore.Data().blocked[sender] then
		ns.TaskChannel.Whisper(sender, { type = "here" })
	end
end

-- A player who blocked you never answers a call, so an answer means that the block ended.
local function Here(sender)
	if not ns.TaskPeople.Relation(sender) then
		return
	end
	if not answered[sender] then
		if answeredCount >= MAX_ANSWERED then
			answered, answeredCount = {}, 0
		end
		answeredCount = answeredCount + 1
	end
	answered[sender] = true
	ns.TaskStore.Data().refusedBy[sender] = nil
	Changed()
end

-- `sender` is the full name that the game gave. Each message is checked against what this
-- addon knows: a hello comes to the group or the guild, every other type only as a whisper,
-- an offer only from party, guild, or friends, and the rest only from the other player of
-- its task.
function PlayerTasks.Receive(sender, message, channel)
	if message.type == "hello" then
		return Hello(sender)
	end
	if channel ~= "WHISPER" then
		return
	end
	if message.type == "here" then
		return Here(sender)
	end
	if message.type == "offer" then
		if ns.TaskPeople.Relation(sender) then
			Offered(sender, message)
		end
		return
	end
	local given, received = GivenTo(sender, message.id), ReceivedFrom(sender, message.id)
	if given and FROM_DOER[message.type] then
		FROM_DOER[message.type](given, message)
		Changed()
	elseif received and FROM_GIVER[message.type] then
		FROM_GIVER[message.type](received, message)
		Changed()
	end
end

-- For the pages and the tracker --------------------------------------------------------------

function PlayerTasks.Given()
	return ns.TaskStore.List(ns.TaskStore.Data().given)
end

function PlayerTasks.Received()
	return ns.TaskStore.List(ns.TaskStore.Data().received)
end

-- The tasks that you got and accepted: the ones whose steps your addon watches.
function PlayerTasks.Doing()
	local doing = {}
	for _, entry in ipairs(PlayerTasks.Received()) do
		if entry.task.status == "accepted" then
			doing[#doing + 1] = entry.task
		end
	end
	return doing
end

-- The tasks that you gave and that are still open: the ones your addon witnesses.
function PlayerTasks.Watching()
	local watching = {}
	for _, entry in ipairs(PlayerTasks.Given()) do
		if ns.TaskStore.IsOpen(entry.task) then
			watching[#watching + 1] = entry.task
		end
	end
	return watching
end
