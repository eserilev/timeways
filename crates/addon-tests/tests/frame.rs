//! The window of the journal: the tabs, the path, the list, the parchment, and the buttons
//! (GAMEPLAY.md 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    clippy::float_cmp,
    reason = "the offsets of the frame are sums of whole numbers, so they are exact"
)]

mod common;

use common::Game;

/// A journal with a long task and a long note, so each line of the page wraps.
const LONG: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"#,
    r#""quests":[{"number":1,"giver":"Keeper Tessa","title":"A task with a very long title that goes on and on","status":"offered","#,
    r#""text":"Bram worked the old mill with his daughter before the plague, and he still goes to the pond every night.","steps":[]}],"#,
    r#""hero":{"sheet":[],"entries":[{"number":1,"at":1790000000,"text":"A note of the player that runs long enough to wrap twice on the page."}]}}"#,
);

fn open(section: &str) -> Game {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(LONG);
    game.run(&format!("ns.JournalFrame.Open('{section}')"));
    game
}

fn frame_width(game: &Game) -> f64 {
    game.eval("TimewaysJournalFrame.width")
}

/// Each tab as { x, y, width, label width }, in the order of the sections.
fn tabs(game: &Game) -> Vec<Vec<f64>> {
    game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysJournalFrame
                 and widget.template == 'UIPanelButtonTemplate' then
                 table.insert(out, { widget.point[4], widget.point[5], widget.width,
                     widget:GetStringWidth() })
             end
         end
         return out",
    )
}

#[test]
fn the_tabs_stand_in_one_row_inside_the_frame_and_fit_their_labels() {
    let game = open("quests");

    let tabs = tabs(&game);

    assert_eq!(tabs.len(), 6);
    let right_edge = frame_width(&game) - 12.0;
    let mut end = 12.0;
    for tab in &tabs {
        let [x, y, width, label] = tab[..] else {
            panic!("{tab:?}")
        };
        assert_eq!(y, tabs[0][1], "{tabs:?}");
        assert!(x >= end, "{tabs:?}");
        assert!(width >= label + 6.0, "{tab:?}");
        end = x + width;
    }
    assert!(end <= right_edge + 0.001, "{tabs:?}");
}

#[test]
fn only_the_tab_of_the_open_section_is_marked() {
    let game = open("quests");

    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.text == 'Chronicle' then widget:Click() end
         end",
    );

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "chapters");
    let enabled: Vec<bool> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysJournalFrame
                 and widget.template == 'UIPanelButtonTemplate' then
                 table.insert(out, widget:IsEnabled())
             end
         end
         return out",
    );
    assert_eq!(enabled, [true, false, true, true, true, true]);
}

/// Each line of the page that shows, as { left, width }, and each of its buttons as
/// { right, width }, in offsets from the left of the scroll frame.
fn page_bounds(game: &Game) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let texts = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.parent == TimewaysJournalFrameScroll:GetScrollChild() and widget.shown
                 and widget.kind == 'FontString' then
                 table.insert(out, { widget.point[4], widget.width })
             end
         end
         return out",
    );
    let buttons = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.parent == TimewaysJournalFrameScroll:GetScrollChild() and widget.shown
                 and widget.kind == 'Button' then
                 table.insert(out, { widget.point[4], widget.width })
             end
         end
         return out",
    );
    (texts, buttons)
}

fn assert_inside_the_parchment(game: &Game) {
    let scroll: f64 = game.eval("TimewaysJournalFrameScroll.width");
    let (texts, buttons) = page_bounds(game);
    assert!(!texts.is_empty());
    for text in texts {
        assert!(
            text[0] >= 0.0 && text[0] + text[1] <= scroll,
            "{text:?} in {scroll}"
        );
    }
    for button in buttons {
        assert!(
            button[0] - button[1] >= 0.0 && button[0] <= scroll,
            "{button:?} in {scroll}"
        );
    }
}

#[test]
fn each_line_of_a_task_stays_inside_the_parchment() {
    let game = open("quests");

    assert_inside_the_parchment(&game);
}

#[test]
fn each_line_of_the_hero_stays_inside_the_parchment() {
    let game = open("hero");

    assert_inside_the_parchment(&game);
}

#[test]
fn the_map_and_the_parchment_share_the_frame_side_by_side() {
    let game = open("quests");

    // The map pane is the parent of the list box, and the parchment holds the scroll frame.
    let map: Vec<f64> = game.eval(
        "local box = TimewaysJournalListScroll.parent
         local pane = box.parent
         return { pane.point[4], pane.width }",
    );
    let parchment: Vec<f64> = game.eval(
        "local detail = TimewaysJournalFrameScroll.parent
         return { detail.point[4], detail.width }",
    );
    let scroll: f64 = game.eval("TimewaysJournalFrameScroll.width");
    assert!(map[0] >= 12.0, "{map:?}");
    assert!(map[0] + map[1] < parchment[0], "{map:?} {parchment:?}");
    assert!(
        parchment[0] + parchment[1] <= frame_width(&game) - 12.0,
        "{parchment:?}"
    );
    // The scroll bar of the template stands right of the scroll frame, on the parchment.
    assert!(scroll + 26.0 <= parchment[1], "{scroll} {parchment:?}");
}

