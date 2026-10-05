//! Past talks (GAMEPLAY.md 3.5): what you said to each NPC and what it answered, kept in
//! the saved variables of the character, and shown lighter in the talk window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::MessageId;
use timeways_story::story::Output;

const NOW: u64 = 1_790_000_000;
const DAY: u64 = 86_400;
const FARLEY: &str = "Innkeeper Farley";

fn talk_to(game: &Game, npc: &str, words: &str) {
    game.run(&format!(
        "wow.units.target = {{ name = '{npc}' }}; wow.Slash('/talk', '{words}')"
    ));
}

fn answer(game: &Game, npc: &str, text: Option<&str>) {
    let line = serde_json::to_string(&Output::TalkAnswer {
        id: MessageId(1),
        npc: npc.to_string(),
        text: text.map(str::to_string),
        notice: None,
    })
    .unwrap();
    game.reply(&line);
}

fn page(game: &Game) -> Vec<String> {
    game.eval("wow.ShownTexts(TimewaysTalkFrameScroll:GetScrollChild())")
}

/// The saved exchanges of an NPC as `at said | heard`.
fn saved(game: &Game, npc: &str) -> Vec<String> {
    game.eval(&format!(
        "local out = {{}}
         for _, exchange in ipairs(ns.TalkHistory.Of('{npc}')) do
             table.insert(out, string.format('%d %s | %s', exchange.at, exchange.said, exchange.heard))
         end
         return out"
    ))
}

/// The saved variable as another addon or a reload left it.
fn load(game: &Game, lua: &str) {
    game.run(&format!("TimewaysTalk = {lua}"));
}

/// The value of the saved variable, after the addon read it.
fn kept_count(game: &Game) -> u32 {
    game.eval(
        "ns.TalkHistory.Of('')
         local count = 0
         for _, list in pairs(TimewaysTalk) do count = count + #list end
         return count",
    )
}

#[test]
fn an_answer_is_saved_with_the_words_and_the_time() {
    let game = Game::new();
    talk_to(&game, FARLEY, "Any news?");

    answer(&game, FARLEY, Some("Nothing but rain."));

    assert_eq!(
        saved(&game, FARLEY),
        [format!("{NOW} Any news? | Nothing but rain.")]
    );
}

#[test]
fn an_answer_for_a_closed_window_is_saved_too() {
    let game = Game::new();
    talk_to(&game, FARLEY, "Any news?");
    game.run("TimewaysTalkFrame:Hide()");

    answer(&game, FARLEY, Some("Nothing but rain."));

    assert_eq!(saved(&game, FARLEY).len(), 1);
}

#[test]
fn an_npc_that_says_nothing_saves_nothing() {
    let game = Game::new();
    talk_to(&game, FARLEY, "Any news?");

    answer(&game, FARLEY, None);

    assert!(saved(&game, FARLEY).is_empty());
}

#[test]
fn an_exchange_holds_only_the_words_the_answer_and_the_time() {
    let game = Game::new();
    talk_to(&game, FARLEY, "Any news?");

    answer(&game, FARLEY, Some("Nothing but rain."));

    let keys: Vec<String> = game.eval(
        "local keys = {}
         for key in pairs(TimewaysTalk['Innkeeper Farley'][1]) do table.insert(keys, key) end
         table.sort(keys)
         return keys",
    );
    assert_eq!(keys, ["at", "heard", "said"]);
}

#[test]
fn past_talks_show_lighter_above_the_new_one_with_a_day_line() {
    let game = Game::new();
    load(
        &game,
        &format!(
            "{{ ['{FARLEY}'] = {{
                 {{ at = {}, said = 'Hello.', heard = 'Welcome, stranger.' }},
                 {{ at = {}, said = 'Any work?', heard = 'Not today.' }},
             }} }}",
            NOW - 3 * DAY,
            NOW - DAY
        ),
    );

    talk_to(&game, FARLEY, "Any news?");

    assert_eq!(
        page(&game),
        [
            "18 Sep 2026",
            "You: Hello.",
            "Welcome, stranger.",
            "Yesterday",
            "You: Any work?",
            "Not today.",
            "You: Any news?",
            "Thinking...",
        ]
    );
    let past: Vec<f64> = game.eval(
        "local page = TimewaysTalkFrameScroll:GetScrollChild()
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent == page and widget.text == 'Not today.' then
                 return widget.textColor
             end
         end",
    );
    let now: Vec<f64> = game.eval(
        "local page = TimewaysTalkFrameScroll:GetScrollChild()
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent == page and widget.text == 'You: Any news?' then
                 return widget.textColor
             end
         end",
    );
    assert!(past[0] > now[0], "{past:?} is not lighter than {now:?}");
}

