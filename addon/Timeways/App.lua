-- The names of Timeways for the shared transport of Gnomish Relay (relay SPEC.md 9.7,
-- decision 5). They differ from the relay names, so the two addons never share a global.

local _, ns = ...

ns.App = {
	slotPrefix = "Timeways_S%04d",
	slotData = "Timeways_SlotData",
	restore = "Timeways_Restore",
	live = "Timeways_Live",
	strip = "TimewaysStrip",
	saved = "TimewaysDB",
	title = "Timeways",
	helloChat = "story",
	-- The bridge answers a version out of its range with "Update Timeways." or
	-- "Update the desktop program: gnomish-relay update.".
	version = 1,
}