/// Two stories that wait, so the Stories page has a list and three buttons.
const WAITING: &str = "TimewaysStories = { waiting = {
     { id = 'b1', author = 'Bram-Stormrage', title = 'One', text = 'Hi.', at = 1 },
     { id = 'c1', author = 'Cora-Stormrage', title = 'Two', text = 'Hi.', at = 1 } } }";

#[test]
fn the_buttons_at_the_bottom_stand_inside_the_frame() {
    let game = Game::new();
    game.run(WAITING);
    game.run("ns.JournalFrame.Open('stories')");

    // Each button as { offset of its right edge from the right of the footer, width }.
    let buttons: Vec<Vec<f64>> = game.eval(
        "local out = {}
         for _, label in ipairs({ 'Decline', 'Accept' }) do
             local button = wow.Button(label)
             table.insert(out, { button.point[4], button.width, button.parent.width })
         end
         return out",
    );
    let footer = buttons[0][2];
    assert!(footer <= frame_width(&game) - 24.0);
    let [previous, next] = [&buttons[0], &buttons[1]];
    assert!(next[0] <= 0.0, "{buttons:?}");
    assert!(previous[0] <= next[0] - next[1], "{buttons:?}");
    assert!(previous[0] - previous[1] >= -footer, "{buttons:?}");
}

#[test]
fn the_list_floats_over_the_map_and_fills_the_left_half_for_the_stories() {
    let game = open("quests");
    game.run(WAITING);
    let floating: Vec<f64> = game.eval(
        "local box = TimewaysJournalListScroll.parent
         return { box.width, box.parent.width }",
    );

    game.run("ns.JournalFrame.Open('stories')");

    let sheet: Vec<f64> = game.eval(
        "local box = TimewaysJournalListScroll.parent
         return { box.width, box.parent.width }",
    );
    assert!(floating[0] < floating[1] / 2.0, "{floating:?}");
    assert_eq!(sheet[0], sheet[1]);
}

#[test]
fn the_list_marks_the_open_item() {
    let game = Game::new();
    game.run(WAITING);
    game.run("ns.JournalFrame.Open('stories')");

    game.run("ns.JournalFrame.Select('w:Bram-Stormrage:b1')");

    // The shade of each row of the list, in order.
    let marked: Vec<bool> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.shade then
                 table.insert(out, widget.shade.shown)
             end
         end
         return out",
    );
    // The newest story first.
    assert_eq!(marked, [false, true]);
}

#[test]
fn a_list_with_no_rows_hides_its_box() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(r#"{"type":"journal","page":0,"pages":1}"#);

    game.run("ns.JournalFrame.Open('deeds')");

    assert!(!game.eval::<bool>("TimewaysJournalListScroll.parent:IsShown()"));
}

#[test]
fn the_path_names_the_section_and_the_open_chapter() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"chapters":[{"number":2,"began":1790000000,"zones":["Brill"],"people":[],"deeds":[],"left_out":0}]}"#,
    );

    game.run("ns.JournalFrame.Open('chapters')");

    let shown: Vec<String> = game.eval("wow.ShownTexts(TimewaysJournalFrame)");
    assert!(
        shown.contains(&"Journal  >  Chronicle  >  Chapter 2".to_string()),
        "{shown:?}"
    );
}

#[test]
fn the_editor_takes_the_place_of_the_page_the_list_the_tabs_and_the_buttons() {
    let game = open("hero");

    game.run("ns.Hero.Write()");

    let shown: Vec<bool> = game.eval(
        "return {
             TimewaysJournalFrameScroll:IsShown(),
             TimewaysJournalCardsScroll.parent:IsShown(),
             wow.Button('Hero'):IsShown(),
             wow.Button('Your Story'):IsShown() and TimewaysJournalCardsScroll.parent:IsShown(),
             ns.Editor.IsShown(),
         }",
    );
    assert_eq!(shown, [false, false, false, false, true]);
}

#[test]
fn the_book_comes_back_whole_when_the_player_cancels() {
    let game = open("hero");
    game.run("ns.Hero.Write()");

    game.run("wow.Button('Cancel'):Click()");

    let shown: Vec<bool> = game.eval(
        "return {
             TimewaysJournalFrameScroll:IsShown(),
             TimewaysJournalCardsScroll.parent:IsShown(),
             wow.Button('Hero'):IsShown(),
             wow.Button('Your Story'):IsShown(),
         }",
    );
    assert_eq!(shown, [true, true, true, true]);
}

#[test]
fn the_editor_writes_on_the_parchment() {
    let game = open("hero");

    game.run("ns.Hero.Write()");

    let inside: bool =
        game.eval("return TimewaysEditorScroll.parent.parent == TimewaysJournalFrameScroll.parent");
    assert!(inside);
}
