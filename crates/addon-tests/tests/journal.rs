#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{journal, pages};
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;

/// The page before the desktop answers.
const FILLING: &str = "help: Loading... If this stays empty, start Gnomish Relay on your computer.";

/// The reply line of the first page that the story program writes for this character.
fn journal_reply(character: &Character) -> String {
    let page = pages(journal(character)).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page: Box::new(page),
        notice: None,
        dev: None,
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

/// The page as text: one `style: text` entry for each line. A link shows as its name.
fn lines(game: &Game, section: &str) -> Vec<String> {
    game.eval(&format!(
        "local out = {{}}
         for _, line in ipairs(ns.Journal.Lines('{section}')) do
             local text = line.text:gsub('|c%x%x%x%x%x%x%x%x|H[^|]*|h(.-)|h|r', '%1')
             table.insert(out, line.style .. ': ' .. text)
         end
         return out"
    ))
}

/// The lines of the open chapter after "In this chapter".
fn in_this_chapter(game: &Game) -> Vec<String> {
    let lines = lines(game, "chapters");
    let start = lines
        .iter()
        .position(|line| line == "section: In this chapter")
        .unwrap_or_else(|| panic!("{lines:?}"));
    lines[start + 1..].to_vec()
}

/// A journal of one open chapter that holds these deeds, as JSON.
fn chapter_with_deeds(deeds: &str) -> String {
    format!(
        r#"{{"type":"journal","page":0,"pages":1,"chapters":[{{"number":1,"state":"open","began":1790000000,"zones":[],"people":[],"deeds":[{deeds}],"left_out":0}}]}}"#
    )
}

/// The count of the entries of one list of the journal that the book holds.
fn held(game: &Game, list: &str) -> usize {
    game.eval(&format!("#ns.Journal.Current().{list}"))
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

    assert_eq!(lines(&game, "knowledge"), [FILLING]);
}

#[test]
fn the_book_has_no_deeds_tab_and_each_deed_shows_in_its_chapter() {
    let game = Game::new();

    game.reply(&chapter_with_deeds(
        r#"{"kind":"defeated","foe":"Hogger","times":1,"at":1790000001}"#,
    ));

    let sections: Vec<String> = game.eval("ns.Journal.SECTIONS");
    assert!(!sections.contains(&"deeds".to_string()), "{sections:?}");
    assert_eq!(in_this_chapter(&game), ["text: Defeated: Hogger."]);
}

#[test]
fn the_open_page_shows_the_lines_of_its_section() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");

    game.reply(&journal_reply(&traveler()));
    game.run("ns.JournalFrame.Open('chapters')");

    let shown: Vec<String> =
        game.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert_eq!(shown.first().map(String::as_str), Some("Chapter 1"));
    assert_eq!(shown.len(), lines(&game, "chapters").len());
}

#[test]
fn a_tab_opens_its_section_and_marks_itself() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(&journal_reply(&traveler()));

    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.text == 'Knowledge' then widget:Click() end
         end",
    );

    assert_eq!(
        game.eval::<String>("ns.JournalFrame.Section()"),
        "knowledge"
    );
    let knowledge_enabled: bool = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.text == 'Knowledge' then return widget:IsEnabled() end
         end",
    );
    assert!(!knowledge_enabled);
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

    game.run("ns.JournalFrame.Open('chapters'); ns.JournalFrame.Open('stories')");

    let shown: Vec<String> =
        game.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert_eq!(shown.len(), lines(&game, "stories").len());
}

#[test]
fn a_name_that_the_bridge_escaped_shows_as_it_is() {
    let game = Game::new();
    let mut character = Character::new();
    character
        .defeat_npc(Tick(DAY), "||cffff0000Fake||r")
        .unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(
        in_this_chapter(&game),
        ["text: Defeated: ||cffff0000Fake||r."]
    );
}

#[test]
fn a_broken_journal_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"places":"x","people":[{"name":5,"trust":10}],"deeds":[{"kind":"level"},{"kind":"odd"}]}"#,
    );

    let knowledge = lines(&game, "knowledge");
    let empty =
        "help: Nothing yet. People you meet, books you read, and quests you finish show up here.";
    assert!(knowledge.contains(&empty.to_string()), "{knowledge:?}");
}

fn explorer() -> Character {
    let mut character = Character::new();
    for n in 0..400 {
        let foe = format!("A foe with a long name, so that pages fill fast, number {n}");
        character.defeat_npc(Tick(DAY + n), &foe).unwrap();
    }
    character
}

