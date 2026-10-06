//! The setup window: a player who got the addon from `CurseForge` learns how to install the
//! desktop app (GAMEPLAY.md 5.12).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

/// Words of the code that a player never sees (CLAUDE.md, "UI copy").
const INTERNAL_WORDS: [&str; 8] = [
    "story program",
    "bridge",
    "slot",
    "strip",
    "batch",
    "fact",
    "model",
    "bard",
];

fn game_with_no_key() -> Game {
    let game = Game::with_transport();
    game.run("ns.key = nil");
    game
}

/// Each slot load gives a fresh body, as a desktop app that runs does.
fn game_with_the_desktop_app() -> Game {
    let game = Game::with_transport();
    game.run(
        "C_AddOns.LoadAddOn = function(name)
             wow.loaded[name] = true
             Timeways_SlotData = { proto = 1, now = time(), replies = {} }
             return true
         end",
    );
    game
}

/// Plays for this long: the clock moves, every ticker runs, and each timer runs when it is due.
fn wait(game: &Game, seconds: u32) {
    game.run(&format!(
        "for _ = 1, {seconds} do
             wow.now = wow.now + 1
             wow.RunTickers()
             local timers = wow.after
             wow.after = {{}}
             for _, timer in ipairs(timers) do
                 if timer.at <= wow.now then
                     timer.callback()
                 else
                     table.insert(wow.after, timer)
                 end
             end
         end"
    ));
}

fn login(game: &Game) {
    game.run("wow.Fire('PLAYER_ENTERING_WORLD')");
}

fn shown(game: &Game) -> bool {
    game.eval("ns.Welcome.IsShown()")
}

/// Each text of the window that shows: its labels, and the lines of its edit boxes.
fn texts(game: &Game) -> Vec<String> {
    game.eval(
        "local function InWindow(widget)
             while widget do
                 if widget == TimewaysWelcomeFrame then return true end
                 if not widget.shown then return false end
                 widget = widget.parent
             end
             return false
         end
         local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.text and InWindow(widget) then table.insert(out, widget:GetDisplayText()) end
         end
         return out",
    )
}

fn heading(game: &Game) -> String {
    texts(game)[1].clone()
}

#[test]
fn a_player_with_no_desktop_app_sees_the_setup_window_at_login() {
    let game = game_with_no_key();

    login(&game);
    wait(&game, 1);

    assert!(shown(&game));
    assert_eq!(heading(&game), "Install the desktop app");
}

#[test]
fn the_window_gives_the_install_line_for_windows_and_for_macos_and_linux() {
    let game = game_with_no_key();

    game.run("ns.Welcome.Open('setup')");

    let texts = texts(&game);
    let windows: String = game.eval("ns.Welcome.COMMANDS.windows");
    let unix: String = game.eval("ns.Welcome.COMMANDS.unix");
    assert!(
        windows.contains("eserilev.github.io/timeways/install.txt"),
        "{windows}"
    );
    assert!(
        unix.contains("eserilev.github.io/timeways/install.sh"),
        "{unix}"
    );
    assert!(texts.contains(&windows), "{texts:?}");
    assert!(texts.contains(&unix), "{texts:?}");
    assert!(
        texts.iter().any(|text| text.contains("Then restart WoW.")),
        "{texts:?}"
    );
}

#[test]
fn a_running_desktop_app_shows_no_window() {
    let game = game_with_the_desktop_app();

    login(&game);
    wait(&game, 120);

    assert!(!shown(&game));
}

#[test]
fn the_window_waits_a_minute_for_the_desktop_app() {
    let game = Game::with_transport();
    login(&game);

    wait(&game, 59);
    let before = shown(&game);
    wait(&game, 1);

    assert!(!before);
    assert!(shown(&game));
}

#[test]
fn missing_files_say_so_in_the_heading() {
    let game = Game::with_transport();

    login(&game);
    wait(&game, 60);

    assert_eq!(heading(&game), "Some Timeways files are missing");
}

