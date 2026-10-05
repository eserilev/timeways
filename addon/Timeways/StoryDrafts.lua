-- Stories that you started and did not send yet (GAMEPLAY.md 4.8): one for each player, 10
-- in all. A draft keeps the text as you typed it, so it can wait for a player who is
-- offline or out of your group.

local _, ns = ...

local StoryDrafts = {}
ns.StoryDrafts = StoryDrafts

StoryDrafts.FULL = "You have 10 drafts. Send or delete one first."

local function Drafts()
	return ns.StorySaved.Data().drafts
end

local function IndexOf(to)
	for index, draft in ipairs(Drafts()) do
		if draft.to == to then
			return index
		end
	end
end

-- Newest first.
function StoryDrafts.All()
	local drafts = {}
	for _, draft in ipairs(Drafts()) do
		drafts[#drafts + 1] = draft
	end
	table.sort(drafts, function(a, b)
		return a.at > b.at
	end)
	return drafts
end

function StoryDrafts.For(to)
	local index = IndexOf(to)
	return index and Drafts()[index]
end

-- False when 10 drafts wait and none is for this player. Save never drops a draft.
function StoryDrafts.Save(to, title, text)
	local index = IndexOf(to)
	local drafts = Drafts()
	if not index and #drafts >= ns.StorySaved.MAX_DRAFTS then
		return false
	end
	local draft = { to = to, title = title, text = text, at = time() }
	drafts[index or #drafts + 1] = draft
	return true
end

function StoryDrafts.Delete(to)
	local index = IndexOf(to)
	if index then
		table.remove(Drafts(), index)
	end
end

StaticPopupDialogs.TIMEWAYS_DRAFT_DELETE = {
	text = "Delete this draft?",
	button1 = "Delete",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function(_, data)
		StoryDrafts.Delete(data.to)
		ns.JournalFrame.Refresh()
	end,
}

-- Asks first, because a deleted draft never comes back.
function StoryDrafts.AskDelete(to)
	StaticPopup_Show("TIMEWAYS_DRAFT_DELETE", nil, nil, { to = to })
end