fn page_reply(page: timeways_story::journal::Page) -> String {
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page: Box::new(page),
        notice: None,
        dev: None,
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

    assert_eq!(held(&game, "deeds"), 400);
    assert_eq!(asked_pages(&game), (1..count).collect::<Vec<_>>());
}

#[test]
fn a_page_out_of_order_is_dropped() {
    let game = Game::new();
    let mut all = pages(journal(&explorer()));

    game.reply(&page_reply(all.remove(1)));

    assert_eq!(lines(&game, "knowledge"), [FILLING]);
    assert!(asked_pages(&game).is_empty());
}

#[test]
fn the_old_journal_stays_until_every_page_of_the_new_one_came() {
    let game = Game::new();
    game.reply(&journal_reply(&traveler()));

    game.reply(&page_reply(pages(journal(&explorer())).remove(0)));

    assert_eq!(held(&game, "places"), 4);
}

#[test]
fn a_kill_and_its_echo_show_in_their_chapter() {
    let game = Game::new();
    let mut character = Character::new();
    character
        .enter_zone(Tick(DAY), "Elwynn Forest", None)
        .unwrap();
    character.defeat_npc(Tick(DAY), "Hogger").unwrap();
    character.defeat_npc(Tick(DAY), "Hogger").unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(
        in_this_chapter(&game),
        [
            "text: Places: Elwynn Forest.",
            "text: Defeated: Hogger.",
            "text: Defeated Hogger again, 2 times.",
        ]
    );
}

#[test]
fn deaths_show_in_their_chapter_with_the_killer_when_known() {
    let game = Game::new();
    let mut character = Character::new();
    character.enter_zone(Tick(DAY), "Westfall", None).unwrap();
    character.die(Tick(DAY), Some("Defias Pillager")).unwrap();
    character.die(Tick(DAY), None).unwrap();

    game.reply(&journal_reply(&character));

    assert_eq!(
        in_this_chapter(&game)[1],
        "text: Deaths: Defias Pillager at Westfall and Unknown at Westfall."
    );
}

#[test]
fn an_entry_that_is_not_a_table_is_skipped() {
    let game = Game::new();

    game.reply(r#"{"type":"journal","page":0,"pages":1,"places":[5],"people":[true,{"name":"Ada","trust":10}],"deeds":["x"]}"#);

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");
    assert_eq!(held(&game, "people"), 1);
}

#[test]
fn the_journal_opens_on_the_chronicle() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "chapters");
}

#[test]
fn a_chapter_page_shows_its_dates_its_levels_and_what_happened_in_it() {
    let game = Game::new();

    game.reply(&journal_reply(&traveler()));

    let expected = [
        "note: Chapter 1".to_string(),
        "heading: Elwynn Forest".to_string(),
        format!("text: {} · Levels 12 to 13.", day(&game)),
        "help: This chapter isn't over yet. Its story comes when the next one starts.".to_string(),
        "section: In this chapter".to_string(),
        "text: Places: Elwynn Forest and Westfall.".to_string(),
        "text: People: Innkeeper Farley.".to_string(),
    ];
    assert_eq!(lines(&game, "chapters"), expected);
}

#[test]
fn in_this_chapter_names_each_kind_on_one_line_in_its_order() {
    let game = Game::new();

    game.reply(concat!(
        r#"{"type":"journal","page":0,"pages":1,"stories":[{"number":3,"title":"The Courtyard","paragraphs":["x"],"at":1790000100}],"#,
        r#""chapters":[{"number":8,"first":40,"began":1790000000,"ended":1790000200,"title":"Silverpine Forest","state":"closed","#,
        r#""zones":["Silverpine Forest"],"people":["Dalar Dawnweaver"],"deeds":["#,
        r#"{"kind":"defeated","foe":"Krethis","times":1,"at":1790000001},"#,
        r#"{"kind":"game_quest_done","title":"Arugal's Folly","at":1790000002},"#,
        r#"{"kind":"died","killer":"Bleak Worg","at":1790000003},"#,
        r#"{"kind":"died","at":1790000004,"place":"The Dead Field"},"#,
        r#"{"kind":"titled","title":"Wolf Friend","at":1790000005}],"#,
        r#""again":["Defeated Hogger again, 6 times."],"left_out":0}]}"#,
    ));

    let lines = lines(&game, "chapters");
    assert_eq!(
        lines[3..],
        [
            "section: In this chapter",
            "text: Places: Silverpine Forest.",
            "text: People: Dalar Dawnweaver.",
            "text: Defeated: Krethis.",
            "text: Quests: Arugal's Folly.",
            "text: Deaths: Bleak Worg and Unknown at The Dead Field.",
            "text: Stories: The Courtyard.",
            "entry: Earned the title Wolf Friend.",
            "text: Defeated Hogger again, 6 times.",
        ]
    );
}

