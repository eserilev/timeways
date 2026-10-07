-- Ratings of narrator text (GAMEPLAY.md 3.2.2): a like or a dislike on the newest narrator
-- line, a chapter, a tale, or the title page. They are off until the player turns them on.
-- A rating names what it rates, never its text: the desktop finds the text itself, and
-- keeps the rating in the world of the character, on this computer only.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local Ratings = {}
ns.Ratings = Ratings

local GLOBAL = "TimewaysSettings"
local PREFIX = "|cffc8a064Timeways|r: "

-- A narrator line came in this session, so `/timeways like` has a line to rate.
local heard = false

local function Say(text)
	DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. text)
end

-- Any addon can write the saved variables, so only a plain `true` turns ratings on.
function Ratings.IsOn()
	local settings = _G[GLOBAL]
	return type(settings) == "table" and settings.ratings == true
end

function Ratings.SetOn(on)
	if type(_G[GLOBAL]) ~= "table" then
		_G[GLOBAL] = {}
	end
	_G[GLOBAL].ratings = on
	ns.JournalFrame.Refresh()
end

function Ratings.Heard()
	heard = true
end

-- `rated` is "narrator", "chapter", "tale", or "summary", and `rating` is "up" or "down".
function Ratings.Rate(rated, first, rating)
	if not Ratings.IsOn() then
		return
	end
	ns.Outbox.Add(ns.Inputs.LineRated(time(), rated, first, rating))
	ns.Outbox.Flush()
	Say(rating == "up" and "Thanks. You liked it." or "Thanks. You disliked it.")
end

local function RateLine(rating)
	if not Ratings.IsOn() then
		Say("Ratings are off. Type /timeways ratings on to rate narrator lines.")
	elseif not heard then
		Say("No narrator line to rate yet.")
	else
		Ratings.Rate("narrator", nil, rating)
	end
end

local COMMANDS = {
	["ratings on"] = function()
		Ratings.SetOn(true)
		Say(
			"Ratings are on. Type /timeways like or /timeways dislike after a narrator line. They stay on your computer."
		)
	end,
	["ratings off"] = function()
		Ratings.SetOn(false)
		Say("Ratings are off.")
	end,
	like = function()
		RateLine("up")
	end,
	dislike = function()
		RateLine("down")
	end,
}

-- Runs a command of `/timeways`, and returns false for a message that is no rating command.
function Ratings.Command(message)
	local words = message:lower():match("^%s*(.-)%s*$"):gsub("%s+", " ")
	local command = COMMANDS[words]
	if not command then
		return false
	end
	command()
	return true
end

-- What the open page of the Chronicle shows from the narrator: a chapter, a tale, or the
-- title page. A page with no story of the narrator has nothing to rate.
local function RatedOf(page, journal)
	if page.chapter then
		local chapter = page.chapter
		return type(chapter.prose) == "string" and "chapter", chapter.first
	end
	if page.tale then
		local tale = page.tale
		return type(tale.text) == "string" and "tale", tale.first
	end
	return type(journal.summary) == "string" and "summary", nil
end

-- Adds Like and Dislike to `buttons` while ratings are on and the page shows a story of
-- the narrator.
function Ratings.AddButtons(buttons, page, journal)
	local rated, first = RatedOf(page, journal)
	if not Ratings.IsOn() or not rated then
		return
	end
	for _, choice in ipairs({ { "Like", "up" }, { "Dislike", "down" } }) do
		buttons[#buttons + 1] = ns.JournalRows.Button(choice[1], function()
			Ratings.Rate(rated, first, choice[2])
		end)
	end
end