#[test]
fn a_desktop_app_that_stopped_gets_the_restart_hint() {
    let game = game_with_the_desktop_app();
    wait(&game, 10);
    game.run("wow.now = wow.now + 3600");

    game.run("ns.Welcome.Open(ns.Welcome.Reason())");

    let texts = texts(&game);
    assert_eq!(texts[0], "Timeways Setup");
    assert_eq!(texts[1], "Can't reach the desktop app");
    assert!(
        texts
            .iter()
            .any(|text| text.contains("gnomish-relay restart")),
        "{texts:?}"
    );
}

#[test]
fn a_player_who_needs_an_install_gets_no_restart_hint() {
    let game = game_with_no_key();

    game.run("ns.Welcome.Open(ns.Welcome.Reason())");

    let texts = texts(&game);
    assert!(
        !texts
            .iter()
            .any(|text| text.contains("gnomish-relay restart")),
        "{texts:?}"
    );
}

#[test]
fn the_window_shows_once_each_session() {
    let game = game_with_no_key();
    login(&game);
    wait(&game, 1);
    game.run("TimewaysWelcomeFrame:Hide()");

    login(&game);
    wait(&game, 120);

    assert!(!shown(&game));
}

#[test]
fn timeways_help_opens_the_window_again() {
    let game = game_with_the_desktop_app();

    game.run("wow.Slash('/timeways', 'help')");

    assert!(shown(&game));
    assert!(!game.eval::<bool>("TimewaysJournalFrame ~= nil"));
}

#[test]
fn close_hides_the_window() {
    let game = game_with_no_key();
    game.run("ns.Welcome.Open('setup')");

    game.run("wow.Button('Close'):Click()");

    assert!(!shown(&game));
}

#[test]
fn a_click_on_an_install_line_selects_it_for_copying() {
    let game = game_with_no_key();
    game.run("ns.Welcome.Open('setup')");

    let highlighted: bool = game.eval(
        "local box = wow.EditBox()
         box.scripts.OnEditFocusGained(box)
         return box.highlighted == true",
    );

    assert!(highlighted);
}

#[test]
fn a_press_on_an_install_line_gives_it_the_cursor_and_selects_it() {
    let game = game_with_no_key();
    game.run("ns.Welcome.Open('setup')");

    game.run("wow.MouseDown(wow.EditBox())");

    assert!(game.eval::<bool>("return wow.EditBox():HasFocus()"));
    assert!(game.eval::<bool>("return wow.EditBox().highlighted == true"));
}

#[test]
fn the_setup_window_at_login_takes_no_cursor() {
    let game = game_with_no_key();

    login(&game);
    wait(&game, 1);

    assert!(shown(&game));
    assert!(game.eval::<bool>("return wow.focus == nil"));
}

#[test]
fn close_gives_the_cursor_of_an_install_line_back() {
    let game = game_with_no_key();
    game.run("ns.Welcome.Open('setup')");
    game.run("wow.MouseDown(wow.EditBox())");

    game.run("wow.Button('Close'):Click()");

    assert!(game.eval::<bool>("return wow.focus == nil"));
}

/// The text of each edit box of the window, as the method `get` gives it.
fn boxes(game: &Game, get: &str) -> Vec<String> {
    game.eval(&format!(
        "local out = {{}}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' then table.insert(out, widget:{get}()) end
         end
         return out"
    ))
}

#[test]
fn no_install_line_holds_a_pipe() {
    let game = game_with_no_key();

    game.run("ns.Welcome.Open('setup')");

    let windows: String = game.eval("ns.Welcome.COMMANDS.windows");
    let unix: String = game.eval("ns.Welcome.COMMANDS.unix");
    assert!(!windows.contains('|') && !unix.contains('|'));
    assert_eq!(
        boxes(&game, "GetDisplayText"),
        [windows.clone(), unix.clone()]
    );
    assert_eq!(boxes(&game, "GetText"), [windows, unix]);
}

#[test]
fn typing_in_an_install_line_puts_the_line_back() {
    let game = game_with_no_key();
    game.run("ns.Welcome.Open('setup')");

    let text: String = game.eval(
        "local box = wow.EditBox()
         box:SetText('oops')
         box.scripts.OnTextChanged(box, true)
         return box:GetDisplayText()",
    );

    assert_eq!(text, game.eval::<String>("ns.Welcome.COMMANDS.windows"));
}

