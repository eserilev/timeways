//! Remembered players (GAMEPLAY.md 4.9): your own mark and note on another player, from its
//! right-click menu, in its tooltip, and in the saved variables only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

/// A game where Ada targets Bram, another player.
fn game() -> Game {
    let game = Game::new();
    game.run(
        "wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }
         wow.units.target = { name = 'Bram', player = true, guid = 'Player-1-Bram' }",
    );
    game
}

/// The items of the Remember menu of the target, as `kind text` with `*` on a chosen mark.
fn remember_menu(game: &Game) -> Vec<String> {
    game.eval(
        "local root = wow.OpenMenu('MENU_UNIT_PLAYER', { unit = 'target', name = 'Bram' })
         local out = {}
         for _, item in ipairs(root.items) do
             if item.text == 'Remember' then
                 for _, sub in ipairs(item.items) do
                     local chosen = sub.isSelected and sub.isSelected() and '*' or ''
                     table.insert(out, sub.kind .. ' ' .. tostring(sub.text) .. chosen)
                 end
             end
         end
         return out",
    )
}

/// Clicks the item of the Remember menu of the target with this text.
fn click(game: &Game, text: &str) {
    game.run(&format!(
        "local root = wow.OpenMenu('MENU_UNIT_PLAYER', {{ unit = 'target', name = 'Bram' }})
         for _, item in ipairs(root.items) do
             for _, sub in ipairs(item.items) do
                 if sub.text == '{text}' then sub.callback() end
             end
         end"
    ));
}

fn tooltip(game: &Game) -> Vec<String> {
    game.eval("return wow.ShowTooltip('target')")
}

/// Types this text in the editor and clicks Save.
fn write(game: &Game, text: &str) {
    game.run(&format!(
        "wow.EditBox():SetText('{text}'); wow.Button('Save'):Click()"
    ));
}

#[test]
fn the_menu_of_a_player_offers_three_marks_and_a_note() {
    let game = game();

    let menu = remember_menu(&game);

    assert_eq!(
        menu,
        [
            "radio Friendly",
            "radio Neutral",
            "radio Avoid",
            "divider nil",
            "button Add a note",
        ]
    );
}

#[test]
fn a_mark_shows_in_the_menu_and_in_the_tooltip() {
    let game = game();

    click(&game, "Avoid");

    assert!(remember_menu(&game).contains(&"radio Avoid*".to_string()));
    assert_eq!(tooltip(&game), ["Avoid"]);
}

#[test]
fn a_note_goes_on_one_line_with_the_mark() {
    let game = game();
    click(&game, "Avoid");

    click(&game, "Add a note");
    write(&game, "Took the chest and left.");

    assert_eq!(tooltip(&game), ["Avoid: Took the chest and left."]);
    assert!(remember_menu(&game).contains(&"button Edit note".to_string()));
}

#[test]
fn add_a_note_opens_the_writing_page_with_the_cursor() {
    let game = game();

    click(&game, "Add a note");

    assert!(game.eval::<bool>("return wow.CursorAtEnd(wow.EditBox())"));
}

#[test]
fn edit_note_puts_the_cursor_at_the_end_of_the_note() {
    let game = game();
    click(&game, "Add a note");
    write(&game, "Good healer.");

    click(&game, "Edit note");

    assert!(game.eval::<bool>("return wow.CursorAtEnd(wow.EditBox())"));
}

#[test]
fn the_editor_starts_with_the_note_and_an_empty_note_clears_it() {
    let game = game();
    click(&game, "Add a note");
    write(&game, "Good healer.");

    click(&game, "Edit note");
    let before: String = game.eval("wow.EditBox():GetText()");
    write(&game, "");

    assert_eq!(before, "Good healer.");
    assert!(tooltip(&game).is_empty());
    assert!(game.eval::<bool>("return TimewaysPlayers['Bram-Stormrage'] == nil"));
}

#[test]
fn a_pasted_tab_becomes_a_space_and_keeps_the_note() {
    let game = game();
    click(&game, "Add a note");
    write(&game, "Good healer.");

    click(&game, "Edit note");
    write(&game, "Great\\thealer.");

    assert_eq!(tooltip(&game), ["Great healer."]);
}

#[test]
fn a_note_with_a_pipe_stays_in_the_editor_with_the_reason() {
    let game = game();

    click(&game, "Add a note");
    write(&game, "a || b");

    assert!(game.eval::<bool>("return ns.Editor.IsShown()"));
    let shown: Vec<String> = game.eval("wow.ShownTexts(wow.EditBox().parent.parent)");
    assert!(
        shown.contains(&"Notes can't hold the || sign. Take it out to save.".to_string()),
        "{shown:?}"
    );
    assert!(tooltip(&game).is_empty());
}

