//! Knowledge as an atlas (GAMEPLAY.md 3.6): the map is the index, and a zone, a place, or a
//! person shows what the character knows of it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use serde_json::{Value, json};

const DAY: u64 = 1_790_000_000;
const SILVERPINE: u32 = 1421;

fn spot(x: u32, y: u32) -> Value {
    json!({"map": SILVERPINE, "x": x, "y": y})
}

fn person(name: &str, place: &str, trust: Option<i64>, at: (u32, u32)) -> Value {
    json!({"name": name, "place": place, "first_met": DAY + 10, "trust": trust, "spot": spot(at.0, at.1)})
}

/// A Forsaken who worked through Silverpine Forest: five people at the Sepulcher, a quest,
/// a book, gossip, a death at the Dead Field, a rare at Pyrewood, and the lore of the zone.
fn silverpine() -> Value {
    json!({
        "type": "journal", "page": 0, "pages": 1,
        "places": [
            {"name": "Silverpine Forest", "kind": "zone", "first_visit": DAY, "spot": spot(400, 400)},
            {"name": "The Sepulcher", "kind": "zone", "within": "Silverpine Forest", "first_visit": DAY, "spot": spot(450, 420)},
            {"name": "The Dead Field", "kind": "zone", "within": "Silverpine Forest", "first_visit": DAY + 20, "spot": spot(300, 300)},
            {"name": "Pyrewood Village", "kind": "zone", "within": "Silverpine Forest", "first_visit": DAY + 30, "spot": spot(470, 700)},
        ],
        "people": [
            person("Dalar Dawnweaver", "The Sepulcher", Some(20), (460, 430)),
            person("High Executor Hadrec", "The Sepulcher", None, (440, 410)),
            person("Deathstalker Faerleia", "The Sepulcher", None, (455, 440)),
            person("Apothecary Renferrel", "The Sepulcher", None, (445, 425)),
            person("Karos Razok", "The Sepulcher", None, (430, 420)),
        ],
        "learned": [
            {"kind": "quest", "title": "Arugal's Folly", "npc": "Dalar Dawnweaver", "place": "Silverpine Forest", "at": DAY + 11, "excerpt": "Bring me what you find, $N."},
            {"kind": "gossip", "npc": "Deathstalker Faerleia", "place": "Silverpine Forest", "at": DAY + 12, "excerpt": "Watch the road."},
            {"kind": "book", "title": "The Fall of Lordaeron", "place": "Silverpine Forest", "at": DAY + 13, "excerpt": "The plague came north."},
        ],
        "deeds": [
            {"kind": "game_quest_done", "title": "Arugal's Folly", "at": DAY + 14, "place": "The Sepulcher"},
            {"kind": "died", "killer": "Bleak Worg", "at": DAY + 21, "place": "The Dead Field"},
            {"kind": "defeated", "foe": "Krethis Shadowspinner", "times": 1, "at": DAY + 31, "place": "Pyrewood Village"},
        ],
        "quests": [
            {"number": 7, "giver": "Dalar Dawnweaver", "title": "The Lost Lantern", "status": "accepted", "steps": []},
        ],
        "chapters": [
            {"number": 4, "first": 10, "state": "open", "began": DAY, "title": "Silverpine Forest", "zones": ["Silverpine Forest"], "people": ["Dalar Dawnweaver"], "deeds": [], "left_out": 0},
        ],
        "lore": [
            {"about": "Silverpine Forest", "text": "Arugal called the worgen into Silverpine.", "more": true},
            {"about": "The Sepulcher", "text": "The Forsaken hold the Sepulcher."},
        ],
        "histories": [
            {"zone": "Silverpine Forest", "text": "$N fought the worgen here."},
        ],
    })
}