#[test]
fn the_window_uses_no_internal_words() {
    let game = game_with_no_key();

    game.run("ns.Welcome.Open('offline')");

    for text in texts(&game) {
        let lower = text.to_lowercase();
        for word in INTERNAL_WORDS {
            assert!(!lower.contains(word), "{word:?} in {text:?}");
        }
    }
}

#[test]
fn the_readme_gives_the_same_install_lines_as_the_window() {
    let game = game_with_no_key();
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).unwrap();

    let windows: String = game.eval("ns.Welcome.COMMANDS.windows");
    let unix: String = game.eval("ns.Welcome.COMMANDS.unix");

    assert!(readme.contains(&windows), "{windows}");
    assert!(readme.contains(&unix), "{unix}");
}

/// A game after login whose link can take a message, but with no key to sign it.
fn logged_in_with_no_key() -> Game {
    let game = Game::new();
    game.run("ns.key = nil");
    game
}

fn nothing_waits_to_go(game: &Game) -> bool {
    game.eval("ns.Outbox.Waiting() == 0 and #sent == 0")
}

fn told_to_install(game: &Game) -> bool {
    game.printed()
        .iter()
        .any(|line| line.contains("needs its desktop app"))
}

#[test]
fn lore_with_no_desktop_app_opens_the_setup_window_and_asks_nothing() {
    let game = logged_in_with_no_key();

    game.run("wow.Slash('/lore', 'why is this tower in ruins?')");

    assert!(shown(&game));
    assert!(told_to_install(&game));
    assert!(nothing_waits_to_go(&game));
    assert_eq!(game.eval::<usize>("#ns.Lore.Entries()"), 0);
}

#[test]
fn talk_with_no_desktop_app_opens_the_setup_window_and_sends_nothing() {
    let game = logged_in_with_no_key();
    game.run("wow.units.target = { name = 'Innkeeper Farley' }");

    game.run("wow.Slash('/talk', 'hello')");

    assert!(shown(&game));
    assert!(told_to_install(&game));
    assert!(nothing_waits_to_go(&game));
}

#[test]
fn quest_with_no_desktop_app_opens_the_setup_window_and_asks_nothing() {
    let game = logged_in_with_no_key();
    game.run("wow.units.target = { name = 'Innkeeper Farley' }");

    game.run("wow.Slash('/quest', '')");

    assert!(shown(&game));
    assert!(told_to_install(&game));
    assert!(nothing_waits_to_go(&game));
}

#[test]
fn writing_help_with_no_desktop_app_opens_the_setup_window() {
    let game = logged_in_with_no_key();

    game.run("ns.TaskDraftHelp.Open()");

    assert!(shown(&game));
    assert!(told_to_install(&game));
    assert!(game.eval::<bool>("ns.TaskDraftHelp.State() == nil"));
}

#[test]
fn the_window_waits_for_the_end_of_combat() {
    let game = game_with_no_key();
    game.run("wow.combat = true");
    login(&game);
    wait(&game, 1);
    let in_combat = shown(&game);

    game.run("wow.combat = false; wow.Fire('PLAYER_REGEN_ENABLED')");

    assert!(!in_combat);
    assert!(shown(&game));
}

#[test]
fn a_later_session_gets_a_chat_line_in_place_of_the_window() {
    let game = game_with_no_key();
    game.run("TimewaysDB = { welcomeShown = { setup = true } }");

    login(&game);
    wait(&game, 1);

    assert!(!shown(&game));
    let printed = game.printed();
    assert!(
        printed.iter().any(|line| line.contains("/timeways help")),
        "{printed:?}"
    );
}

#[test]
fn the_window_at_login_is_remembered_for_later_sessions() {
    let game = game_with_no_key();

    login(&game);
    wait(&game, 1);

    assert!(game.eval::<bool>("TimewaysDB.welcomeShown.setup == true"));
}

#[test]
fn a_new_problem_opens_the_window_once_more() {
    let game = Game::with_transport();
    game.run("TimewaysDB = { welcomeShown = { setup = true } }");

    login(&game);
    wait(&game, 60);

    assert!(shown(&game));
    assert_eq!(heading(&game), "Some Timeways files are missing");
}
