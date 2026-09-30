-- The hero: the sheet and the player's own lore (GAMEPLAY.md 3.7). The desktop keeps them.
-- Each edit goes out with a journal request. The book shows the edit at once, and the
-- journal that comes back replaces it.

local _, ns = ...

local Hero = {}
ns.Hero = Hero

Hero.FIELDS = { "origin", "background", "goal", "bond", "flaw", "traits" }
Hero.LABELS = {
	origin = "Origin",
	background = "Background",
	goal = "Goal",
	bond = "Bond",
	flaw = "Flaw",
	traits = "Traits",
}
Hero.HINTS = {
	origin = "Where is your character from?",
	background = "What did your character do before adventuring?",
	goal = "What does your character want?",
	bond = "Who or what does your character care about most?",
	flaw = "What is your character's biggest flaw?",
	traits = "How would you describe your character's personality?",
}

-- The desktop refuses a longer text, so the editor stops at the same length.
Hero.MAX_LETTERS = 1000
-- A letter outside ASCII takes up to 4 bytes, and the desktop also limits the bytes.
Hero.MAX_BYTES = 1200
local TOO_LONG = "Too long to save. Try a shorter version."

local asked = false

-- The edits that the desktop did not confirm yet, so the book shows them at once. The next
-- whole journal replaces them: it holds each saved edit and leaves out a refused one.
local unsaved

local function ForgetUnsaved()
	unsaved = { fields = {}, added = {}, removed = {} }
end
ForgetUnsaved()

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Send(input)
	ns.Outbox.Add(input)
	ns.Journal.Request(0)
	ns.JournalFrame.Refresh()
end

local function IsField(field)
	for _, name in ipairs(Hero.FIELDS) do
		if name == field then
			return true
		end
	end
	return false
end

-- The desktop refuses a control character, so a line break becomes a space.
local function Clean(text)
	local flat = tostring(text or ""):gsub("%s*[\r\n]+%s*", " ")
	return (flat:match("^%s*(.-)%s*$"))
end

function Hero.Problem(text)
	if #Clean(text) > Hero.MAX_BYTES then
		return TOO_LONG
	end
end

function Hero.Unsaved()
	return unsaved
end

function Hero.JournalCame()
	ForgetUnsaved()
end

-- An empty text clears the field.
function Hero.Set(field, text)
	text = Clean(text)
	unsaved.fields[field] = text
	Send(ns.Inputs.HeroSet(time(), field, text))
end

function Hero.Add(text, npc)
	text = Clean(text)
	if text == "" then
		return
	end
	table.insert(unsaved.added, { text = text, npc = npc })
	Send(ns.Inputs.HeroAdded(time(), text, npc))
end

StaticPopupDialogs.TIMEWAYS_HERO_REMOVE = {
	text = "Remove this note?\n\n%s",
	button1 = "Remove",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		unsaved.removed[data.number] = true
		Send(ns.Inputs.HeroRemoved(time(), data.number))
	end,
}

-- `current` is the text that the book shows now, so an unchanged text sends nothing.
function Hero.Edit(field, current)
	ns.JournalFrame.Edit({
		title = Hero.LABELS[field],
		hint = Hero.HINTS[field],
		text = current,
		limit = Hero.MAX_LETTERS,
		problem = Hero.Problem,
		save = function(text)
			if Clean(text) ~= (current or "") then
				Hero.Set(field, text)
			end
		end,
	})
end

function Hero.Write()
	ns.JournalFrame.Edit({
		title = "New note",
		hint = "Anything about your character. It becomes part of your story.",
		text = "",
		limit = Hero.MAX_LETTERS,
		problem = Hero.Problem,
		save = Hero.Add,
	})
end

-- The number goes out in JSON, which takes only a whole number.
function Hero.Remove(entry)
	local number = entry.number
	if type(number) == "number" and number >= 0 and number % 1 == 0 then
		StaticPopup_Show("TIMEWAYS_HERO_REMOVE", ns.Plain(tostring(entry.text)), nil, { number = entry.number })
	end
end

-- The reason that the desktop refused the last edit, once.
function Hero.ShowRefused(reason)
	if type(reason) == "string" and reason ~= "" then
		Say(ns.Plain(reason))
	end
end

local function IsEmpty(hero)
	return type(hero) ~= "table" or (#(hero.sheet or {}) == 0 and #(hero.entries or {}) == 0)
end

-- "Who are you?": the first time the book shows an empty hero in this session, it opens
-- the Hero page.
function Hero.AskOnce(hero)
	if asked or not IsEmpty(hero) or not ns.JournalFrame.IsShown() then
		return
	end
	asked = true
	ns.JournalFrame.Open("hero")
end

local USAGE = "Usage: /hero, /hero add <note>, /hero note <note about your target>, or /hero set <field> <answer>"

-- `/hero` opens the page. The words after it add to the story, or set a field.
function Hero.Command(message)
	local verb, rest = Clean(message):match("^(%S*)%s*(.-)$")
	if verb == "" then
		ns.JournalFrame.Open("hero")
	elseif verb == "add" and rest ~= "" then
		Hero.Add(rest)
	elseif verb == "note" and rest ~= "" then
		local npc = ns.Units.NpcName("target")
		if not npc then
			Say("Target someone first, or use /hero add.")
			return
		end
		Hero.Add(rest, npc)
	elseif verb == "set" then
		local field, text = rest:match("^(%S+)%s*(.-)$")
		if not IsField(field) then
			Say("Pick one of these fields: " .. table.concat(Hero.FIELDS, ", ") .. ".")
			return
		end
		Hero.Set(field, text)
	else
		Say(USAGE)
	end
end