#[test]
fn a_story_in_a_chapter_shows_your_name_in_place_of_your_mark() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");

    game.reply(concat!(
        r#"{"type":"journal","page":0,"pages":1,"stories":[{"number":3,"title":"$N at the Bridge","paragraphs":["x"],"at":1790000100}],"#,
        r#""chapters":[{"number":8,"first":40,"began":1790000000,"ended":1790000200,"title":"Silverpine Forest","state":"closed","#,
        r#""zones":["Silverpine Forest"],"people":[],"deeds":[],"left_out":0}]}"#,
    ));

    assert!(
        lines(&game, "chapters").contains(&"text: Stories: Ada at the Bridge.".to_string()),
        "{:?}",
        lines(&game, "chapters")
    );
}

#[test]
fn a_return_names_its_chapter_from_the_desktop() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":5,"title":"Return to Westfall","state":"closed","began":1790000000,"zones":["Westfall"],"people":[],"deeds":[],"left_out":0}]}"#,
    );

    assert_eq!(lines(&game, "chapters")[1], "heading: Return to Westfall");
}

#[test]
fn a_chapter_counts_what_it_left_out() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":4,"began":1790000000,"zones":["A","B","C"],"people":[],"deeds":[],"left_out":12}]}"#,
    );

    let lines = lines(&game, "chapters");
    assert_eq!(lines[4], "text: Places: A, B and C.");
    assert_eq!(lines[5], "text: And 12 more.");
}

#[test]
fn a_chapter_with_no_zone_is_named_by_its_number() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":4,"began":1790000000,"zones":[],"people":[],"deeds":[],"left_out":0}]}"#,
    );

    assert_eq!(lines(&game, "chapters")[0], "heading: Chapter 4");
}

#[test]
fn a_chapter_that_spans_two_days_shows_both() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":1,"began":1790000000,"ended":1790100000,"zones":["A"],"people":[],"deeds":[],"left_out":0}]}"#,
    );

    let dates: String = game.eval(
        "return date('%d %b %Y', 1790000000) .. ' to ' .. date('%d %b %Y', 1790100000) .. '.'",
    );
    assert_eq!(lines(&game, "chapters")[2], format!("text: {dates}"));
}

/// A journal with three chapters, in the zones A, B, and C.
const THREE_CHAPTERS: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"chapters":["#,
    r#"{"number":1,"state":"closed","began":1790000000,"zones":["A"],"people":[],"deeds":[],"left_out":0},"#,
    r#"{"number":2,"state":"closed","began":1790000000,"zones":["B"],"people":[],"deeds":[],"left_out":0},"#,
    r#"{"number":3,"state":"open","began":1790000000,"zones":[],"people":[],"deeds":[],"left_out":0}]}"#,
);

#[test]
fn the_contents_list_each_chapter_by_its_place_newest_first() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");

    game.reply(THREE_CHAPTERS);

    let titles: Vec<String> = game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('chapters').list) do table.insert(out, row.text) end
         return out",
    );
    assert_eq!(
        titles,
        [
            "Ada",
            "Chapters",
            "Chapter 3",
            "Chapter 2: B",
            "Chapter 1: A"
        ]
    );
}

#[test]
fn the_title_page_is_the_first_row_of_the_chronicle() {
    let game = Game::new();
    game.reply(THREE_CHAPTERS);

    let first: String = game.eval("return ns.Journal.Page('chapters').list[1].detail");
    game.run("ns.Journal.Select('chapters', 'title')");
    let footer: String = game.eval("return ns.Journal.Page('chapters').footer");

    assert_eq!(first, "Who you've become");
    assert_eq!(footer, "Your story so far, chapter by chapter.");
}

#[test]
fn the_title_page_puts_your_name_in_place_of_the_hero_mark() {
    let game = Game::new();
    game.run(
        "wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada', level = 12 }
         wow.race, wow.class = 'Undead', 'Paladin'",
    );
    game.reply(&THREE_CHAPTERS.replace(
        r#""page":0,"#,
        r#""page":0,"summary":"Brill trusts $N now.","#,
    ));
    game.run("ns.Journal.Select('chapters', 'title')");

    assert_eq!(
        lines(&game, "chapters"),
        [
            "heading: Ada",
            "text: Level 12 Undead Paladin",
            "prose: Brill trusts Ada now."
        ]
    );
}

