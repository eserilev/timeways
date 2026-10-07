//! The map on the left of the journal: the game's own art of a zone (GAMEPLAY.md 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

const ELWYNN: u32 = 1429;
const TIRISFAL: u32 = 1420;

/// A zone map of the classic size: 1002 by 668, in 4 columns and 3 rows of 256 tiles. The
/// file of each tile is the map id times 100, plus its number.
fn add_map(game: &Game, id: u32, name: &str) {
    game.run(&format!(
        "local textures = {{}}
         for n = 1, 12 do textures[n] = {id} * 100 + n end
         wow.maps[{id}] = {{
             name = '{name}',
             layers = {{ {{ layerWidth = 1002, layerHeight = 668, tileWidth = 256, tileHeight = 256 }} }},
             textures = textures,
         }}"
    ));
}

/// A player in Elwynn Forest, in a world that also has Tirisfal Glades.
fn in_elwynn() -> Game {
    let game = Game::new();
    add_map(&game, ELWYNN, "Elwynn Forest");
    add_map(&game, TIRISFAL, "Tirisfal Glades");
    game.run(&format!("wow.playerMap = {ELWYNN}"));
    game
}

fn chapter_in(zone: &str) -> String {
    format!(
        r#"{{"type":"journal","page":0,"pages":1,"chapters":[{{"number":1,"began":1790000000,"zones":["{zone}"],"people":[],"deeds":[],"left_out":0}}]}}"#
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.001
}

fn shown_map(game: &Game) -> Option<u32> {
    game.eval("ns.MapPane.Shown()")
}

/// The file of each tile that shows, in the order of creation.
fn tiles(game: &Game) -> Vec<u32> {
    game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Texture' and type(widget.file) == 'number' and widget.shown then
                 table.insert(out, widget.file)
             end
         end
         return out",
    )
}

fn path(game: &Game) -> String {
    game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and (widget.text or ''):find('^Journal') then
                 return widget.text
             end
         end",
    )
}

/// The pin, as { shown, x, y }: its offsets from the top left of the pane.
fn pin(game: &Game) -> (bool, f64, f64) {
    let shown: bool = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.atlas == 'Waypoint-MapPin-Tracked' then return widget.shown end
         end",
    );
    let point: Vec<f64> = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.atlas == 'Waypoint-MapPin-Tracked' then
                 return widget.point and { widget.point[4], widget.point[5] } or { 0, 0 }
             end
         end",
    );
    (shown, point[0], point[1])
}

#[test]
fn the_map_shows_the_zone_of_the_player_by_default() {
    let game = in_elwynn();

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert_eq!(shown_map(&game), Some(ELWYNN));
    assert_eq!(path(&game), "Journal  >  Knowledge  >  Elwynn Forest");
}

#[test]
fn the_map_draws_every_tile_of_the_art_in_rows() {
    let game = in_elwynn();

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    let expected: Vec<u32> = (1..=12).map(|n| ELWYNN * 100 + n).collect();
    assert_eq!(tiles(&game), expected);
}

#[test]
fn the_art_covers_the_pane_and_keeps_its_shape() {
    let game = in_elwynn();

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    // Each tile as { x, y, width, height }, and the pane as { width, height }.
    let tiles: Vec<Vec<f64>> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Texture' and type(widget.file) == 'number' then
                 table.insert(out, { widget.point[4], widget.point[5], widget.width, widget.height })
             end
         end
         return out",
    );
    let pane: Vec<f64> = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Texture' and type(widget.file) == 'number' then
                 return { widget.parent.width, widget.parent.height }
             end
         end",
    );
    let [left, top, size, height] = tiles[0][..] else {
        panic!("{tiles:?}")
    };
    let scale = size / 256.0;
    assert!(close(size, height), "{tiles:?}");
    assert!(left <= 0.0 && top >= 0.0, "{tiles:?}");
    assert!(
        left + 1002.0 * scale >= pane[0] - 0.001,
        "{tiles:?} {pane:?}"
    );
    assert!(
        -top + 668.0 * scale >= pane[1] - 0.001,
        "{tiles:?} {pane:?}"
    );
    assert!(close(tiles[5][0], left + size), "{tiles:?}");
    assert!(close(tiles[5][1], top - size), "{tiles:?}");
}

