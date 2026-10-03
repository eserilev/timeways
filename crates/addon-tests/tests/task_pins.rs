//! The pins of the open task on the journal map (GAMEPLAY.md 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use serde_json::json;
use timeways_story::input::MessageId;
use timeways_story::journal::{Journal, Person, Place, pages};
use timeways_story::places::PlaceKind;
use timeways_story::quest::{Status, Step, Tracked};
use timeways_story::spot::Spot;
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;
const ELWYNN: u32 = 1429;
const TIRISFAL: u32 = 1420;

fn spot(map: u32, x: i64, y: i64) -> Spot {
    serde_json::from_value(json!({"map": map, "x": x, "y": y})).unwrap()
}

fn place(name: &str, within: Option<&str>, spot: Option<Spot>) -> Place {
    Place {
        name: name.to_string(),
        kind: PlaceKind::Zone,
        within: within.map(str::to_string),
        first_visit: Tick(DAY),
        spot,
    }
}

fn person(name: &str, place: &str, spot: Option<Spot>) -> Person {
    Person {
        name: name.to_string(),
        place: Some(place.to_string()),
        first_met: Tick(DAY),
        trust: None,
        slapped: None,
        spot,
        trust_why: None,
    }
}

/// Keeper Tessa asks you to visit Mill Pond, then to meet Farmer Bram.
fn lantern(status: Status, steps_done: usize) -> Tracked {
    Tracked {
        number: 1,
        offered_at: Tick(DAY),
        giver: "Keeper Tessa".to_string(),
        title: "The Lost Lantern".to_string(),
        text: "Find the lantern.".to_string(),
        steps: vec![
            Step::Visit {
                place: "Mill Pond".to_string(),
            },
            Step::Meet {
                npc: "Farmer Bram".to_string(),
            },
        ],
        steps_done,
        kills: 0,
        status,
        done_at: None,
    }
}

/// Everyone and every place of the task in Tirisfal Glades, where the journal knows them.
fn journal(quest: Tracked, bram: Option<Spot>) -> Journal {
    Journal {
        places: vec![
            place("Tirisfal Glades", None, Some(spot(TIRISFAL, 100, 100))),
            place(
                "Mill Pond",
                Some("Tirisfal Glades"),
                Some(spot(TIRISFAL, 300, 400)),
            ),
        ],
        people: vec![
            person(
                "Keeper Tessa",
                "Tirisfal Glades",
                Some(spot(TIRISFAL, 500, 500)),
            ),
            person("Farmer Bram", "Mill Pond", bram),
        ],
        quests: vec![quest],
        ..Journal::default()
    }
}

fn reply(journal: Journal) -> String {
    let page = pages(journal).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
        notice: None,
    })
    .unwrap()
}

fn add_map(game: &Game, id: u32, name: &str) {
    game.run(&format!(
        "wow.maps[{id}] = {{
             name = '{name}',
             layers = {{ {{ layerWidth = 1002, layerHeight = 668, tileWidth = 256, tileHeight = 256 }} }},
             textures = {{ {id} * 100 + 1 }},
         }}"
    ));
}

/// A player in Elwynn Forest with the Tasks page open on this journal.
fn tasks_page(journal: Journal) -> Game {
    let game = Game::new();
    add_map(&game, ELWYNN, "Elwynn Forest");
    add_map(&game, TIRISFAL, "Tirisfal Glades");
    game.run(&format!("wow.playerMap = {ELWYNN}"));
    game.reply(&reply(journal));
    game.run("ns.JournalFrame.Open('quests')");
    game
}

/// Each pin that shows, as `kind number alpha x y`, with x and y from the top left of the
/// art in thousandths of its size. The first tile of the art gives its corner and its scale.
fn pins(game: &Game) -> Vec<String> {
    game.eval(
        "local art
         for _, widget in ipairs(wow.widgets) do
             if widget.file == ns.MapPane.Shown() * 100 + 1 then
                 local scale = widget.width / 256
                 art = { left = widget.point[4], top = -widget.point[5], width = 1002 * scale, height = 668 * scale }
             end
         end
         local out = {}
         for _, widget in ipairs(wow.widgets) do
             local giver = type(widget.file) == 'string' and widget.file:find('QuestIcon$')
             local step = widget.atlas == 'Waypoint-MapPin-Untracked'
             if widget.shown and (giver or step) then
                 local number = ''
                 for _, other in ipairs(wow.widgets) do
                     if other.kind == 'FontString' and other.point and other.point[2] == widget and other.shown then
                         number = other.text
                     end
                 end
                 local x = math.floor((widget.point[4] - art.left) / art.width * 1000 + 0.5)
                 local y = math.floor((-widget.point[5] - art.top) / art.height * 1000 + 0.5)
                 local kind = giver and widget.file:match('(%a+)QuestIcon$') or 'step'
                 table.insert(out, string.format('%s %s %s %d %d', kind, number, widget.alpha, x, y))
             end
         end
         return out",
    )
}

