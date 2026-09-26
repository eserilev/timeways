-- The hero: the sheet and the player's own lore (GAMEPLAY.md 3.7). The desktop keeps them.
-- Each edit goes out with a journal request, so the page shows the result at once.

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
	origin = "Where your hero comes from.",
	background = "What your hero did before the adventure.",
	goal = "What your hero wants most.",
	bond = "A person or a place that your hero cares about.",
	flaw = "What gets your hero into trouble.",
	traits = "Your hero's manner, in a line or two.",
}

-- The desktop refuses a longer text, so the box stops at the same length.
local MAX_LETTERS = 300

local asked = false

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: " .. text)
end

local function Send(input)
	ns.Outbox.Add(input)
	ns.Journal.Request(0)
end

local function IsField(field)
	for _, name in ipairs(Hero.FIELDS) do
		if name == field then
			return true
		end
	end
	return false
end

local function Trim(text)
	return (tostring(text or ""):match("^%s*(.-)%s*$"))
end

-- An empty text clears the field.
function Hero.Set(field, text)
	Send(ns.Inputs.HeroSet(time(), field, Trim(text)))
end

function Hero.Add(text, npc)
	text = Trim(text)
	if text ~= "" then
		Send(ns.Inputs.HeroAdded(time(), text, npc))
	end
end

-- A dialog of the game with a text box. `data.save` gets the text on Save or Enter.
StaticPopupDialogs.TIMEWAYS_HERO_TEXT = {
	text = "%s",
	button1 = "Save",
	button2 = "Cancel",
	hasEditBox = 1,
	maxLetters = MAX_LETTERS,
	editBoxWidth = 350,
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnShow = function(dialog, data)
		dialog:GetEditBox():SetText(data.text or "")
		dialog:GetEditBox():SetFocus()
	end,
	OnAccept = function(dialog, data)
		data.save(dialog:GetEditBox():GetText())
	end,
	EditBoxOnEnterPressed = function(editBox, data)
		data.save(editBox:GetText())
		editBox:GetParent():Hide()
	end,
	EditBoxOnEscapePressed = function(editBox)
		editBox:GetParent():Hide()
	end,
}

StaticPopupDialogs.TIMEWAYS_HERO_REMOVE = {
	text = "Remove this from your story?\n\n%s",
	button1 = "Remove",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		Send(ns.Inputs.HeroRemoved(time(), data.number))
	end,
}

function Hero.Edit(field, current)
	local prompt = Hero.LABELS[field] .. ": " .. Hero.HINTS[field]
	StaticPopup_Show("TIMEWAYS_HERO_TEXT", prompt, nil, {
		text = current,
		save = function(text)
			Hero.Set(field, text)
		end,
	})
end

function Hero.Write()
	StaticPopup_Show("TIMEWAYS_HERO_TEXT", "Add to your story: a memory, a rumor, or a vow.", nil, {
		text = "",
		save = function(text)
			Hero.Add(text)
		end,
	})
end

function Hero.Remove(entry)
	if type(entry.number) == "number" then
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

local USAGE = "/hero, /hero add <text>, /hero note <text about your target>, or /hero set <field> <text>"

-- `/hero` opens the page. The words after it add to the story, or set a field.
function Hero.Command(message)
	local verb, rest = Trim(message):match("^(%S*)%s*(.-)$")
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
			Say("The fields are: " .. table.concat(Hero.FIELDS, ", ") .. ".")
			return
		end
		Hero.Set(field, text)
	else
		Say(USAGE)
	end
end