#[test]
fn a_chapter_shows_the_map_of_its_zone() {
    let game = in_elwynn();
    game.run("wow.Slash('/journal', '')");

    game.reply(&chapter_in("Tirisfal Glades"));
    game.run("ns.JournalFrame.Open('chapters')");

    assert_eq!(shown_map(&game), Some(TIRISFAL));
}

#[test]
fn a_zone_with_no_map_shows_the_map_of_the_player() {
    let game = in_elwynn();
    game.run("wow.Slash('/journal', '')");

    game.reply(&chapter_in("A place of no map"));
    game.run("ns.JournalFrame.Open('chapters')");

    assert_eq!(shown_map(&game), Some(ELWYNN));
}

#[test]
fn with_no_map_at_all_the_pane_says_so() {
    let game = Game::new();

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert_eq!(shown_map(&game), None);
    assert!(tiles(&game).is_empty());
    let shown: Vec<String> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.shown and widget.text then
                 table.insert(out, widget.text)
             end
         end
         return out",
    );
    assert!(
        shown.contains(&"No map for this place.".to_string()),
        "{shown:?}"
    );
    assert_eq!(path(&game), "Journal  >  Knowledge");
}

#[test]
fn a_map_whose_art_has_no_size_counts_as_no_map() {
    let game = in_elwynn();
    game.run(&format!("wow.maps[{ELWYNN}].layers[1].tileWidth = 0"));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert_eq!(shown_map(&game), None);
}

#[test]
fn a_map_that_leaves_for_a_map_of_no_art_hides_its_tiles() {
    let game = in_elwynn();
    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    game.run(&format!("wow.maps[{ELWYNN}].layers = nil"));
    game.run("ns.JournalFrame.Open('knowledge')");

    assert!(tiles(&game).is_empty());
}

#[test]
fn the_pin_marks_where_the_player_stands() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].player = {{ x = 0.5, y = 0.25 }}"
    ));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    let (shown, x, y) = pin(&game);
    let first_tile: Vec<f64> = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Texture' and type(widget.file) == 'number' then
                 return { widget.point[4], widget.point[5], widget.width }
             end
         end",
    );
    let scale = first_tile[2] / 256.0;
    assert!(shown);
    assert!(close(x, first_tile[0] + 0.5 * 1002.0 * scale), "{x}");
    assert!(close(y, first_tile[1] - 0.25 * 668.0 * scale), "{y}");
}

#[test]
fn the_pin_hides_on_a_map_where_the_player_is_not() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].player = {{ x = 0.5, y = 0.25 }}"
    ));
    game.run("wow.Slash('/journal', '')");

    game.reply(&chapter_in("Tirisfal Glades"));
    game.run("ns.JournalFrame.Open('chapters')");

    assert!(!pin(&game).0);
}

#[test]
fn a_hidden_position_hides_the_pin() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].player = {{ x = 0.5, y = 0.25 }}; wow.secrets[0.5] = true"
    ));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert!(!pin(&game).0);
}

#[test]
fn a_map_that_comes_after_an_empty_tree_is_found() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    add_map(&game, TIRISFAL, "Tirisfal Glades");

    game.reply(&chapter_in("Tirisfal Glades"));
    game.run("ns.JournalFrame.Open('chapters')");

    assert_eq!(shown_map(&game), Some(TIRISFAL));
}

/// Each explored part that shows, as { file, x, y, width, height }, in the units of the
/// pane. The files of the parts run from 9000 to 9999.
fn explored(game: &Game) -> Vec<Vec<f64>> {
    game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Texture' and type(widget.file) == 'number' and widget.file >= 9000 and widget.file < 10000 and widget.shown then
                 table.insert(out, { widget.file, widget.point[4], widget.point[5], widget.width, widget.height })
             end
         end
         return out",
    )
}