/// A player at the Sepulcher with the book open on Knowledge, and the map of the zone.
fn at_the_sepulcher(journal: &Value) -> Game {
    let game = Game::new();
    game.run(&format!(
        "wow.units.player = {{ name = 'Ada', player = true }}
         wow.zone, wow.subzone = 'Silverpine Forest', 'The Sepulcher'
         wow.maps[{SILVERPINE}] = {{
             name = 'Silverpine Forest', type = 3,
             layers = {{ {{ layerWidth = 1002, layerHeight = 668, tileWidth = 256, tileHeight = 256 }} }},
             textures = {{ 1 }},
         }}
         wow.playerMap = {SILVERPINE}
         wow.Slash('/journal', '')"
    ));
    game.reply(&journal.to_string());
    game.run("ns.JournalFrame.Open('knowledge')");
    game
}

/// The page as text, `style: text` for each line, with each link shown as its name.
fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('knowledge')) do
             local text = line.text:gsub('|c%x%x%x%x%x%x%x%x|H[^|]*|h(.-)|h|r', '%1')
             if line.detail then text = text .. ' | ' .. line.detail end
             table.insert(out, line.style .. ': ' .. text)
         end
         return out",
    )
}

/// The link of the line whose text holds `name`.
fn link_of(game: &Game, name: &str) -> String {
    game.eval(&format!(
        "for _, line in ipairs(ns.Journal.Lines('knowledge')) do
             for _, link in ipairs(ns.JournalLinks.InText(line.text)) do
                 if line.text:find([[{name}]], 1, true) then return link end
             end
         end"
    ))
}

/// The player clicks the name in the page of the book.
fn click(game: &Game, name: &str) {
    let link = link_of(game, name);
    game.run(&format!(
        "wow.ClickLink(TimewaysJournalFrameScroll:GetScrollChild(), '{link}')"
    ));
}

fn after(lines: &[String], section: &str) -> Vec<String> {
    let start = lines
        .iter()
        .position(|line| line == &format!("section: {section}"))
        .unwrap_or_else(|| panic!("no {section} in {lines:?}"));
    lines[start + 1..]
        .iter()
        .take_while(|line| !line.starts_with("section: "))
        .cloned()
        .collect()
}

#[test]
fn the_atlas_opens_on_the_zone_where_you_stand() {
    let game = at_the_sepulcher(&silverpine());

    let lines = lines(&game);

    assert_eq!(lines[0], "heading: Silverpine Forest");
    assert_eq!(
        lines[1],
        format!(
            "note: First visit {}, in Chapter 4",
            game.eval::<String>(&format!("date('%d %b %Y', {DAY})"))
        )
    );
    assert_eq!(
        game.eval::<Option<u32>>("ns.MapPane.Shown()"),
        Some(SILVERPINE)
    );
}

#[test]
fn a_zone_counts_what_you_know_of_it_and_never_a_total() {
    let game = at_the_sepulcher(&silverpine());

    let pills: Vec<String> = game.eval("ns.Journal.Lines('knowledge')[3].pills");

    assert_eq!(pills, ["5 people", "1 quest", "3 read", "1 death"]);
}

#[test]
fn the_lore_of_a_zone_and_its_places_and_your_history_there_show_first() {
    let game = at_the_sepulcher(&silverpine());

    let lines = lines(&game);

    assert_eq!(
        after(&lines, "What you know"),
        [
            "prose: Arugal called the worgen into Silverpine.",
            "prose: The Forsaken hold the Sepulcher.",
        ]
    );
    assert_eq!(
        after(&lines, "Your history here"),
        ["prose: Ada fought the worgen here."]
    );
}

#[test]
fn people_show_how_they_feel_about_you_in_the_words_of_the_tooltip() {
    let game = at_the_sepulcher(&silverpine());

    let people = after(&lines(&game), "People");

    assert_eq!(people[0], "link: Dalar Dawnweaver | Likes you");
    assert_eq!(people[1], "link: High Executor Hadrec | Neutral");
}

