-- The names that a page of the journal links: a place or a person opens its page of
-- Knowledge, a chapter opens the Chronicle, and a quest opens the Quests tab (GAMEPLAY.md
-- 3.6). A link is a hyperlink of the game in the text of a line, and the page of the book
-- takes its click.

local _, ns = ...

local JournalLinks = {}
ns.JournalLinks = JournalLinks

local PREFIX = "timeways"

-- The dark red of the buttons of the game, so a name reads as one to click.
local COLOR = "|cff7a2410"

-- `key` is a number or a short word, never a name: the bridge doubles each `|` of a name,
-- and a doubled sign inside the link itself would end it.
function JournalLinks.Of(kind, key)
	return PREFIX .. ":" .. kind .. ":" .. tostring(key)
end

function JournalLinks.Text(kind, key, label)
	return string.format("%s|H%s|h%s|h|r", COLOR, JournalLinks.Of(kind, key), label)
end

-- The kind and the key of a link of the journal, or nil for any other link.
function JournalLinks.Parse(link)
	local kind, key = tostring(link):match("^" .. PREFIX .. ":(%a+):(.+)$")
	if not kind then
		return nil
	end
	return kind, tonumber(key) or key
end

-- Each kind opens its page. Knowledge and the Chronicle fill this table, so this file
-- loads before them.
JournalLinks.OPENERS = {}

function JournalLinks.Open(link)
	local kind, key = JournalLinks.Parse(link)
	local open = kind and JournalLinks.OPENERS[kind]
	if open then
		open(key)
	end
end

-- Every link in a text, in order, for the tests and the fuzzer.
function JournalLinks.InText(text)
	local links = {}
	for link in tostring(text):gmatch("|H(" .. PREFIX .. ":[^|]+)|h") do
		links[#links + 1] = link
	end
	return links
end
