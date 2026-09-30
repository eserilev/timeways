#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Journal, journal, pages};
use timeways_story::learned::{Learned, LearnedKind};
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;

/// The page before the desktop answers.
const FILLING: &str = "help: The ink is still drying... If the pages stay blank, start Gnomish Relay on your computer.";

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

    assert_eq!(lines(&game, "places"), [FILLING]);
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

    assert_eq!(
        lines(&game, "places"),
        ["help: You haven't set foot anywhere yet. The road is waiting."]
    );
    assert_eq!(
        lines(&game, "people"),
        ["help: No one knows your name yet. Try saying hello."]
    );
    assert_eq!(
        lines(&game, "deeds"),
        ["help: Nothing worth a song yet. Give it time."]
    );
}

#[test]
fn the_open_page_shows_the_lines_of_its_section() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");

    game.reply(&journal_reply(&traveler()));
    game.run("ns.JournalFrame.Open('places')");

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

/// The dark band under the parchment of the quest art, in offsets from the bottom left of
/// the 384 by 512 frame, where QuestFrame.xml puts Accept and Decline. Below it, the art
/// ends and a button floats over the world.
const BAND_LEFT: f64 = 22.0;
const BAND_RIGHT: f64 = 345.0;
const BAND_BOTTOM: f64 = 72.0;
const BAND_TOP: f64 = 96.0;

#[test]
fn the_tabs_stand_in_one_row_inside_the_dark_band_and_fit_their_labels() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    let tabs: Vec<Vec<f64>> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysJournalFrame
                 and widget.template == 'UIPanelButtonTemplate' then
                 local point = widget.point
                 table.insert(out, { point[4], point[5], widget.width, widget.height,
                     widget:GetStringWidth() })
             end
         end
         return out",
    );
    assert_eq!(tabs.len(), 7);
    for tab in tabs {
        let [x, y, width, height, label] = tab[..] else {
            panic!("{tab:?}")
        };
        // The last tab ends at the band edge, give or take a rounding of the sum.
        assert!(x >= BAND_LEFT && x + width <= BAND_RIGHT + 0.001, "{tab:?}");
        assert!(y >= BAND_BOTTOM && y + height <= BAND_TOP, "{tab:?}");
        assert!(width >= label + 6.0, "{tab:?}");
    }
}

#[test]
fn the_book_says_how_to_use_the_open_page() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(&journal_reply(&traveler()));

    game.run("ns.JournalFrame.Open('quests')");

    let shown: Vec<String> = game.eval("wow.ShownTexts(TimewaysJournalFrame)");
    assert!(
        shown.iter().any(|text| text.contains("/quest")),
        "{shown:?}"
    );
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
fn a_name_that_the_bridge_escaped_shows_as_it_is() {
    let game = Game::new();
    let mut character = Character::new();
    character.meet_npc(Tick(DAY), "||cffff0000Fake||r").unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(lines(&game, "people")[0], "entry: ||cffff0000Fake||r");
}

#[test]
fn a_broken_journal_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"places":"x","people":[{"name":5}],"deeds":[{"kind":"level"},{"kind":"odd"}]}"#,
    );

    assert_eq!(
        lines(&game, "places"),
        ["help: You haven't set foot anywhere yet. The road is waiting."]
    );
    assert_eq!(lines(&game, "people")[0], "entry: ?");
    assert_eq!(
        lines(&game, "deeds"),
        ["help: Nothing worth a song yet. Give it time."]
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

    assert_eq!(lines(&game, "places"), [FILLING]);
    assert!(asked_pages(&game).is_empty());
}

#[test]
fn the_old_journal_stays_until_every_page_of_the_new_one_came() {
    let game = Game::new();
    game.reply(&journal_reply(&traveler()));

    game.reply(&page_reply(pages(journal(&explorer())).remove(0)));

    assert_eq!(lines(&game, "places")[0], "heading: Elwynn Forest");
}

#[test]
fn a_kill_and_its_echo_show_as_deeds() {
    let game = Game::new();
    let mut character = Character::new();
    character
        .enter_zone(Tick(DAY), "Elwynn Forest", None)
        .unwrap();
    character.defeat_npc(Tick(DAY), "Hogger").unwrap();
    character.defeat_npc(Tick(DAY), "Hogger").unwrap();

    game.reply(&journal_reply(&character));

    let place = format!("text: Elwynn Forest, {}.", day(&game));
    let expected = [
        "entry: Defeated Hogger".to_string(),
        place.clone(),
        "entry: Defeated Hogger again (2 times)".to_string(),
        place,
    ];
    assert_eq!(lines(&game, "deeds"), expected);
}

