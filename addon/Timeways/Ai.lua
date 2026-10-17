-- AI on or off (GAMEPLAY.md 5.6). The player chooses with `/timeways ai on|off`, and the
-- choice is saved for the account. The `ai` field of each journal confirms it, and the dev
-- switch of the desktop can turn AI off too.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local Ai = {}
ns.Ai = Ai

local GLOBAL = "TimewaysSettings"
local PREFIX = "|cffc8a064Timeways|r: "

Ai.TALK_OFF = "Talking to NPCs needs AI. To turn it on, type /timeways ai on."
Ai.QUESTS_OFF = "Quests from NPCs need AI. To turn it on, type /timeways ai on."

-- True after a journal said "off". A desktop from before the switch sends no `ai`.
local desktopOff = false
-- The changes of the setting in this session. A journal asked before the newest change
-- tells an old state.
local changes = 0

-- Any addon can write the saved variables, so only a plain `false` turns AI off.
function Ai.SettingIsOn()
	local settings = _G[GLOBAL]
	return not (type(settings) == "table" and settings.ai == false)
end

-- Off from either side wins.
function Ai.IsOn()
	return Ai.SettingIsOn() and not desktopOff
end

function Ai.Changes()
	return changes
end

-- From the first page of each journal. Only "off" turns AI off. `asked` is `Ai.Changes()`
-- when the batch of the journal went, or nil when the batch is unknown, as after a reload.
function Ai.DesktopSays(value, asked)
	if asked and asked < changes then
		return
	end
	desktopOff = value == "off"
end

-- The word of the `ai_set` line.
function Ai.Word()
	return Ai.SettingIsOn() and "on" or "off"
end

-- The journal that said off came before the change, so only a new journal counts.
local function SetOn(on)
	if type(_G[GLOBAL]) ~= "table" then
		_G[GLOBAL] = {}
	end
	_G[GLOBAL].ai = on
	desktopOff = false
	changes = changes + 1
	ns.Outbox.AiChanged()
	ns.JournalFrame.Refresh()
end

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

local COMMANDS = {
	on = function()
		SetOn(true)
		Say("AI is on.")
	end,
	off = function()
		SetOn(false)
		Say("AI is off. NPCs won't talk and the narrator stays quiet.")
	end,
	other = function()
		Say("Type /timeways ai on or /timeways ai off.")
	end,
}

-- Runs a command of `/timeways`, and returns false for a message that is no AI command.
function Ai.Command(message)
	local command = COMMANDS[ns.Switch.Read(message, "ai")]
	if not command then
		return false
	end
	command()
	return true
end

-- The line of `/timeways help`.
function Ai.HelpLine()
	if Ai.IsOn() then
		return "AI: On. To turn it off, type /timeways ai off."
	end
	return "AI: Off. To turn it on, type /timeways ai on."
end
