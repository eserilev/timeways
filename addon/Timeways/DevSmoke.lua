-- `/twdev smoke`: one command runs many features in order, through the real code of the
-- addon and the real outbox (TESTING.md, "One command to test everything"). Each step
-- checks what the addon can see itself, and records PASS, FAIL, or WAIT with a short
-- reason. The result goes to the desktop as a small dev line, and the story program writes
-- the log of the run, with its own checks, into the dev folder.

-- The windows of the addon have global names, and only _G reaches them.
--# selene: allow(global_usage)

local _, ns = ...

local Dev = ns.Dev

local DevSmoke = {}
ns.DevSmoke = DevSmoke

-- A step starts this long after the step before ended.
local GAP_SECONDS = 1
-- The longest wait for each kind of answer. A quest card takes one or two model calls,
-- and the talk window asks the journal for it every 10 seconds.
local WAITS = { narrator = 30, talk = 60, lore = 60, journal = 30, card = 180, room = 10 }
local FPS_SECONDS = 10
-- The run stops by itself after this time, also with steps left.
local MAX_SECONDS = 20 * 60
-- The limits of a line of the run (crates/story/src/dev_smoke.rs).
local MAX_REASON, MAX_TEXT, MAX_ERROR, MAX_ERRORS = 160, 300, 300, 5

local NPC = "Innkeeper Farley"
local LORE_QUESTION = "What happened to Edwin VanCleef?"
local VANCLEEF = "Edwin VanCleef"
-- The named windows that a step can leave open. The run closes them between steps.
local WINDOWS = {
	"TimewaysLoreFrame",
	"TimewaysJournalFrame",
	"TimewaysTalkFrame",
	"TimewaysWelcomeFrame",
}

-- The run of now: { started, index, phase, nextAt, deadline, heard, failure, context,
-- counts, errors, results, previousHandler }. Nil while no run is on. `heard` holds what
-- the desktop sent during the step, and `context` what a step leaves for a later one.
local run

-- One line of valid UTF-8 within `most` bytes, or nil for no text.
local function Clean(text, most)
	if type(text) ~= "string" then
		return nil
	end
	local line = ns.Utf8.Cut(text:gsub("%c", " "), most, most)
	return line ~= "" and line or nil
end

local function Pass(reason, text)
	return "pass", reason, text
end

local function Fail(reason)
	return "fail", reason
end

local function Wait(reason)
	return "wait", reason
end

-- The commands of `/twdev` that a step runs, by name.
local used = {}

-- `/twdev <message>`, as the player types it.
local function Command(message)
	used[message:match("^(%S+)")] = true
	return function()
		Dev.Command(message)
	end
end

local function Has(lines, text)
	for _, line in ipairs(lines) do
		if type(line.text) == "string" and line.text:find(text, 1, true) then
			return true
		end
	end
	return false
end

-- Only a level of ten, 20, and so on is a moment, and only above the level that the world
-- holds. `/twdev smoke` runs on a character of any level, so the step goes `above` levels
-- past the tens of the character: level 10 and 20 for a fresh one.
local function LevelUp(above)
	used.level = true
	return function()
		local level = UnitLevel("player")
		level = (type(level) == "number" and not issecretvalue(level)) and level or 1
		local milestone = math.min(60, math.floor(level / 10) * 10 + above)
		Dev.Command("level " .. milestone)
	end
end

local function TalkTo(words)
	return Command("talk " .. NPC .. " / " .. words)
end

-- The cursor is in a box that the player can see. A moving player or a fight keeps it out,
-- as Focus.lua wants.
local function CursorCheck(where)
	local speed = GetUnitSpeed("player")
	if InCombatLockdown() or issecretvalue(speed) or speed > 0 then
		return Wait("you moved or fought, so " .. where .. " didn't take the cursor")
	end
	local box = GetCurrentKeyBoardFocus()
	if not box or not box:IsVisible() then
		return Fail("the cursor isn't in " .. where)
	end
	return Pass("the cursor is in " .. where)
end

-- What the desktop sent since the step began: a narrator line, the answers, a journal.
local function Narrated(heard)
	if heard.narrator then
		return Pass(nil, heard.narrator)
	end
	return Wait("no narrator line in " .. WAITS.narrator .. " s")
end