#[test]
fn deaths_show_as_deeds_with_the_killer_when_known() {
    let game = Game::new();
    let mut character = Character::new();
    character.enter_zone(Tick(DAY), "Westfall", None).unwrap();
    character.die(Tick(DAY), Some("Defias Pillager")).unwrap();
    character.die(Tick(DAY), None).unwrap();

    game.reply(&journal_reply(&character));

    let place = format!("text: Westfall, {}.", day(&game));
    let expected = [
        "entry: Fell to Defias Pillager".to_string(),
        place.clone(),
        "entry: Died".to_string(),
        place,
    ];
    assert_eq!(lines(&game, "deeds"), expected);
}

#[test]
fn an_entry_that_is_not_a_table_is_skipped() {
    let game = Game::new();

    game.reply(r#"{"type":"journal","page":0,"pages":1,"places":[5],"people":[true,{"name":"Ada"}],"deeds":["x"]}"#);

    game.run(
        "wow.Slash('/journal', ''); ns.JournalFrame.Open('people'); ns.JournalFrame.Open('deeds')",
    );
    assert_eq!(
        lines(&game, "places"),
        ["help: You haven't set foot anywhere yet. The road is waiting."]
    );
    assert_eq!(lines(&game, "people")[0], "entry: Ada");
}

#[test]
fn the_journal_opens_on_the_chronicle() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "chapters");
}

#[test]
fn a_chapter_tells_what_was_new_in_one_session() {
    let game = Game::new();

    game.reply(&journal_reply(&traveler()));

    let expected = [
        "heading: Chapter 1".to_string(),
        format!("text: {}.", day(&game)),
        "entry: Traveled to Elwynn Forest and Westfall.".to_string(),
        "entry: Met Innkeeper Farley.".to_string(),
        "entry: Began this journal at level 12.".to_string(),
        "entry: Reached level 13.".to_string(),
    ];
    assert_eq!(lines(&game, "chapters"), expected);
}

#[test]
fn a_chapter_counts_what_it_left_out() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":4,"began":1790000000,"zones":["A","B","C"],"people":[],"deeds":[],"left_out":12}]}"#,
    );

    let lines = lines(&game, "chapters");
    assert_eq!(lines[2], "entry: Traveled to A, B and C.");
    assert_eq!(lines[3], "text: And 12 more.");
}

#[test]
fn no_chapter_yet_shows_a_note() {
    let game = Game::new();

    game.reply(&journal_reply(&Character::new()));

    assert_eq!(
        lines(&game, "chapters"),
        ["help: Your story hasn't started yet. Go make some trouble."]
    );
}

#[test]
fn a_slapped_npc_shows_the_slaps_and_the_trust_in_words() {
    let game = Game::new();
    let mut character = Character::new();
    character
        .enter_zone(Tick(DAY), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(DAY), "Innkeeper Farley").unwrap();
    character.slap(Tick(DAY), "Innkeeper Farley").unwrap();
    character.slap(Tick(DAY), "Innkeeper Farley").unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(
        lines(&game, "people")[2],
        "text: Slapped 2 times. Wary of you."
    );
}

#[test]
fn the_saga_of_the_bard_comes_before_the_list_of_its_chapter() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":1,"began":1790000000,"zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"Our hero rode west. ||Hfake||h"}]}"#,
    );

    let lines = lines(&game, "chapters");
    assert_eq!(lines[2], "prose: Our hero rode west. ||Hfake||h");
    assert_eq!(lines[3], "entry: Traveled to Westfall.");
}