#[test]
fn forget_clears_the_mark_and_the_note() {
    let game = game();
    click(&game, "Friendly");

    click(&game, "Forget");

    assert!(tooltip(&game).is_empty());
    assert_eq!(remember_menu(&game).last().unwrap(), "button Add a note");
}

#[test]
fn a_player_from_the_chat_is_found_by_name_and_realm() {
    let game = game();

    game.run(
        "local root = wow.OpenMenu('MENU_UNIT_FRIEND', { name = 'Cora', server = 'Argent Dawn' })
         root.items[2].items[1].callback()",
    );

    let mark: String = game.eval("return TimewaysPlayers['Cora-ArgentDawn'].mark");
    assert_eq!(mark, "friendly");
}

#[test]
fn your_own_menu_and_an_npc_get_no_remember() {
    let game = game();

    let own: usize = game
        .eval("return #wow.OpenMenu('MENU_UNIT_PLAYER', { unit = 'player', name = 'Ada' }).items");
    let npc: usize =
        game.eval("return #wow.OpenMenu('MENU_UNIT_PLAYER', { name = 'Bad|Name' }).items");

    assert_eq!((own, npc), (0, 0));
}

#[test]
fn a_chat_name_with_no_character_part_gets_no_remember() {
    let game = game();

    let items: usize = game.eval(
        "return #wow.OpenMenu('MENU_UNIT_FRIEND', { name = '', server = 'Stormrage' }).items",
    );

    assert_eq!(items, 0);
}

#[test]
fn a_broken_saved_entry_is_dropped() {
    let game = game();
    game.run(
        "TimewaysPlayers = {
             ['Bram-Stormrage'] = { mark = 'avoid', note = 'Fine.', at = 5 },
             ['Cora-Stormrage'] = { mark = 'enemy', note = 'a|cffbad', at = 5 },
             ['Dax-Stormrage'] = { mark = 'friendly' },
             ['not a name'] = { mark = 'friendly', at = 5 },
             ['Eve-Stormrage'] = 'friendly',
             ['Finn-Stormrage'] = { mark = 'neutral', note = string.rep('a', 121), at = 5 },
         }",
    );

    let names: Vec<String> = game.eval(
        "ns.PlayerNotes.Of('x')
         local names = {}
         for name in pairs(TimewaysPlayers) do table.insert(names, name) end
         table.sort(names)
         return names",
    );

    assert_eq!(names, ["Bram-Stormrage", "Finn-Stormrage"]);
    assert_eq!(
        game.eval::<Option<String>>("return TimewaysPlayers['Finn-Stormrage'].note"),
        None
    );
}

#[test]
fn only_the_newest_players_stay() {
    let game = game();
    game.run(
        "TimewaysPlayers = {}
         for n = 1, ns.PlayerNotes.MAX_PLAYERS do
             TimewaysPlayers['P' .. n .. '-Stormrage'] = { mark = 'neutral', at = n }
         end",
    );

    click(&game, "Friendly");

    let count: usize = game.eval(
        "local count = 0 for _ in pairs(TimewaysPlayers) do count = count + 1 end return count",
    );
    assert_eq!(count, 300);
    assert!(game.eval::<bool>("return TimewaysPlayers['P1-Stormrage'] == nil"));
    assert!(game.eval::<bool>("return TimewaysPlayers['Bram-Stormrage'] ~= nil"));
}

#[test]
fn nothing_goes_to_the_desktop_or_to_another_player() {
    let game = game();
    game.run("wow.Fire('PLAYER_ENTERING_WORLD')");
    game.run("wow.now = wow.now + 1; wow.RunTickers()");
    let sent_before = game.sent().len();

    click(&game, "Avoid");
    click(&game, "Add a note");
    write(&game, "Took the chest and left.");
    game.run("wow.now = wow.now + 700; wow.RunTickers()");

    let messages: usize = game.eval("return #wow.addonSent");
    assert_eq!(messages, 0);
    let after = game.sent();
    assert!(
        after[sent_before..]
            .iter()
            .all(|text| !text.contains("Bram") && !text.contains("chest")),
        "{after:?}"
    );
}

#[test]
fn the_mark_shows_under_the_roleplay_name_of_the_player() {
    let game = game();
    game.run(
        "ns.MspProfile.SetSharing(true)
         wow.Fire('CHAT_MSG_ADDON_LOGGED', 'MSP2', '00A001001001NA1:Bram Stone', 'WHISPER',
             'Bram-Stormrage', 'Ada-Stormrage', 0, 0, '', 0)",
    );

    click(&game, "Avoid");

    assert_eq!(tooltip(&game), ["Bram Stone", "Avoid"]);
}