#[test]
fn with_no_model_the_title_page_holds_the_header_alone() {
    let game = Game::new();
    game.reply(THREE_CHAPTERS);
    game.run("ns.Journal.Select('chapters', 'title')");

    assert_eq!(lines(&game, "chapters").len(), 2);
}

#[test]
fn the_chronicle_opens_on_the_newest_chapter() {
    let game = Game::new();

    game.reply(THREE_CHAPTERS);

    let selected: i64 = game.eval("ns.Journal.Page('chapters').selected");
    let footer: String = game.eval("ns.Journal.Page('chapters').footer");
    assert_eq!(selected, 3);
    assert_eq!(footer, "Chapter 3 of 3");
}

#[test]
fn previous_chapter_opens_the_one_before_and_the_newest_has_no_next() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(THREE_CHAPTERS);
    game.run("ns.JournalFrame.Open('chapters')");

    let next_enabled: bool = game.eval("return wow.Button('Next chapter'):IsEnabled()");
    game.run("wow.Button('Previous chapter'):Click()");

    assert!(!next_enabled);
    assert_eq!(lines(&game, "chapters")[1], "heading: B");
}

#[test]
fn before_the_first_summary_the_title_page_says_when_it_fills_in() {
    let game = Game::new();

    game.reply(&journal_reply(&Character::new()));

    assert_eq!(
        lines(&game, "chapters")[2],
        "help: Fills in when your first chapter ends."
    );
}

#[test]
fn the_saga_comes_before_the_list_of_its_chapter() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":1,"began":1790000000,"zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"Our hero rode west. ||Hfake||h"}]}"#,
    );

    let lines = lines(&game, "chapters");
    assert_eq!(lines[3], "prose: Our hero rode west. ||Hfake||h");
    assert_eq!(lines[4], "section: In this chapter");
}

#[test]
fn a_title_shows_in_its_chapter() {
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
        in_this_chapter(&game)[1],
        "entry: Earned the title Lord of the Goldshire Dance Floor."
    );
}

#[test]
fn the_footnotes_follow_their_saga() {
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

#[test]
fn a_journal_with_no_pages_keeps_the_book_as_it_is() {
    let game = Game::new();
    game.reply(&journal_reply(&traveler()));

    game.reply(r#"{"type":"journal","page":0,"pages":0,"narrator":null}"#);

    assert_eq!(held(&game, "places"), 4);
}

#[test]
fn a_quest_of_the_game_and_a_class_quest_show_in_their_chapter() {
    let game = Game::new();

    game.reply(&chapter_with_deeds(concat!(
        r#"{"kind":"game_quest_done","title":"Rattling the Rattlecages","at":1790000000,"place":null},"#,
        r#"{"kind":"class_quest_done","title":"Rediscovering the Light","at":1790000000,"place":null}"#,
    )));

    assert_eq!(
        in_this_chapter(&game),
        ["text: Quests: Rattling the Rattlecages and Rediscovering the Light."]
    );
}

#[test]
fn a_quest_mark_shows_in_its_chapter_with_its_quest() {
    let game = Game::new();

    game.reply(&chapter_with_deeds(
        r#"{"kind":"quest_marked","mark":"Touched by the Light","quest":"Rediscovering the Light","at":1790000000,"place":null}"#,
    ));

    assert_eq!(
        in_this_chapter(&game),
        ["entry: Touched by the Light, from Rediscovering the Light."]
    );
}

#[test]
fn mounts_and_gear_show_in_their_chapter() {
    let game = Game::new();

    game.reply(&chapter_with_deeds(concat!(
        r#"{"kind":"mounted","mount":"Gray Ram","epic":false,"at":1790000000,"place":null},"#,
        r#"{"kind":"mounted","mount":"Swift Gray Ram","epic":true,"at":1790000000,"place":null},"#,
        r#"{"kind":"epic_item","item":"Barman Shanker","at":1790000000,"place":null},"#,
        r#"{"kind":"upgraded","item":"Cruel Barb","at":1790000000,"place":null}"#,
    )));

    assert_eq!(
        in_this_chapter(&game),
        [
            "entry: Rode your first mount, Gray Ram.",
            "entry: Rode your first epic mount, Swift Gray Ram.",
            "entry: Equipped your first epic item, Barman Shanker.",
            "entry: Equipped Cruel Barb, a big upgrade.",
        ]
    );
}

/// Three chapters at level 8, 12, and 15, and a tale of the first one.
const BANDS_AND_A_TALE: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"chapters":["#,
    r#"{"number":1,"first":1,"state":"closed","began":1790000000,"levels":[7,8],"title":"Westfall","zones":["Westfall"],"people":[],"deeds":[],"left_out":0},"#,
    r#"{"number":2,"first":50,"state":"closed","began":1790000000,"levels":[8,12],"opened_by":"level","title":"Duskwood","zones":["Duskwood"],"people":[],"deeds":[],"left_out":0},"#,
    r#"{"number":3,"first":90,"state":"open","began":1790000000,"levels":[12,15],"title":"Redridge Mountains","zones":["Redridge Mountains"],"people":[],"deeds":[],"left_out":0}],"#,
    r#""tales":[{"first":20,"chapter":1,"instance":"The Deadmines","kind":"dungeon","began":1790000000,"runs":2,"#,
    r#""deeds":[{"kind":"defeated","foe":"Edwin VanCleef","times":1,"at":1790000001}],"again":["Defeated Cookie again, 2 times."],"left_out":0}]}"#,
);

fn contents(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('chapters').list) do
             table.insert(out, row.text .. (row.detail and (' / ' .. row.detail) or '') .. (row.mark and (' [' .. row.mark .. ']') or ''))
         end
         return out",
    )
}