local function LoreCheck(heard)
	if not heard.lore then
		return Wait("no answer in " .. WAITS.lore .. " s")
	end
	if not ns.LoreBook.IsShown() then
		return Fail("the lore book isn't open")
	end
	local entries = ns.Lore.Entries()
	local entry = entries[#entries]
	if not entry or entry.state ~= "answered" then
		return Fail("the lore book shows no answer")
	end
	return Pass(entry.text and "answered" or "nobody knows", entry.text)
end

local function TalkLines()
	return ns.TalkWindow.Lines()
end

local function TalkCheck(heard, words)
	if not heard.talk then
		return Wait("no answer in " .. WAITS.talk .. " s")
	end
	if not ns.TalkWindow.IsShown() then
		return Fail("the talk window isn't open")
	end
	if not Has(TalkLines(), "You: " .. words) then
		return Fail("the talk window lost your words")
	end
	if type(heard.talk.text) ~= "string" then
		return Wait(NPC .. " said nothing: no model answered")
	end
	local result, reason = CursorCheck("the reply box")
	return result, reason, heard.talk.text
end

local function AcceptButton()
	return _G.TimewaysTalkFrameAccept
end

local function CardShown()
	local button = AcceptButton()
	return button ~= nil and button:IsVisible()
end

-- The newest story that waits from this fake player, and its place in the list.
local function WaitingStory(short)
	local author = short .. "-" .. ns.DevPeer.REALM
	for index, story in ipairs(ns.PlayerStories.Waiting()) do
		if story.author == author then
			return index
		end
	end
end

local function StoryAnswer(short, title, answer, answered)
	return {
		name = "peer-story-" .. answer:lower(),
		run = Command("peer " .. short .. " story " .. title),
		check = function()
			local index = WaitingStory(short)
			if not index then
				return Fail("no story of " .. short .. " waits")
			end
			ns.PlayerStories[answer](index)
			if WaitingStory(short) then
				return Fail("the story of " .. short .. " still waits after " .. answer)
			end
			return Pass("the story of " .. short .. " is " .. answered)
		end,
	}
end

-- The quest that this fake player gave you, while it waits for your answer.
local function OfferedQuest(short)
	local giver = short .. "-" .. ns.DevPeer.REALM
	for _, entry in ipairs(ns.PlayerTasks.Received()) do
		if entry.task.giver == giver and entry.task.status == "offered" then
			return entry
		end
	end
end

local function QuestAnswer(short, answer, status)
	return {
		name = "peer-quest-" .. answer:lower(),
		run = Command("peer " .. short .. " quest"),
		check = function()
			local offered = OfferedQuest(short)
			if not offered then
				return Fail("no quest of " .. short .. " waits")
			end
			ns.PlayerTasks[answer](offered.key)
			if offered.task.status ~= status then
				return Fail("the quest of " .. short .. " is " .. tostring(offered.task.status))
			end
			return Pass("the quest of " .. short .. " is " .. status)
		end,
	}
end

-- A fake player answers `/story` with the room of its box.
local function RoomAnswer(short, room)
	local command = Command("peer " .. short .. " " .. room)
	return {
		name = "peer-" .. room,
		run = function(context)
			command()
			ns.PlayerStories.Ask(short .. "-" .. ns.DevPeer.REALM, function(answer)
				context.room = answer
			end)
		end,
		wait = "room",
		check = function(_, context)
			if context.room ~= room then
				return Fail("the box of " .. short .. " answered " .. tostring(context.room))
			end
			return Pass(ns.PlayerStories.RoomLine(room, short))
		end,
	}
end

local function TabCheck(section)
	return {
		name = "tab-" .. ns.Journal.TITLES[section]:lower(),
		run = function()
			ns.JournalFrame.Open(section)
		end,
		check = function()
			if not ns.JournalFrame.IsShown() or ns.JournalFrame.Section() ~= section then
				return Fail("the " .. ns.Journal.TITLES[section] .. " tab isn't open")
			end
			local lines = ns.Journal.Lines(section)
			if #lines == 0 or Has(lines, "Loading...") then
				return Fail("the " .. ns.Journal.TITLES[section] .. " tab shows nothing")
			end
			return Pass(#lines .. " lines", lines[1].text)
		end,
	}
end

local function WelcomeCheck(reason)
	return {
		name = "welcome-" .. reason,
		run = Command("welcome " .. reason),
		check = function()
			if not ns.Welcome.IsShown() then
				return Fail("the setup window isn't open")
			end
			return Pass("the setup window is open")
		end,
	}
end

local startFps = Command("fps start smoke")

-- The steps, in order. Each step has a `name`, a `run` (a command of `/twdev`), and else
-- what it needs: `expect` (the kinds of the lines that it sends, which the desktop checks),
-- `wait` (an answer of the desktop to wait for), `seconds` (a fixed wait), `gate` (the
-- spoiler gate that the desktop checks), `check` (what the addon sees after it), and
-- `keepWindows` (the windows of the step before stay open).
local STEPS = {
	{
		name = "start",
		check = function()
			return Pass("dev mode is on")
		end,
	},
	{
		name = "zone-move",
		run = Command("zone Elwynn Forest / Goldshire"),
		expect = { "zone_entered" },
		wait = "narrator",
	},
	{ name = "subzone-move", run = Command("zone Elwynn Forest / Lion's Pride Inn"), expect = { "zone_entered" } },
	{ name = "capital", run = Command("zone Stormwind City / Trade District"), expect = { "zone_entered" } },
	{ name = "taxi", run = Command("taxi"), expect = { "zone_entered" } },
	{ name = "level-up", run = LevelUp(10), expect = { "level_reached" }, wait = "narrator" },
	{
		name = "dungeon-entry",
		run = Command("dungeon The Deadmines"),
		expect = { "zone_entered", "instance_entered" },
		wait = "narrator",
	},
	{
		name = "dungeon-again",
		run = Command("dungeon-again The Deadmines"),
		expect = { "zone_entered", "instance_entered" },
	},
	{
		name = "lore-before-kill",
		run = function()
			ns.Lore.Ask(LORE_QUESTION)
		end,
		wait = "lore",
		gate = { subject = VANCLEEF, expect = "blocked" },
		check = LoreCheck,
	},
	{ name = "rare-kill", run = Command("kill Hogger rare"), expect = { "npc_defeated" } },
	{
		name = "vancleef-kill",
		run = Command("kill " .. VANCLEEF .. " boss"),
		expect = { "npc_defeated" },
		wait = "narrator",
	},
	{
		name = "lore-after-kill",
		run = function()
			ns.Lore.Ask(LORE_QUESTION)
		end,
		wait = "lore",
		gate = { subject = VANCLEEF, expect = "open" },
		check = LoreCheck,
	},
	{ name = "death", run = Command("death Mor'Ladim"), expect = { "died" }, wait = "narrator" },
	{ name = "death-again", run = Command("death Mor'Ladim"), expect = { "died" } },
	{ name = "revenge", run = Command("kill Mor'Ladim rare"), expect = { "npc_defeated" } },
	{ name = "fall", run = Command("fall"), expect = { "died" } },
	-- The second death in lava earns the joke title "Lava Enthusiast".
	{ name = "lava", run = Command("lava"), expect = { "died" } },
	{ name = "lava-again", run = Command("lava"), expect = { "died" } },
	{ name = "level-up-again", run = LevelUp(20), expect = { "level_reached" } },
	{ name = "mount", run = Command("mount Brown Horse"), expect = { "mount_ridden" } },
	{
		name = "epic-mount",
		run = Command("mount Swift Brown Wolf epic"),
		expect = { "mount_ridden" },
		wait = "narrator",
	},
	{ name = "epic-item", run = Command("item Ironfoe epic"), expect = { "item_equipped" }, wait = "narrator" },
	{ name = "quest-step-kill", run = Command("kill Prowler"), expect = { "npc_killed" } },
	{ name = "seen-npc", run = Command("seen Prowler beast"), expect = { "npc_seen" } },
	{
		name = "game-quest",
		run = Command("game-quest Westfall Stew"),
		expect = { "game_quest_accepted", "game_quest_done" },
	},
	{ name = "class-quest", run = Command("game-quest The Tome of Valor class"), expect = { "game_quest_done" } },
	{ name = "quest-mark", run = Command("mark Call of Earth / Earth Sapta"), expect = { "quest_marked" } },
	{ name = "battleground", run = Command("bg-win"), expect = { "bg_won" } },
	{ name = "pvp-rank", run = Command("pvp-rank 3"), expect = { "pvp_rank" } },
	{ name = "rest", run = Command("rest"), expect = { "rest_changed" } },
	{ name = "raid", run = Command("raid Molten Core"), expect = { "instance_entered" } },
	{ name = "meet-npc", run = Command("meet " .. NPC), expect = { "npc_met" } },
	{ name = "gossip", run = Command("gossip " .. NPC .. " / Rest a while, friend."), expect = { "text_seen" } },
	{ name = "quest-text", run = Command("quest-text " .. NPC .. " / Lost Necklace"), expect = { "text_seen" } },
	{ name = "book", run = Command("book The Kingdom of Stormwind"), expect = { "text_seen" } },
	{ name = "slap", run = Command("slap " .. NPC), expect = { "npc_slapped" } },
	{ name = "emote", run = Command("emote dance / " .. NPC), expect = { "emote_done" } },
	{ name = "carry", run = Command("carry 3 Linen Cloth / " .. NPC), expect = { "items_held" } },
	{ name = "hour", run = Command("hour 22"), expect = { "hour_changed" } },
	{
		name = "talk",
		run = TalkTo("Any news?"),
		expect = { "talk_asked" },
		wait = "talk",
		check = function(heard)
			return TalkCheck(heard, "Any news?")
		end,
	},
	{
		name = "talk-work",
		run = TalkTo("Any work for me?"),
		expect = { "talk_asked" },
		wait = "talk",
		check = function(heard, context)
			context.workAsked = heard.talk ~= nil
			return TalkCheck(heard, "Any work for me?")
		end,
	},
	{
		name = "talk-quest-card",
		keepWindows = true,
		wait = "card",
		check = function(_, context)
			if not context.workAsked then
				return Wait("no talk asked for work")
			end
			if not CardShown() then
				return Wait("no quest card in " .. WAITS.card .. " s")
			end
			return Pass("the quest card shows Accept")
		end,
	},
	{
		name = "talk-accept",
		keepWindows = true,
		run = function(context)
			context.accepted = CardShown()
			if context.accepted then
				AcceptButton():Click()
			end
		end,
		expect = { "quest_accepted" },
		check = function(_, context)
			if not context.accepted then
				return Wait("no quest card to accept")
			end
			local lines = TalkLines()
			if not Has(lines, "You: Any work for me?") then
				return Fail("the talk window lost your words after Accept")
			end
			if not Has(lines, "Quest accepted.") then
				return Fail('the talk window shows no "Quest accepted."')
			end
			return Pass("the talk and the quest text stay")
		end,
	},
	{
		name = "player-quest",
		run = Command("quest " .. NPC),
		expect = { "quest_asked" },
	},
	StoryAnswer("Kobee", "The Bridge", "Accept", "accepted"),
	StoryAnswer("Morvane", "A Debt of Bandages", "Decline", "declined"),
	QuestAnswer("Kobee", "Accept", "accepted"),
	QuestAnswer("Liandra", "Decline", "declined"),
	RoomAnswer("Brokka", "full"),
	RoomAnswer("Thrandok", "blocked"),
	{
		name = "story-scroll-cursor",
		run = Command("peer Kobee write"),
		check = function()
			if not ns.StoryScroll.IsShown() then
				return Fail("the story scroll isn't open")
			end
			local result, reason = CursorCheck("the story scroll")
			ns.StoryScroll.Close()
			return result, reason
		end,
	},
	{
		name = "quest-form-cursor",
		run = function()
			ns.JournalFrame.Open("quests")
			ns.TaskForm.Open()
		end,
		check = function()
			if not ns.JournalFrame.IsShown() then
				return Fail("the quest form isn't open")
			end
			return CursorCheck("the quest form")
		end,
	},
	{
		name = "chapter-end",
		run = Command("chapter-end"),
		expect = { "game_quest_done", "zone_entered" },
		wait = "narrator",
	},
	{
		name = "journal",
		run = Command("journal"),
		wait = "journal",
		check = function(heard)
			if not heard.journal then
				return Wait("no journal in " .. WAITS.journal .. " s")
			end
			if not ns.JournalFrame.IsShown() then
				return Fail("the journal isn't open")
			end
			return Pass("the journal came")
		end,
	},
	TabCheck("hero"),
	TabCheck("chapters"),
	TabCheck("stories"),
	TabCheck("knowledge"),
	TabCheck("quests"),
	{
		name = "atlas",
		run = Command("atlas"),
		wait = "journal",
		check = function(heard)
			if not heard.journal then
				return Wait("no journal in " .. WAITS.journal .. " s")
			end
			if ns.JournalFrame.Section() ~= "knowledge" then
				return Fail("Knowledge isn't open")
			end
			if not Has(ns.Journal.Lines("knowledge"), "Dev Scout") then
				return Fail("Knowledge doesn't list Dev Scout")
			end
			return Pass("Knowledge lists the tour")
		end,
	},
	{
		name = "atlas-empty",
		run = Command("atlas empty"),
		check = function()
			if not Has(ns.Journal.Lines("knowledge"), "haven't been here") then
				return Fail("Knowledge doesn't say that you haven't been here")
			end
			return Pass("the empty place page shows")
		end,
	},
	{
		name = "ratings-link",
		check = function(_, context)
			if not ns.Ratings.IsOn() then
				return Wait("ratings are off: type /timeways ratings on")
			end
			if not context.narratorId then
				return Wait("no narrator line had an ID")
			end
			if ns.Ratings.LinkFor(context.narratorId) == "" then
				return Fail("the narrator line has no [Rate] link")
			end
			return Pass("the narrator line ends in [Rate]")
		end,
	},
	WelcomeCheck("setup"),
	WelcomeCheck("files"),
	WelcomeCheck("offline"),
	{
		name = "fps",
		run = function(context)
			context.fpsStarted = not ns.DevFps.IsRunning()
			if context.fpsStarted then
				startFps()
			end
		end,
		seconds = FPS_SECONDS,
		check = function(_, context)
			if not context.fpsStarted then
				return Wait("another FPS run is on")
			end
			context.fps = ns.DevFps.Stop()
			if not context.fps then
				return Wait("the game hid every frame rate")
			end
			return Pass(string.format("median %d, low 5%% %d", context.fps.median, context.fps.p5))
		end,
	},
}

-- The commands of `/twdev` that the steps run.
function DevSmoke.Commands()
	local names = {}
	for name in pairs(used) do
		names[#names + 1] = name
	end
	table.sort(names)
	return names
end

-- The names of the steps, in order.
function DevSmoke.Steps()
	local names = {}
	for _, step in ipairs(STEPS) do
		names[#names + 1] = step.name
	end
	return names
end

-- The commands of `/twdev` that no step runs, and why. A new command goes into a step, or
-- here (crates/addon-tests/tests/dev_smoke.rs).
DevSmoke.LEFT_OUT = {
	drown = "the same line as fall",
	inbox = "five stories that wait: the story steps cover the box",
	msp = "it needs Share on in your Roleplay Profile",
	remember = "a mark in the saved players, with no window to check",
	note = "it opens the editor, which the quest form covers",
	tooltip = "it shows the tooltip of the game, which the addon can't read back",
	smoke = "this run",
}

-- The kinds of narrator moments that no step makes, and why (crates/addon-tests/tests/
-- dev_smoke.rs).
DevSmoke.LEFT_OUT_MOMENTS = {
	quest_done = "a side quest ends only after its steps, and a model writes the steps",
}

local function CloseWindows()
	if ns.StoryScroll.IsShown() then
		ns.StoryScroll.Close()
	end
	for _, name in ipairs(WINDOWS) do
		local window = _G[name]
		if window then
			window:Hide()
		end
	end
end

local function Remember(message)
	if #run.errors < MAX_ERRORS then
		run.errors[#run.errors + 1] = Clean(tostring(message), MAX_ERROR)
	end
end

local function Send(line)
	line.dev = true
	ns.Outbox.Add(line)
	ns.Outbox.Flush()
end

local function Record(step, result, reason, text)
	run.counts[result] = run.counts[result] + 1
	run.results[#run.results + 1] = { name = step.name, result = result, reason = reason }
	if result == "fail" then
		Dev.Say(string.format("smoke step %d %s failed: %s", run.index, step.name, tostring(reason)))
	end
	Send({
		type = "dev_smoke",
		run = run.started,
		step = run.index,
		name = step.name,
		result = result,
		reason = Clean(reason, MAX_REASON),
		text = Clean(text, MAX_TEXT),
		expect = step.expect,
		gate = step.gate,
	})
end

-- Runs the check with each line that it adds marked as fake, and gives its verdict.
local function Checked(check)
	local verdict = {}
	Dev.Faking(function()
		verdict = { check(run.heard, run.context) }
	end)
	return unpack(verdict)
end

-- A step with no check passes when its run raised no error: the desktop checks its lines.
local function Judge(step)
	if run.failure then
		return Fail(run.failure)
	end
	if not step.check then
		if step.wait == "narrator" then
			return Narrated(run.heard)
		end
		return Pass()
	end
	local ok, result, reason, text = pcall(Checked, step.check)
	if not ok then
		Remember(result)
		return Fail("Lua error: " .. tostring(result))
	end
	return result, reason, text
end

local function Ready(step)
	if step.seconds then
		return false
	end
	if step.wait == "card" then
		return CardShown()
	end
	if step.wait == "room" then
		return run.context.room ~= nil
	end
	return run.heard[step.wait] ~= nil
end

local function Finish(stopped)
	if not run then
		return
	end
	local finished = run
	seterrorhandler(finished.previousHandler)
	run = nil
	CloseWindows()
	Send({
		type = "dev_smoke_done",
		run = finished.started,
		steps = #finished.results,
		passed = finished.counts.pass,
		failed = finished.counts.fail,
		waited = finished.counts.wait,
		seconds = time() - finished.started,
		stopped = stopped,
		errors = finished.errors[1] and finished.errors or nil,
		fps = finished.context.fps,
	})
	Dev.Say(
		string.format(
			"Smoke test done: %d passed, %d failed, %d waited. On your computer, run: timeways-dev smoke --latest",
			finished.counts.pass,
			finished.counts.fail,
			finished.counts.wait
		)
	)
end

local function EndStep(step)
	local result, reason, text = Judge(step)
	Record(step, result, reason, text)
	run.phase = "gap"
	run.nextAt = time() + GAP_SECONDS
end

-- Every line that a step adds is fake, also a click of a button of the addon.
local function StartStep(step)
	if not step.keepWindows then
		CloseWindows()
	end
	run.heard = {}
	run.failure = nil
	local ok, problem = pcall(Dev.Faking, step.run or function() end, run.context)
	if not ok then
		Remember(problem)
		run.failure = "Lua error: " .. tostring(problem)
	end
	local seconds = step.seconds or WAITS[step.wait]
	if not seconds or run.failure then
		EndStep(step)
		return
	end
	run.phase = "wait"
	run.deadline = time() + seconds
end

local function Next()
	run.index = run.index + 1
	local step = STEPS[run.index + 1]
	if not step then
		Finish("done")
		return
	end
	StartStep(step)
end

-- The run waits while you fight, so no window opens and no box takes the cursor in combat.
local function Tick()
	if not run then
		return
	end
	if not Dev.IsOn() then
		seterrorhandler(run.previousHandler)
		run = nil
		return
	end
	if time() - run.started >= MAX_SECONDS then
		Finish("timeout")
		return
	end
	if InCombatLockdown() then
		return
	end
	local step = STEPS[run.index + 1]
	if run.phase == "wait" and (Ready(step) or time() >= run.deadline) then
		EndStep(step)
	elseif run.phase == "gap" and time() >= run.nextAt then
		Next()
	end
end

-- A reply of the desktop while a step waits: its narrator line, its answer, or a journal.
function DevSmoke.Heard(value)
	if not run or type(value) ~= "table" then
		return
	end
	if type(value.narrator) == "string" and value.narrator ~= "" then
		run.heard.narrator = value.narrator
		run.context.narratorId = value.narrator_id or run.context.narratorId
	end
	if value.type == "talk_answer" then
		run.heard.talk = value
	elseif value.type == "lore_answer" then
		run.heard.lore = value
	elseif value.type == "journal" then
		run.heard.journal = true
	end
end

function DevSmoke.IsRunning()
	return run ~= nil
end

-- The result of each step so far: { name, result, reason }.
function DevSmoke.Results()
	return run and run.results or {}
end

local function Start()
	if run then
		Dev.Say('a smoke test is on. Type "/twdev smoke stop" to end it.')
		return
	end
	if InCombatLockdown() then
		Dev.Say("leave combat first, then type /twdev smoke again.")
		return
	end
	run = {
		started = time(),
		index = -1,
		phase = "gap",
		nextAt = time(),
		heard = {},
		context = {},
		counts = { pass = 0, fail = 0, wait = 0 },
		errors = {},
		results = {},
		previousHandler = geterrorhandler(),
	}
	local previous = run.previousHandler
	seterrorhandler(function(message, ...)
		if run then
			Remember(message)
		end
		return previous(message, ...)
	end)
	Dev.Say(
		string.format(
			'smoke test started: %d steps, a few minutes. Stand still out of combat. Wait for "Smoke test done".',
			#STEPS
		)
	)
	Next()
end

Dev.Add("smoke", "smoke [stop]: run every feature in order, and write a log on your computer.", function(rest)
	if rest == "stop" then
		Finish("stopped")
	elseif rest == "" then
		Start()
	else
		Dev.Say("usage: /twdev smoke [stop]")
	end
end)

C_Timer.NewTicker(1, Tick)
