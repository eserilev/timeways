-- `/timeways forget`: clears what the player typed and every prompt on the desktop. The
-- story and its proof stay (GAMEPLAY.md 5.14).

local _, ns = ...

local History = {}
ns.History = History

StaticPopupDialogs.TIMEWAYS_CLEAR_HISTORY = {
	text = "Clear your history?\n\nThis deletes what you typed to Timeways and what the AI was told. Your journal stays the same.",
	button1 = "Clear",
	button2 = "Cancel",
	timeout = 0,
	whileDead = 1,
	hideOnEscape = 1,
	OnAccept = function()
		ns.Outbox.Add(ns.Inputs.HistoryCleared(time()))
		DEFAULT_CHAT_FRAME:AddMessage("|cffc8a064Timeways|r: History cleared.")
	end,
}

-- Asks first, because the words never come back.
function History.Clear()
	StaticPopup_Show("TIMEWAYS_CLEAR_HISTORY")
end
