-- The colors of the book: dark ink on the parchment, as the classic quest frame prints it,
-- and gold on the dark frame, as the Map and Quest Log prints it.

local _, ns = ...

ns.Ink = {
	text = { 0.18, 0.12, 0.06 },
	-- Hints and marks fade, so they never read like a text that the player wrote.
	faded = { 0.42, 0.33, 0.22 },
	title = { 0.36, 0.2, 0.07 },
	gold = { 0.94, 0.81, 0.48 },
	cream = { 0.95, 0.89, 0.71 },
	muted = { 0.79, 0.69, 0.54 },
	-- The row that is open, in the dark red of the buttons of the game.
	selected = { 0.6, 0.14, 0.09, 0.55 },
	-- On parchment the red bar is too loud, so the open row takes a tint of the brown ink.
	selectedOnParchment = { 0.36, 0.2, 0.07, 0.16 },
	night = { 0.08, 0.05, 0.03, 0.9 },
}