#[test]
fn a_long_list_shows_three_rows_and_show_all_opens_the_rest() {
    let game = at_the_sepulcher(&silverpine());
    assert_eq!(after(&lines(&game), "People").len(), 4);

    click(&game, "Show all 5");

    assert_eq!(after(&lines(&game), "People").len(), 5);
}

#[test]
fn a_zone_lists_its_quests_reads_deaths_kills_and_chapters() {
    let game = at_the_sepulcher(&silverpine());

    let lines = lines(&game);

    assert_eq!(after(&lines, "Quests"), ["link: Arugal's Folly | Ch. 4"]);
    assert_eq!(
        after(&lines, "Read and heard"),
        [
            "link: Arugal's Folly | Dalar Dawnweaver",
            "link: Heard from Deathstalker Faerleia",
            "link: The Fall of Lordaeron",
        ]
    );
    assert_eq!(
        after(&lines, "Deaths and kills"),
        [
            "link: Killed by Bleak Worg | The Dead Field",
            "link: Defeated Krethis Shadowspinner | Ch. 4",
        ]
    );
    assert_eq!(after(&lines, "Chapters")[0], "text: Chapter 4");
}

#[test]
fn theres_more_to_learn_here_only_when_the_desktop_says_so() {
    let mut journal = silverpine();
    let with_more = at_the_sepulcher(&journal);
    journal["lore"][0]["more"] = json!(false);
    let without = at_the_sepulcher(&journal);

    let more = "help: There's more to learn here.".to_string();
    assert_eq!(lines(&with_more).last(), Some(&more));
    assert!(!lines(&without).contains(&more));
}

#[test]
fn a_click_on_a_person_opens_their_page_and_back_returns() {
    let game = at_the_sepulcher(&silverpine());

    click(&game, "Dalar Dawnweaver");
    let page = lines(&game);
    game.run("wow.Button('Back'):Click()");

    assert_eq!(page[0], "heading: Dalar Dawnweaver");
    assert_eq!(page[1], "note: The Sepulcher, Silverpine Forest");
    assert_eq!(page[2], "text: Likes you");
    assert_eq!(lines(&game)[0], "heading: Silverpine Forest");
}

#[test]
fn a_person_says_what_they_told_you_and_what_is_between_you() {
    let game = at_the_sepulcher(&silverpine());
    game.run(&format!(
        "TimewaysTalk = {{ ['Dalar Dawnweaver'] = {{
             {{ at = {DAY}, said = 'Hello.', heard = 'Greetings.' }},
             {{ at = {DAY}, said = 'News?', heard = 'None.' }},
         }} }}"
    ));

    click(&game, "Dalar Dawnweaver");

    let lines = lines(&game);
    assert_eq!(
        after(&lines, "Said to you"),
        [
            "prose: \"Bring me what you find, Ada.\"",
            "note: From the quest Arugal's Folly",
        ]
    );
    let between = after(&lines, "Between you");
    assert_eq!(between[0], "link: The Lost Lantern | In progress");
    assert!(
        between[1].starts_with("link: Talked twice | "),
        "{between:?}"
    );
}

#[test]
fn a_place_names_its_zone_and_shows_its_own_people() {
    let game = at_the_sepulcher(&silverpine());

    game.run("ns.JournalKnowledge.Show('place', 'The Sepulcher')");

    let lines = lines(&game);
    assert_eq!(lines[0], "heading: The Sepulcher");
    assert!(
        lines[1].starts_with("note: In Silverpine Forest"),
        "{lines:?}"
    );
    assert_eq!(
        after(&lines, "What you know"),
        ["prose: The Forsaken hold the Sepulcher."]
    );
    assert_eq!(after(&lines, "People").len(), 4);
}

#[test]
fn a_place_that_you_never_visited_says_so() {
    let game = at_the_sepulcher(&silverpine());

    game.run("ns.JournalKnowledge.Show('place', 'Alterac Mountains')");

    assert_eq!(
        lines(&game),
        [
            "heading: Alterac Mountains",
            "help: You haven't been here yet."
        ]
    );
}