#[test]
fn the_tasks_page_shows_the_map_where_the_giver_stands() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 0),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    let shown: Option<u32> = game.eval("ns.MapPane.Shown()");

    assert_eq!(shown, Some(TIRISFAL));
}

#[test]
fn each_step_gets_a_numbered_pin_at_its_place_or_its_person() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 0),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    let expected = ["Active  1 500 500", "step 1 1 300 400", "step 2 1 700 200"];
    assert_eq!(pins(&game), expected);
}

#[test]
fn a_done_step_fades() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 1),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    let expected = [
        "Active  1 500 500",
        "step 1 0.4 300 400",
        "step 2 1 700 200",
    ];
    assert_eq!(pins(&game), expected);
}

#[test]
fn the_giver_of_an_offer_gets_the_mark_of_a_new_quest() {
    let game = tasks_page(journal(
        lantern(Status::Offered, 0),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    assert_eq!(pins(&game)[0], "Available  1 500 500");
}

#[test]
fn a_step_with_no_position_gets_no_pin() {
    let game = tasks_page(journal(lantern(Status::Accepted, 0), None));

    assert_eq!(pins(&game), ["Active  1 500 500", "step 1 1 300 400"]);
}

#[test]
fn a_pin_on_another_map_stays_off_the_map() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 0),
        Some(spot(ELWYNN, 700, 200)),
    ));

    assert_eq!(pins(&game), ["Active  1 500 500", "step 1 1 300 400"]);
}

#[test]
fn with_no_position_a_task_shows_the_zone_of_its_giver_with_no_pins() {
    let mut journal = journal(lantern(Status::Accepted, 0), None);
    for place in &mut journal.places {
        place.spot = None;
    }
    for person in &mut journal.people {
        person.spot = None;
    }

    let game = tasks_page(journal);

    let shown: Option<u32> = game.eval("ns.MapPane.Shown()");
    assert_eq!(shown, Some(TIRISFAL));
    assert!(pins(&game).is_empty());
}

#[test]
fn another_page_shows_no_task_pins() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 0),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    game.run("ns.JournalFrame.Open('deeds')");

    assert!(pins(&game).is_empty());
}

#[test]
fn a_broken_position_from_the_desktop_gets_no_pin() {
    let game = tasks_page(Journal::default());
    add_map(&game, TIRISFAL, "Tirisfal Glades");

    game.reply(concat!(
        r#"{"type":"journal","page":0,"pages":1,"#,
        r#""people":[{"name":"Keeper Tessa","spot":{"map":1420,"x":1001,"y":5}},"#,
        r#"{"name":"Farmer Bram","spot":{"map":"x","x":1,"y":5}}],"#,
        r#""places":[{"name":"Mill Pond","spot":[1420,1,1]}],"#,
        r#""quests":[{"number":1,"giver":"Keeper Tessa","title":"T","status":"accepted","#,
        r#""steps":[{"goal":"visit","place":"Mill Pond"},{"goal":"meet","npc":"Farmer Bram"}]}]}"#,
    ));
    game.run("ns.JournalFrame.Open('quests')");

    assert!(pins(&game).is_empty());
}

#[test]
fn every_pin_draws_over_the_band_of_visited_places() {
    let game = tasks_page(journal(
        lantern(Status::Accepted, 0),
        Some(spot(TIRISFAL, 700, 200)),
    ));

    let under: Vec<String> = game.eval(
        "local band
         for _, widget in ipairs(wow.widgets) do
             if widget.color and widget.point and widget.point[1] == 'BOTTOMLEFT' then band = widget end
         end
         local out = {}
         for _, widget in ipairs(wow.widgets) do
             local pin = widget.atlas == 'Waypoint-MapPin-Untracked' or (type(widget.file) == 'string' and widget.file:find('QuestIcon$'))
             if pin and wow.DrawOrder(widget) <= wow.DrawOrder(band) then
                 table.insert(out, widget.atlas or widget.file)
             end
         end
         return out",
    );

    assert!(under.is_empty(), "{under:?}");
}
