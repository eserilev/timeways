-- Ratings of narrator text (GAMEPLAY.md 3.2.2): a [Rate] link at the end of each narrator
-- line, and two thumbs on a page of the Chronicle with a story. They are off until the
-- player turns them on. A rating names the exact text, never its text: the desktop finds
-- the text itself, and keeps the rating in the world of the character, on this computer.

-- The global comes from the TOC, so only _G can reach it.
--# selene: allow(global_usage)

local _, ns = ...

local Ratings = {}
ns.Ratings = Ratings

local GLOBAL = "TimewaysSettings"
local PREFIX = "|cffc8a064Timeways|r: "
local GREY = "|cff808080"
local LINK_TYPE = "timeways:rate:"
-- A Lua number holds every whole number up to 2^53 exactly.
local MAX_ID = 2 ^ 53

-- The reasons of a dislike, in the order of the menu: the code and the label.
Ratings.REASONS = {
	{ "wrong_lore", "Wrong lore" },
	{ "made_up_name", "Made-up name" },
	{ "boring", "Boring" },
	{ "too_long", "Too long" },
	{ "doesnt_fit", "Doesn't fit the moment" },
	{ "other", "Other" },
}

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

local function LabelOf(reason)
	for _, choice in ipairs(Ratings.REASONS) do
		if choice[1] == reason then
			return choice[2]
		end
	end
end

-- `rated` is "narrator", "chapter", "tale", or "summary", `rating` is "up" or "down", and
-- `reason` is a code of REASONS or nil.
function Ratings.Rate(rated, key, rating, reason)
	if not Ratings.IsOn() then
		return
	end
	ns.Outbox.Add(ns.Inputs.LineRated(time(), rated, key, rating, reason))
	ns.Outbox.Flush()
end

-- Fills `root` with Like and a Dislike submenu of the reasons.
local function FillMenu(root, choose)
	root:CreateButton("Like", function()
		choose("up")
	end)
	local dislike = root:CreateButton("Dislike")
	for _, choice in ipairs(Ratings.REASONS) do
		dislike:CreateButton(choice[2], function()
			choose("down", choice[1])
		end)
	end
end

function Ratings.OpenMenu(owner, choose)
	MenuUtil.CreateContextMenu(owner, function(_, root)
		FillMenu(root, choose)
	end)
end

local function IsId(id)
	return type(id) == "number" and id >= 0 and id <= MAX_ID and id == math.floor(id)
end

local function RateLink(id)
	return string.format("%s|Haddon:%s%d|h[Rate]|h|r", GREY, LINK_TYPE, id)
end

-- The [Rate] link after a narrator line, or "" while ratings are off or for a line with
-- no ID.
function Ratings.LinkFor(id)
	if not Ratings.IsOn() or not IsId(id) then
		return ""
	end
	return " " .. RateLink(id)
end

local function DoneText(rating, reason)
	if rating == "up" then
		return GREY .. "Liked|r"
	end
	local label = LabelOf(reason)
	return GREY .. (label and ("Disliked: " .. label) or "Disliked") .. "|r"
end

-- The link of the line changes in place, so the chat gets no new line.
local function MarkRated(id, rating, reason)
	local link = RateLink(id)
	local function HasLink(message)
		return type(message) == "string" and message:find(link, 1, true) ~= nil
	end
	local function Rated(message, ...)
		local first, last = message:find(link, 1, true)
		return message:sub(1, first - 1) .. DoneText(rating, reason) .. message:sub(last + 1), ...
	end
	DEFAULT_CHAT_FRAME:TransformMessages(HasLink, Rated)
end

local function RateLine(id, rating, reason)
	Ratings.Rate("narrator", id, rating, reason)
	MarkRated(id, rating, reason)
end

-- A click on a [Rate] link. The game sends every addon link to this event.
function Ratings.LinkClicked(_, link, _, _, chatFrame)
	local id = type(link) == "string" and tonumber(link:match("^addon:" .. LINK_TYPE .. "(%d+)$"))
	if not Ratings.IsOn() or not IsId(id) then
		return
	end
	Ratings.OpenMenu(chatFrame or UIParent, function(rating, reason)
		RateLine(id, rating, reason)
	end)
end

EventRegistry:RegisterCallback("SetItemRef", Ratings.LinkClicked, Ratings)

local COMMANDS = {
	["ratings on"] = function()
		Ratings.SetOn(true)
		Say("Ratings are on. Click [Rate] after a narrator line. Ratings stay on your computer.")
	end,
	["ratings off"] = function()
		Ratings.SetOn(false)
		Say("Ratings are off.")
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

-- What the open page of the Chronicle shows from the narrator: a chapter (or the prologue),
-- a tale, or the title page, with the table that holds its rating. A page with no story of
-- the narrator has nothing to rate.
local function RatedOf(page, journal)
	if page.chapter then
		local chapter = page.chapter
		return type(chapter.prose) == "string" and "chapter", chapter, "rating"
	end
	if page.tale then
		local tale = page.tale
		return type(tale.text) == "string" and "tale", tale, "rating"
	end
	return type(journal.summary) == "string" and "summary", journal, "summary_rating"
end

local NOUNS = { chapter = "chapter", tale = "tale", summary = "summary" }

-- The thumbs of the open page while ratings are on and the page shows a story of the
-- narrator, or nil. `rating` is the newest rating: from the journal, or from a click since.
function Ratings.Thumbs(page, journal)
	local rated, holder, field = RatedOf(page, journal)
	if not Ratings.IsOn() or not rated then
		return nil
	end
	local key = rated ~= "summary" and holder.first or nil
	local rating = holder[field]
	local function Choose(chosen, reason)
		Ratings.Rate(rated, key, chosen, reason)
		holder[field] = chosen
		ns.JournalFrame.Refresh()
	end
	return {
		noun = NOUNS[rated],
		rating = (rating == "up" or rating == "down") and rating or nil,
		like = function()
			Choose("up")
		end,
		dislike = function(owner)
			MenuUtil.CreateContextMenu(owner, function(_, root)
				for _, choice in ipairs(Ratings.REASONS) do
					root:CreateButton(choice[2], function()
						Choose("down", choice[1])
					end)
				end
			end)
		end,
	}
end