#[test]
fn a_zone_with_nothing_in_it_says_what_fills_it() {
    let journal = json!({
        "type": "journal", "page": 0, "pages": 1,
        "places": [{"name": "Silverpine Forest", "kind": "zone", "first_visit": DAY}],
    });
    let game = at_the_sepulcher(&journal);

    assert_eq!(
        lines(&game).last().unwrap(),
        "help: Nothing yet. People you meet, books you read, and quests you finish show up here."
    );
}

#[test]
fn a_read_text_opens_with_the_name_of_your_character() {
    let game = at_the_sepulcher(&silverpine());

    game.run("ns.JournalKnowledge.Show('text', 1)");

    let lines = lines(&game);
    assert_eq!(lines[0], "heading: Arugal's Folly");
    assert!(
        lines[1].starts_with("note: From Dalar Dawnweaver · "),
        "{lines:?}"
    );
    assert_eq!(lines[2], "prose: Bring me what you find, Ada.");
}

#[test]
fn a_chapter_of_a_zone_opens_in_the_chronicle() {
    let game = at_the_sepulcher(&silverpine());

    click(&game, "Chapter 4");

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "chapters");
    assert_eq!(game.eval::<i64>("ns.Journal.Selected('chapters')"), 4);
}

#[test]
fn a_side_quest_opens_in_the_quests_tab() {
    let game = at_the_sepulcher(&silverpine());
    click(&game, "Dalar Dawnweaver");

    click(&game, "The Lost Lantern");

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "quests");
    assert_eq!(game.eval::<i64>("ns.Journal.Selected('quests')"), 7);
}

#[test]
fn the_world_page_lists_each_zone_with_its_counts() {
    let game = at_the_sepulcher(&silverpine());
    game.run(&format!(
        "wow.maps[1415] = {{ name = 'Eastern Kingdoms', type = 2 }}
         wow.maps[{SILVERPINE}].parent = 1415
         ns.JournalFrame.Refresh()"
    ));
    let up: String = game.eval("return wow.Button('< Eastern Kingdoms').text");

    game.run("wow.Button('< Eastern Kingdoms'):Click()");

    assert_eq!(up, "< Eastern Kingdoms");
    assert_eq!(
        lines(&game),
        [
            "heading: Eastern Kingdoms",
            "note: You've been to 1 place here.",
            "link: Silverpine Forest | 5 people · 1 quest",
        ]
    );
}

/// Each pin that shows on the map, as `kind title`, from its icon.
fn pins(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for n, pin in ipairs(ns.Journal.Page('knowledge').pins) do
             table.insert(out, pin.kind .. ' ' .. pin.title)
         end
         return out",
    )
}

#[test]
fn a_zone_shows_its_places_and_a_skull_where_you_died() {
    let game = at_the_sepulcher(&silverpine());

    assert_eq!(
        pins(&game),
        [
            "place The Sepulcher",
            "place The Dead Field",
            "place Pyrewood Village",
            "death Killed by Bleak Worg",
        ]
    );
}

#[test]
fn a_place_shows_its_people_and_fades_the_other_places() {
    let game = at_the_sepulcher(&silverpine());

    game.run("ns.JournalKnowledge.Show('place', 'The Sepulcher')");

    let faded: Vec<bool> =
        game.eval("local out = {} for _, pin in ipairs(ns.Journal.Page('knowledge').pins) do table.insert(out, pin.dim == true) end return out");
    assert_eq!(pins(&game).len(), 8);
    assert_eq!(&faded[..3], [false, true, true]);
}

