#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{journal, pages};
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;

/// The reply line of the first page that the story program writes for this character.
fn journal_reply(character: &Character) -> String {
    let page = pages(journal(character)).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
    })
    .unwrap()
}

fn traveler() -> Character {
    let mut character = Character::new();
    character.reach_level(Tick(DAY), 12).unwrap();
    character
        .enter_zone(Tick(DAY), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(DAY), "Innkeeper Farley").unwrap();
    character
        .enter_zone(Tick(DAY), "Westfall", Some("Sentinel Hill"))
        .unwrap();
    character.reach_level(Tick(DAY), 13).unwrap();
    character
}

/// The page as text: one `style: text` entry for each line.
fn lines(game: &Game, section: &str) -> Vec<String> {
    game.eval(&format!(
        "local out = {{}}
         for _, line in ipairs(ns.Journal.Lines('{section}')) do
             table.insert(out, line.style .. ': ' .. line.text)
         end
         return out"
    ))
}

fn day(game: &Game) -> String {
    game.eval(&format!("date('%d %b %Y', {DAY})"))
}

#[test]
fn the_journal_opens_and_asks_the_desktop_for_its_pages() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    assert!(game.eval::<bool>("TimewaysJournalFrame:IsShown()"));
    assert_eq!(
        game.sent_inputs(),
        [Input::JournalAsked {
            id: MessageId(1),
            page: 0
        }]
    );
}

#[test]
fn the_journal_command_closes_an_open_journal() {
    let game = Game::new();

    game.run("wow.Slash('/journal', ''); wow.Slash('/timeways', '')");

    assert!(!game.eval::<bool>("TimewaysJournalFrame:IsShown()"));
}

#[test]
fn escape_closes_the_journal() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    assert!(game.eval::<bool>(
        "for _, name in ipairs(UISpecialFrames) do
             if name == 'TimewaysJournalFrame' then return true end
         end
         return false"
    ));
}

#[test]
fn the_pages_fill_while_the_desktop_answers() {
    let game = Game::new();

    assert_eq!(lines(&game, "places"), ["note: The pages fill with ink..."]);
}

#[test]
fn places_group_each_subzone_under_its_zone() {
    let game = Game::new();

    game.reply(&journal_reply(&traveler()));

    let visited = format!("text: First visited on {}.", day(&game));
    let expected = [
        "heading: Elwynn Forest".to_string(),
        visited.clone(),
        "entry: Goldshire".to_string(),
        "heading: Westfall".to_string(),
        visited,
        "entry: Sentinel Hill".to_string(),
    ];
    assert_eq!(lines(&game, "places"), expected);
}

#[test]
fn people_show_where_you_met_them() {
    let game = Game::new();

    game.reply(&journal_reply(&traveler()));

    let met = format!("text: Met in Goldshire, on {}.", day(&game));
    assert_eq!(
        lines(&game, "people"),
        ["entry: Innkeeper Farley".to_string(), met]
    );
}

#[test]
fn deeds_begin_at_the_first_level_and_follow_each_level_up() {
    let game = Game::new();

    game.reply(&journal_reply(&traveler()));

    let expected = [
        "entry: Began this journal at level 12".to_string(),
        format!("text: {}.", day(&game)),
        "entry: Reached level 13".to_string(),
        format!("text: Sentinel Hill, {}.", day(&game)),
    ];
    assert_eq!(lines(&game, "deeds"), expected);
}

#[test]
fn an_empty_section_shows_a_note() {
    let game = Game::new();

    game.reply(&journal_reply(&Character::new()));

    assert_eq!(lines(&game, "places"), ["note: You have not traveled yet."]);
    assert_eq!(lines(&game, "people"), ["note: You have met no one yet."]);
    assert_eq!(
        lines(&game, "deeds"),
        ["note: Your deeds are not written yet."]
    );
}

#[test]
fn the_open_page_shows_the_lines_of_its_section() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");

    game.reply(&journal_reply(&traveler()));

    let shown: Vec<String> =
        game.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert_eq!(shown.first().map(String::as_str), Some("Elwynn Forest"));
    assert_eq!(shown.len(), 6);
}

#[test]
fn a_tab_opens_its_section_and_marks_itself() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(&journal_reply(&traveler()));

    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.text == 'People' then widget:Click() end
         end",
    );

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "people");
    let people_enabled: bool = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.text == 'People' then return widget:IsEnabled() end
         end",
    );
    assert!(!people_enabled);
}

#[test]
fn a_shorter_page_hides_the_lines_of_a_longer_one() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(&journal_reply(&traveler()));

    game.run("ns.JournalFrame.Open('people')");

    let shown: Vec<String> =
        game.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert_eq!(shown.len(), 2);
}

#[test]
fn a_bar_in_a_name_cannot_start_a_wow_escape() {
    let game = Game::new();
    let mut character = Character::new();
    character.meet_npc(Tick(DAY), "|cffff0000Fake|r").unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(lines(&game, "people")[0], "entry: ||cffff0000Fake||r");
}

#[test]
fn a_broken_journal_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"places":"x","people":[{"name":5}],"deeds":[{"kind":"level"},{"kind":"odd"}]}"#,
    );

    assert_eq!(lines(&game, "places"), ["note: You have not traveled yet."]);
    assert_eq!(lines(&game, "people")[0], "entry: ?");
    assert_eq!(
        lines(&game, "deeds"),
        ["note: Your deeds are not written yet."]
    );
}

fn explorer() -> Character {
    let mut character = Character::new();
    for n in 0..400 {
        let zone = format!("A zone with a long name, so that pages fill fast, number {n}");
        character.enter_zone(Tick(DAY + n), &zone, None).unwrap();
    }
    character
}

fn page_reply(page: timeways_story::journal::Page) -> String {
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
    })
    .unwrap()
}

fn asked_pages(game: &Game) -> Vec<usize> {
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::JournalAsked { page, .. } => Some(page),
            _ => None,
        })
        .collect()
}

#[test]
fn the_addon_asks_for_each_next_page_and_joins_them() {
    let game = Game::new();
    let all = pages(journal(&explorer()));
    assert!(all.len() > 1);
    let count = all.len();

    for page in all {
        game.reply(&page_reply(page));
    }

    let places = lines(&game, "places")
        .iter()
        .filter(|line| line.starts_with("heading"))
        .count();
    assert_eq!(places, 400);
    assert_eq!(asked_pages(&game), (1..count).collect::<Vec<_>>());
}

#[test]
fn a_page_out_of_order_is_dropped() {
    let game = Game::new();
    let mut all = pages(journal(&explorer()));

    game.reply(&page_reply(all.remove(1)));

    assert_eq!(lines(&game, "places"), ["note: The pages fill with ink..."]);
    assert!(asked_pages(&game).is_empty());
}

#[test]
fn the_old_journal_stays_until_every_page_of_the_new_one_came() {
    let game = Game::new();
    game.reply(&journal_reply(&traveler()));

    game.reply(&page_reply(pages(journal(&explorer())).remove(0)));

    assert_eq!(lines(&game, "places")[0], "heading: Elwynn Forest");
}