#[test]
fn the_contents_group_the_chapters_by_level_band_newest_first() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");
    let day: String = game.eval(&format!("return date('%d %b %Y', {DAY})"));

    game.reply(BANDS_AND_A_TALE);

    assert_eq!(
        contents(&game),
        [
            "Ada / Who you've become".to_string(),
            "Levels 10 to 19".to_string(),
            format!("Chapter 3: Redridge Mountains / {day} · now"),
            format!("Chapter 2: Duskwood / {day} [Level 12]"),
            "Levels 1 to 9".to_string(),
            format!("Chapter 1: Westfall / {day} [Dungeon]"),
            "The Deadmines / Dungeon · 2 runs".to_string(),
        ]
    );
}

#[test]
fn a_tale_follows_its_chapter_in_the_book() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(BANDS_AND_A_TALE);
    game.run("ns.JournalFrame.Open('chapters')");
    game.run("ns.JournalFrame.Select(1)");

    game.run("wow.Button('Next chapter'):Click()");

    assert_eq!(lines(&game, "chapters")[1], "heading: The Deadmines");
    let footer: String = game.eval("return ns.Journal.Page('chapters').footer");
    assert_eq!(footer, "After chapter 1");
}

#[test]
fn a_tale_page_shows_its_kind_its_runs_its_first_kills_and_its_tally_lines() {
    let game = Game::new();
    game.reply(BANDS_AND_A_TALE);

    game.run("ns.Journal.Select('chapters', 'tale:20')");

    let day: String = game.eval(&format!("return date('%d %b %Y', {DAY})"));
    assert_eq!(
        lines(&game, "chapters"),
        [
            "note: Dungeon".to_string(),
            "heading: The Deadmines".to_string(),
            format!("text: {day} · 2 runs."),
            "section: In this dungeon".to_string(),
            "text: Defeated: Edwin VanCleef.".to_string(),
            "text: Defeated Cookie again, 2 times.".to_string(),
        ]
    );
}

#[test]
fn a_chapter_that_is_not_over_says_when_its_story_comes() {
    let game = Game::new();

    game.reply(BANDS_AND_A_TALE);

    assert_eq!(
        lines(&game, "chapters")[3],
        "help: This chapter isn't over yet. Its story comes when the next one starts."
    );
}

#[test]
fn a_battleground_win_and_a_pvp_rank_show_in_their_chapter() {
    let game = Game::new();

    game.reply(&chapter_with_deeds(concat!(
        r#"{"kind":"won_battle","battleground":"Warsong Gulch","at":1790000000},"#,
        r#"{"kind":"pvp_rank","rank":3,"at":1790000000}"#,
    )));

    assert_eq!(
        in_this_chapter(&game),
        [
            "entry: Won a battle in Warsong Gulch.",
            "entry: Reached PvP rank 3.",
        ]
    );
}
