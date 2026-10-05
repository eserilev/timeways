-- The building blocks of the pages of the journal: the checks of values from the desktop,
-- the lines of the parchment, and the rows of the list (Journal.lua, JournalQuests.lua).

local _, ns = ...

local JournalRows = {}
ns.JournalRows = JournalRows

-- The status of an edit that the desktop did not confirm yet.
JournalRows.SAVING = "Saving..."

-- Values from the desktop are checked here, so a broken journal shows gaps, not errors.
function JournalRows.List(value)
	return type(value) == "table" and value or {}
end

function JournalRows.Entries(value)
	local entries = {}
	for _, entry in ipairs(JournalRows.List(value)) do
		if type(entry) == "table" then
			entries[#entries + 1] = entry
		end
	end
	return entries
end

function JournalRows.Name(value)
	return type(value) == "string" and ns.Plain(value) or "?"
end

function JournalRows.Day(at)
	return type(at) == "number" and date("%d %b %Y", at) or "an unknown day"
end

-- `action` is an optional button of the line: { label = ..., run = function }.
function JournalRows.Line(style, text, action)
	return { style = style, text = text, action = action }
end

-- A row of the list on the left: a group title, a line of help, or an item that opens.
function JournalRows.Group(text)
	return { style = "group", text = text }
end

function JournalRows.Item(key, text, detail, mark)
	return { style = "item", key = key, text = text, detail = detail, mark = mark }
end

function JournalRows.Button(label, run, disabled)
	return { label = label, run = run, disabled = disabled }
end

-- The place in `keys` of the selected key, or `default` when the selected key is gone.
function JournalRows.OpenIndex(keys, selected, default)
	for n, key in ipairs(keys) do
		if key == selected then
			return n
		end
	end
	return default
end

function JournalRows.Keys(items)
	local keys = {}
	for n, item in ipairs(items) do
		keys[n] = item.key
	end
	return keys
end

-- The first kill is the true kill. Each later kill is an echo after a reset (5.13).
function JournalRows.DeedTitle(deed)
	if deed.kind == "level" and type(deed.to) == "number" then
		local what = deed.from and "Reached level %d" or "Started at level %d"
		return string.format(what, deed.to)
	elseif deed.kind == "defeated" and type(deed.times) == "number" then
		if deed.times == 1 then
			return "Defeated " .. JournalRows.Name(deed.foe)
		end
		return string.format("Defeated %s again (%d times)", JournalRows.Name(deed.foe), deed.times)
	elseif deed.kind == "titled" then
		return "Earned the title " .. JournalRows.Name(deed.title)
	elseif deed.kind == "quest_done" then
		return "Finished the quest " .. JournalRows.Name(deed.title)
	elseif deed.kind == "game_quest_done" then
		return "Finished the quest " .. JournalRows.Name(deed.title)
	elseif deed.kind == "class_quest_done" then
		return "Finished the class quest " .. JournalRows.Name(deed.title)
	elseif deed.kind == "quest_marked" then
		return JournalRows.Name(deed.mark) .. ", from " .. JournalRows.Name(deed.quest)
	elseif deed.kind == "mounted" then
		local first = deed.epic and "Rode your first epic mount, " or "Rode your first mount, "
		return first .. JournalRows.Name(deed.mount)
	elseif deed.kind == "epic_item" then
		return "Equipped your first epic item, " .. JournalRows.Name(deed.item)
	elseif deed.kind == "upgraded" then
		return "Equipped " .. JournalRows.Name(deed.item) .. ", a big upgrade"
	elseif deed.kind == "died" then
		return deed.killer and ("Killed by " .. JournalRows.Name(deed.killer)) or "Died"
	end
end