#[test]
fn an_earlier_talk_of_today_shows_under_today() {
    let game = Game::new();
    load(
        &game,
        &format!(
            "{{ ['{FARLEY}'] = {{ {{ at = {}, said = 'Hello.', heard = 'Hi.' }} }} }}",
            NOW - 60
        ),
    );

    talk_to(&game, FARLEY, "Any news?");

    assert_eq!(page(&game)[0], "Today");
}

#[test]
fn the_history_keeps_the_newest_twenty_of_each_npc() {
    let game = Game::new();

    for n in 0..25 {
        talk_to(&game, FARLEY, &format!("Question {n}?"));
        answer(&game, FARLEY, Some("Hmm."));
    }

    let kept = saved(&game, FARLEY);
    assert_eq!(kept.len(), 20);
    assert!(kept[0].contains("Question 5?"), "{kept:?}");
}

/// A saved table of `npcs` NPCs with 20 exchanges each. "Npc 0" holds the oldest.
fn full_histories(npcs: u64) -> String {
    let exchanges = |npc: u64| -> Vec<String> {
        (0..20)
            .map(|n| {
                format!(
                    "{{ at = {}, said = 'Hi.', heard = 'Hello.' }}",
                    npc * 100 + n
                )
            })
            .collect()
    };
    let lists: Vec<String> = (0..npcs)
        .map(|npc| format!("['Npc {npc}'] = {{ {} }}", exchanges(npc).join(", ")))
        .collect();
    format!("{{ {} }}", lists.join(", "))
}

#[test]
fn the_history_keeps_the_newest_two_hundred_in_all() {
    let game = Game::new();
    load(&game, &full_histories(12));

    assert_eq!(kept_count(&game), 200);
    assert!(saved(&game, "Npc 0").is_empty());
    assert!(saved(&game, "Npc 1").is_empty());
    assert_eq!(saved(&game, "Npc 11").len(), 20);
}

#[test]
fn a_new_exchange_past_the_cap_drops_the_oldest_of_all() {
    let game = Game::new();
    load(&game, &full_histories(10));

    talk_to(&game, FARLEY, "Any news?");
    answer(&game, FARLEY, Some("Nothing but rain."));

    assert_eq!(kept_count(&game), 200);
    assert_eq!(saved(&game, "Npc 0").len(), 19);
    assert_eq!(saved(&game, FARLEY).len(), 1);
}

#[test]
fn broken_entries_are_dropped_when_the_game_loads_them() {
    let game = Game::new();
    let long = "a".repeat(256);
    load(
        &game,
        &format!(
            "{{ ['{FARLEY}'] = {{
                 {{ at = 1, said = 'Hello.', heard = 'Hi.' }},
                 {{ at = 'soon', said = 'Hello.', heard = 'Hi.' }},
                 {{ at = 2, said = '', heard = 'Hi.' }},
                 {{ at = 3, said = 'Hello.', heard = 5 }},
                 {{ at = 4, said = '{long}', heard = 'Hi.' }},
                 {{ at = 5, said = 'Hel\\nlo.', heard = 'Hi.' }},
                 {{ at = 6, said = 'Hello.', heard = '|cffff0000red' }},
                 {{ at = -1, said = 'Hello.', heard = 'Hi.' }},
                 'not a table',
             }},
             [5] = {{ {{ at = 1, said = 'Hello.', heard = 'Hi.' }} }},
             ['Bad|Name'] = {{ {{ at = 1, said = 'Hello.', heard = 'Hi.' }} }},
             ['Empty'] = 'not a list',
           }}"
        ),
    );

    assert_eq!(saved(&game, FARLEY), ["1 Hello. | Hi."]);
    assert_eq!(kept_count(&game), 1);
}

#[test]
fn a_saved_variable_that_is_no_table_starts_empty() {
    let game = Game::new();
    load(&game, "'garbage'");

    talk_to(&game, FARLEY, "Any news?");
    answer(&game, FARLEY, Some("Nothing but rain."));

    assert_eq!(saved(&game, FARLEY).len(), 1);
}

#[test]
fn a_doubled_pipe_from_the_desktop_is_kept_and_shows_as_one() {
    let game = Game::new();
    talk_to(&game, FARLEY, "Any news?");

    answer(&game, FARLEY, Some("A || B"));

    assert_eq!(saved(&game, FARLEY), [format!("{NOW} Any news? | A || B")]);
}
