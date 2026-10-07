//! The Stories tab of the journal (GAMEPLAY.md 3.6 and 4.8): the stories that wait, the
//! accepted ones, and the drafts, with a page for the open one.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

const NOW: i64 = 1_790_000_000;

/// Two stories that wait: Bram's has a title, Cora's has none and came later.
const WAITING: &str = "TimewaysStories = { waiting = {
     { id = 'b1', author = 'Bram-Stormrage', title = 'The Bridge', text = 'We held it.\\nThen we ran.', at = 1790000000 },
     { id = 'c1', author = 'Cora-Stormrage', title = '', text = '$N froze me too.', at = 1790000000 } },
     authors = { [1] = 'Ada-Stormrage' }, nextNumber = 2 }";

const ACCEPTED: &str = r#"{"type":"journal","page":0,"pages":1,"hero":{"sheet":[],"entries":[]},"stories":[{"number":1,"title":"Up the Stairs","paragraphs":["$N carried the pack.","Then $N charged for it."],"at":1789900000,"used":false},{"number":5,"paragraphs":["A tale that stays."],"at":1789900000,"used":true}]}"#;

fn game_with(saved: &str) -> Game {
    let game = Game::new();
    game.run(&format!("wow.now = {NOW}"));
    game.run("wow.units.player = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");
    game.run(saved);
    game
}

fn page_lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('stories')) do
             table.insert(out, line.style .. ': ' .. line.text)
         end
         return out",
    )
}

/// The rows of the list: a group or help as its text, an item as "title | detail | mark".
fn list(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('stories').list or {}) do
             if row.style == 'item' then
                 local faded = row.faded and ' (faded)' or ''
                 table.insert(out, row.text .. faded .. ' | ' .. row.detail .. ' | ' .. row.mark)
             else
                 table.insert(out, row.text)
             end
         end
         return out",
    )
}

fn buttons(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, button in ipairs(ns.Journal.Page('stories').buttons) do
             table.insert(out, button.label)
         end
         return out",
    )
}

fn click(game: &Game, label: &str) {
    game.run(&format!(
        "for _, button in ipairs(ns.Journal.Page('stories').buttons) do
             if button.label == '{label}' then button.run() end
         end"
    ));
}

fn select(game: &Game, key: &str) {
    game.run(&format!("ns.Journal.Select('stories', '{key}')"));
}

fn badge(game: &Game) -> Option<String> {
    game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent and widget.parent.text == 'Stories'
                 and widget:IsShown() and widget.text ~= '' then
                 return widget.text
             end
         end",
    )
}

#[test]
fn the_badge_counts_the_waiting_stories() {
    let game = game_with(WAITING);

    game.run("wow.Slash('/stories', '')");

    assert_eq!(badge(&game).as_deref(), Some("2"));
}

#[test]
fn the_badge_hides_with_no_waiting_story() {
    let game = game_with("TimewaysStories = {}");

    game.run("wow.Slash('/stories', '')");

    assert_eq!(badge(&game), None);
}

#[test]
fn stories_slash_command_opens_the_tab() {
    let game = game_with(WAITING);

    game.run("wow.Slash('/stories', '')");

    assert_eq!(
        game.eval::<String>("return ns.JournalFrame.Section()"),
        "stories"
    );
}

#[test]
fn waiting_stories_show_while_the_journal_loads() {
    let game = game_with(WAITING);

    let rows = list(&game);

    assert_eq!(
        rows,
        [
            "Waiting 2",
            "A story from Cora (faded) | Cora | Today",
            "The Bridge | Bram | Today",
            "Loading...",
        ]
    );
}

#[test]
fn a_story_with_no_title_shows_who_told_it() {
    let game = game_with(WAITING);

    let lines = page_lines(&game);

    assert_eq!(
        lines,
        [
            "note: New",
            "heading: A story from Cora",
            "note: By Cora · Today",
            "prose: $N froze me too.",
            "help: To report abuse, open Support in the game menu.",
        ]
    );
    assert_eq!(buttons(&game), ["Block player", "Decline", "Accept"]);
}

#[test]
fn a_waiting_story_reads_as_a_page_with_a_paragraph_each() {
    let game = game_with(WAITING);
    select(&game, "w:Bram-Stormrage:b1");

    let lines = page_lines(&game);

    assert_eq!(
        lines[1..4],
        [
            "heading: The Bridge",
            "note: By Bram · Today",
            "prose: We held it."
        ]
    );
    assert_eq!(lines[4], "prose: Then we ran.");
}

#[test]
fn accept_opens_the_next_waiting_story() {
    let game = game_with(WAITING);

    click(&game, "Accept");

    assert_eq!(page_lines(&game)[1], "heading: The Bridge");
}

#[test]
fn an_accept_that_does_not_fit_says_why_on_the_page() {
    let game = game_with(WAITING);
    game.run("linkLimit = 100");

    click(&game, "Accept");

    assert_eq!(
        page_lines(&game).pop().unwrap(),
        "hint: This story is too long to keep. Decline it, or ask Cora for a shorter one."
    );
}