#[test]
fn a_pin_opens_its_page_and_its_tooltip_gives_the_counts() {
    let game = at_the_sepulcher(&silverpine());
    let pin = "for _, widget in ipairs(wow.widgets) do
                   if widget.kind == 'Button' and widget.shown and widget.scripts.OnEnter
                       and widget.parent and widget.parent.parent == TimewaysJournalFrame then
                       return widget
                   end
               end";

    game.run(&format!(
        "local pin = (function() {pin} end)(); pin.scripts.OnEnter(pin)"
    ));
    let tooltip: Vec<String> = game.eval("wow.tooltip.lines");
    game.run(&format!(
        "local pin = (function() {pin} end)(); pin:Click()"
    ));

    assert_eq!(tooltip, ["The Sepulcher", "5 people · 1 quest"]);
    assert_eq!(lines(&game)[0], "heading: The Sepulcher");
}

#[test]
fn a_press_on_a_link_of_a_page_that_you_can_edit_opens_the_link_not_the_editor() {
    let game = at_the_sepulcher(&silverpine());
    game.run("ns.JournalFrame.Open('chapters')");
    let page = "TimewaysJournalFrameScroll:GetScrollChild()";

    game.run(&format!(
        "local page = {page}
         page.scripts.OnHyperlinkEnter(page)
         wow.MouseDown(page)"
    ));

    assert!(!game.eval::<bool>("ns.Editor.IsShown()"));
}

/// The lines of the open chapter, each link shown as its name.
fn chapter_lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('chapters')) do
             table.insert(out, (line.text:gsub('|c%x%x%x%x%x%x%x%x|H[^|]*|h(.-)|h|r', '%1')))
         end
         return out",
    )
}

#[test]
fn a_name_in_a_chapter_opens_its_page_in_knowledge() {
    let game = at_the_sepulcher(&silverpine());
    game.run("ns.JournalFrame.Open('chapters')");
    let link: String = game.eval(
        "for _, line in ipairs(ns.Journal.Lines('chapters')) do
             local links = ns.JournalLinks.InText(line.text)
             if line.text:find('People', 1, true) then return links[1] end
         end",
    );

    game.run(&format!(
        "wow.ClickLink(TimewaysJournalFrameScroll:GetScrollChild(), '{link}')"
    ));

    assert_eq!(
        game.eval::<String>("ns.JournalFrame.Section()"),
        "knowledge"
    );
    assert_eq!(lines(&game)[0], "heading: Dalar Dawnweaver");
}

#[test]
fn a_chapter_names_the_people_you_talked_to_while_it_was_open() {
    let game = at_the_sepulcher(&silverpine());
    game.run(&format!(
        "TimewaysTalk = {{
             ['Dalar Dawnweaver'] = {{ {{ at = {DAY} + 5, said = 'Hello.', heard = 'Greetings.' }} }},
             ['Old Friend'] = {{ {{ at = {DAY} - 5, said = 'Hello.', heard = 'Long time.' }} }},
         }}"
    ));

    let lines = chapter_lines(&game);

    assert!(
        lines.contains(&"Talked to: Dalar Dawnweaver.".to_string()),
        "{lines:?}"
    );
}

#[test]
fn a_chapter_shows_what_it_found_on_its_map() {
    let game = at_the_sepulcher(&silverpine());

    let kinds: Vec<String> = game.eval(
        "local out = {}
         for _, pin in ipairs(ns.Journal.Page('chapters').pins) do table.insert(out, pin.kind) end
         return out",
    );

    assert_eq!(kinds.iter().filter(|kind| *kind == "place").count(), 3);
    assert_eq!(kinds.iter().filter(|kind| *kind == "person").count(), 5);
}

#[test]
fn the_contents_of_the_chronicle_float_over_the_pins_of_its_map() {
    let game = at_the_sepulcher(&silverpine());
    game.run("ns.JournalFrame.Open('chapters')");

    let levels: Vec<i64> = game.eval(
        "local pin
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.shown and widget.scripts.OnEnter
                 and widget.parent and widget.parent.parent == TimewaysJournalFrame then
                 pin = widget
             end
         end
         return { pin:GetFrameLevel(), TimewaysJournalListScroll.parent:GetFrameLevel() }",
    );

    assert!(levels[1] > levels[0], "{levels:?}");
}