#[test]
fn trust_shows_in_words_for_each_band() {
    let game = Game::new();
    let people: Vec<String> = [60, 50, 49, 10, 9, -9, -10, -49, -50, -100]
        .iter()
        .map(|trust| format!(r#"{{"name":"N","first_met":1790000000,"trust":{trust}}}"#))
        .collect();

    game.reply(&format!(
        r#"{{"type":"journal","page":0,"pages":1,"people":[{}]}}"#,
        people.join(",")
    ));

    let words: Vec<String> = lines(&game, "people")
        .into_iter()
        .filter(|line| !line.starts_with("entry") && !line.contains("Met "))
        .collect();
    let expected = [
        "Trusts you.",
        "Trusts you.",
        "Likes you.",
        "Likes you.",
        "Thinks little of you.",
        "Thinks little of you.",
        "Wary of you.",
        "Wary of you.",
        "Distrusts you.",
        "Distrusts you.",
    ];
    assert_eq!(words, expected.map(|word| format!("text: {word}")));
}

#[test]
fn a_title_shows_as_a_deed() {
    let game = Game::new();
    let mut character = Character::new();
    character
        .enter_zone(Tick(DAY), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character
        .earn_title(Tick(DAY), "Lord of the Goldshire Dance Floor")
        .unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(
        lines(&game, "deeds")[0],
        "entry: Earned the title Lord of the Goldshire Dance Floor"
    );
}

#[test]
fn the_footnotes_of_the_bard_follow_its_saga() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":1,"began":1790000000,"zones":[],"people":[],"deeds":[],"left_out":0,"prose":"Our hero rode west.","footnotes":["Nobody knows why.",5]}]}"#,
    );

    let lines = lines(&game, "chapters");
    assert_eq!(
        lines[2..],
        ["prose: Our hero rode west.", "note: * Nobody knows why."]
    );
}

fn learned_reply(entries: Vec<Learned>) -> String {
    let whole = Journal {
        learned: entries,
        ..Journal::default()
    };
    let page = pages(whole).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
    })
    .unwrap()
}

fn learned_entry(kind: LearnedKind, title: Option<&str>, npc: Option<&str>, text: &str) -> Learned {
    Learned {
        kind,
        title: title.map(str::to_string),
        npc: npc.map(str::to_string),
        place: Some("Stormwind City".to_string()),
        at: Tick(DAY),
        excerpt: text.to_string(),
    }
}

#[test]
fn the_learned_page_shows_a_book_with_the_name_of_your_character() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true }");
    let book = learned_entry(
        LearnedKind::Book,
        Some("The Kingdom of Stormwind"),
        None,
        "Well met, $N.",
    );

    game.reply(&learned_reply(vec![book]));

    assert_eq!(
        lines(&game, "learned"),
        [
            "entry: Read The Kingdom of Stormwind".to_string(),
            "prose: Well met, Ada.".to_string(),
            format!("text: Stormwind City, {}.", day(&game)),
        ]
    );
}

#[test]
fn the_learned_page_marks_a_rumor_as_only_a_rumor() {
    let game = Game::new();
    let rumor = learned_entry(
        LearnedKind::Rumor,
        None,
        Some("Innkeeper Farley"),
        "The gnolls grow bold.",
    );

    game.reply(&learned_reply(vec![rumor]));

    let shown = lines(&game, "learned");
    assert_eq!(shown[0], "entry: A rumor from Innkeeper Farley");
    assert!(shown[2].starts_with("text: Only a rumor."), "{shown:?}");
}

#[test]
fn the_learned_page_names_a_quest_and_the_npc_of_gossip() {
    let game = Game::new();
    let quest = learned_entry(
        LearnedKind::Quest,
        Some("Wanted: Hogger"),
        None,
        "Hogger must die.",
    );
    let gossip = learned_entry(
        LearnedKind::Gossip,
        None,
        Some("Guard Thomas"),
        "Stay safe.",
    );

    game.reply(&learned_reply(vec![quest, gossip]));

    let shown = lines(&game, "learned");
    assert_eq!(shown[0], "entry: The quest Wanted: Hogger");
    assert_eq!(shown[3], "entry: Heard from Guard Thomas");
}

#[test]
fn an_empty_learned_page_says_how_to_learn() {
    let game = Game::new();

    game.reply(&learned_reply(Vec::new()));

    assert!(lines(&game, "learned")[0].contains("Pick up a book"));
}

#[test]
fn a_journal_with_no_pages_keeps_the_book_as_it_is() {
    let game = Game::new();
    game.reply(&journal_reply(&traveler()));

    game.reply(r#"{"type":"journal","page":0,"pages":0,"narrator":null}"#);

    assert_eq!(lines(&game, "places")[0], "heading: Elwynn Forest");
}