#[test]
fn block_player_asks_first() {
    let game = game_with(WAITING);

    click(&game, "Block player");
    let waiting_before: usize = game.eval("return #ns.PlayerStories.Waiting()");
    game.run("wow.AcceptPopup()");

    assert_eq!(waiting_before, 2);
    assert_eq!(game.eval::<usize>("return #ns.PlayerStories.Waiting()"), 1);
    assert_eq!(game.eval::<String>("return wow.popups[1].text"), "Cora");
}

#[test]
fn at_twenty_waiting_the_list_asks_for_answers() {
    let game = game_with(
        "TimewaysStories = { waiting = {} }
         for n = 1, 20 do
             table.insert(TimewaysStories.waiting,
                 { id = 'w' .. n, author = 'P' .. n .. '-Stormrage', title = '', text = 'Hi.', at = 1 })
         end",
    );

    let rows = list(&game);

    assert_eq!(rows[21], "Answer some stories to get new ones.");
}

#[test]
fn the_reader_puts_your_name_in_place_of_the_hero_mark() {
    let game = game_with("TimewaysStories = { authors = { [1] = 'Ada-Stormrage' } }");
    game.reply(ACCEPTED);
    select(&game, "a:1");

    let lines = page_lines(&game);

    assert_eq!(
        lines,
        [
            "heading: Up the Stairs",
            "note: By Ada · Accepted 20 Sep",
            "prose: Corvin carried the pack.",
            "prose: Then Corvin charged for it.",
        ]
    );
    assert_eq!(buttons(&game), ["Remove"]);
}

#[test]
fn a_used_story_says_why_it_stays() {
    let game = game_with("TimewaysStories = {}");
    game.reply(ACCEPTED);

    let lines = page_lines(&game);

    assert_eq!(lines[0], "heading: A story from a friend");
    assert_eq!(
        lines.last().unwrap(),
        "hint: Your story uses this one, so it stays."
    );
    assert!(buttons(&game).is_empty());
}

#[test]
fn drafts_show_with_continue_and_delete_and_delete_asks_first() {
    let game = game_with(
        "TimewaysStories = { drafts = { { to = 'Morvane-Stormrage', title = '', text = 'We went.\\n\\nWe came back.', at = 1790000000 } } }",
    );
    game.reply(ACCEPTED);
    select(&game, "d:Morvane-Stormrage");

    let rows = list(&game);
    let lines = page_lines(&game);
    let shown = buttons(&game);
    click(&game, "Delete");
    let still_there: usize = game.eval("return #ns.StoryDrafts.All()");
    game.run("wow.AcceptPopup()");

    assert_eq!(
        rows[rows.len() - 2..],
        [
            "Drafts 1",
            "A story about Morvane (faded) | To Morvane | Today"
        ]
    );
    assert_eq!(
        lines,
        [
            "heading: A story about Morvane",
            "note: To Morvane · Today",
            "prose: We went.",
            "prose: We came back.",
        ]
    );
    assert_eq!(shown, ["Continue", "Delete"]);
    assert_eq!(still_there, 1);
    assert_eq!(game.eval::<usize>("return #ns.StoryDrafts.All()"), 0);
}

#[test]
fn no_stories_says_how_to_tell_one() {
    let game = game_with("TimewaysStories = {}");
    game.reply(
        ACCEPTED
            .replace(r#""stories":["#, r#""stories":[],"x":["#)
            .as_str(),
    );

    let lines = page_lines(&game);

    assert_eq!(
        lines,
        [
            "help: No stories yet.",
            "help: To tell one, target someone in your group and type /story.",
        ]
    );
}

#[test]
fn the_bar_counts_the_waiting_stories() {
    let game = game_with(WAITING);

    let footer: String = game.eval("return ns.Journal.Page('stories').footer");

    assert_eq!(footer, "2 waiting");
}

#[test]
fn the_hero_page_has_no_stories_part() {
    let game = game_with(WAITING);
    game.reply(ACCEPTED);

    let hero: Vec<String> = game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('hero')) do table.insert(out, line.text) end
         return out",
    );

    assert!(!hero.iter().any(|line| line.contains("Stories About You")));
    assert!(!hero.iter().any(|line| line.contains("We held it.")));
}

/// The buttons of Bram's page, kept after his story stopped waiting, as an open page keeps
/// them until the next draw.
fn buttons_of_a_story_that_no_longer_waits(game: &Game) {
    select(game, "w:Bram-Stormrage:b1");
    game.run(
        "kept = {}
         for _, button in ipairs(ns.Journal.Page('stories').buttons) do kept[button.label] = button.run end
         table.remove(ns.PlayerStories.Waiting(), 1)",
    );
}

#[test]
fn accept_on_a_story_that_no_longer_waits_takes_no_other_story() {
    let game = game_with(WAITING);
    buttons_of_a_story_that_no_longer_waits(&game);

    game.run("kept.Accept()");

    assert_eq!(game.eval::<usize>("return #ns.PlayerStories.Waiting()"), 1);
}

#[test]
fn decline_on_a_story_that_no_longer_waits_declines_no_other_story() {
    let game = game_with(WAITING);
    buttons_of_a_story_that_no_longer_waits(&game);

    game.run("kept.Decline()");

    assert_eq!(game.eval::<usize>("return #ns.PlayerStories.Waiting()"), 1);
}