/// The corner and the scale of the art, from its first tile.
fn art(game: &Game) -> (f64, f64, f64) {
    let tile: Vec<f64> = game.eval(&format!(
        "for _, widget in ipairs(wow.widgets) do
             if widget.file == {ELWYNN} * 100 + 1 then
                 return {{ widget.point[4], widget.point[5], widget.width }}
             end
         end"
    ));
    (tile[0], tile[1], tile[2] / 256.0)
}

#[test]
fn the_explored_parts_of_a_zone_draw_over_its_art() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].explored = {{
             {{ textureWidth = 300, textureHeight = 200, offsetX = 100, offsetY = 50,
                fileDataIDs = {{ 9001, 9002 }}, isShownByMouseOver = false }},
         }}"
    ));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    let (left, top, scale) = art(&game);
    let parts = explored(&game);
    assert_eq!(parts.len(), 2, "{parts:?}");
    let [file, x, y, width, height] = parts[0][..] else {
        panic!("{parts:?}")
    };
    assert!(close(file, 9001.0));
    assert!(close(x, left + 100.0 * scale), "{parts:?}");
    assert!(close(y, top - 50.0 * scale), "{parts:?}");
    assert!(close(width, 256.0 * scale) && close(height, 200.0 * scale));
    assert!(close(parts[1][1], left + 356.0 * scale), "{parts:?}");
    assert!(close(parts[1][3], 44.0 * scale), "{parts:?}");
}

#[test]
fn a_part_that_shows_only_under_the_mouse_stays_hidden() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].explored = {{
             {{ textureWidth = 100, textureHeight = 100, offsetX = 0, offsetY = 0,
                fileDataIDs = {{ 9001 }}, isShownByMouseOver = true }},
         }}"
    ));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert!(explored(&game).is_empty());
}

#[test]
fn a_broken_part_draws_nothing() {
    let game = in_elwynn();
    game.run(&format!(
        "wow.maps[{ELWYNN}].explored = {{ 7, {{ textureWidth = 0, textureHeight = 100, offsetX = 0, offsetY = 0, fileDataIDs = {{ 9001 }} }} }}"
    ));

    game.run("wow.Slash('/journal', ''); ns.JournalFrame.Open('knowledge')");

    assert!(explored(&game).is_empty());
}

fn visited_line(game: &Game) -> Option<String> {
    game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.shown and (widget.text or ''):find('^Visited') then
                 return widget.text
             end
         end",
    )
}

#[test]
fn the_map_names_the_subzones_of_its_zone_that_the_player_visited() {
    let game = in_elwynn();
    game.run("wow.Slash('/journal', '')");

    game.reply(concat!(
        r#"{"type":"journal","page":0,"pages":1,"places":["#,
        r#"{"name":"Elwynn Forest","first_visit":1790000000},"#,
        r#"{"name":"Goldshire","within":"Elwynn Forest","first_visit":1790000000},"#,
        r#"{"name":"Brill","within":"Tirisfal Glades","first_visit":1790000000},"#,
        r#"{"name":"Northshire Valley","within":"Elwynn Forest","first_visit":1790000001}]}"#,
    ));
    game.run("ns.JournalFrame.Open('knowledge')");

    assert_eq!(
        visited_line(&game).as_deref(),
        Some("Visited: Goldshire, Northshire Valley")
    );
}

#[test]
fn a_zone_with_no_visited_subzone_shows_no_list() {
    let game = in_elwynn();
    game.run("wow.Slash('/journal', '')");

    game.reply(r#"{"type":"journal","page":0,"pages":1,"places":[{"name":"Elwynn Forest","first_visit":1790000000}]}"#);
    game.run("ns.JournalFrame.Open('knowledge')");

    assert_eq!(visited_line(&game), None);
}
