//! A whole session of play: the real addon, the real checks of the bridge, and the real
//! story program, with each reply back in the addon. The bridge drops no line of the addon,
//! refuses no batch, and the book shows what happened.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use fake_bridge::{FakeBridge, Reply};
use std::path::Path;
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::Story;

/// The model answers each call as a model does: words, a line, a saga, or a quest.
fn model(prompt: &str) -> String {
    if prompt.contains("Write chapter") {
        r#"{"saga": "Our hero came to Goldshire.", "footnotes": []}"#.to_string()
    } else if prompt.contains("small task of your own") {
        r#"{"title": "The Lost Lantern", "text": "Find it.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#.to_string()
    } else if prompt.contains("A player speaks to you") {
        r#"{"say": "Well met.", "trust": 2}"#.to_string()
    } else if prompt.contains("Tell the moment") {
        "Our hero walks on.".to_string()
    } else {
        "Nobody knows.".to_string()
    }
}

/// Each test gets its own pack, because the tests run at once.
fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("loop-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

/// The addon, with each message of its link kept in `sent` for the bridge.
fn game() -> Game {
    let game = Game::with_transport();
    game.run(
        "sent = {}
         ns.Link.Send = function(text)
             table.insert(sent, text)
             return #sent
         end",
    );
    game
}

/// Takes every message that the addon sent, and gives the reply of the bridge back to the
/// addon, until the addon sends nothing more. A reply can make the addon send again.
fn pump(game: &Game, bridge: &mut FakeBridge) {
    let receive: mlua::Function = game.eval("ns.Link.Receive");
    let mut done = 0;
    loop {
        let sent: Vec<String> = game.eval("sent");
        let Some(text) = sent.get(done) else {
            return;
        };
        done += 1;
        match bridge.batch(text) {
            Reply::Done(reply) => receive.call::<()>((done, "done", reply)).unwrap(),
            Reply::Error(error) => panic!("the bridge refused a batch: {error}\n{text}"),
        }
    }
}

/// Plays one step, flushes the outbox, and runs the bridge.
fn play(game: &Game, bridge: &mut FakeBridge, lua: &str) {
    game.run(lua);
    game.run("ns.Outbox.Flush()");
    pump(game, bridge);
}

fn lines(game: &Game, section: &str) -> String {
    let lines: Vec<String> = game.eval(&format!(
        "local out = {{}}
         for _, line in ipairs(ns.Journal.Lines('{section}')) do table.insert(out, line.text) end
         return out"
    ));
    lines.join("\n")
}

#[test]
fn a_session_of_play_goes_through_the_bridge_and_back_into_the_book() {
    let game = game();
    let mut bridge =
        FakeBridge::new(story("session")).with_model(Box::new(|prompt| Some(model(prompt))));

    let session = [
        "wow.units.player = { name = 'Ada', level = 12, player = true }
         wow.zone, wow.subzone = 'Elwynn Forest', 'Goldshire'
         wow.Fire('PLAYER_ENTERING_WORLD')",
        "wow.units.npc = { name = 'Innkeeper Farley' }
         wow.text.gossip = 'Welcome, Ada!\\n\\nThe inn is warm.'
         wow.Fire('GOSSIP_SHOW')",
        "wow.text.title = 'Wanted: Hogger'
         wow.text.quest = 'Hogger must die.\\nHe lives in the west.'
         wow.text.objectives = 'Kill Hogger.'
         wow.Fire('QUEST_DETAIL')",
        "wow.text.item = 'The Kingdom of Stormwind'
         wow.text.page = 'Long ago, a king\\r\\nruled here.'
         wow.Fire('ITEM_TEXT_READY')",
        "wow.units.target = { name = 'Innkeeper Farley' }
         C_ChatInfo.PerformEmote('DANCE', '')",
        "wow.Slash('/lore', 'who is Hogger?')",
        "wow.Slash('/talk', 'any news?')",
        "wow.Slash('/quest', '')",
        "wow.Slash('/quest', 'accept')",
        "wow.Slash('/hero', 'set goal Find my brother.')",
        "wow.Fire('PLAYER_LEVEL_UP', 13)",
        "wow.Slash('/journal', '')",
    ];
    for step in session {
        play(&game, &mut bridge, step);
    }

    assert_eq!(bridge.dropped_lines(), 0);
    let chapters = lines(&game, "chapters");
    assert!(chapters.contains("Elwynn Forest"), "{chapters}");
    assert!(chapters.contains("Innkeeper Farley"), "{chapters}");
    let learned = lines(&game, "learned");
    assert!(learned.contains("The inn is warm."), "{learned}");
    assert!(learned.contains("Kill Hogger."), "{learned}");
    assert!(learned.contains("ruled here."), "{learned}");
    game.run("ns.Journal.Select('hero', 'goal')");
    let hero = lines(&game, "hero");
    assert!(hero.contains("Find my brother."), "{hero}");
    let quests = lines(&game, "quests");
    assert!(quests.contains("The Lost Lantern"), "{quests}");
}

#[test]
fn a_giver_with_no_task_says_so_in_a_line_of_timeways() {
    let game = game();
    let mut bridge = FakeBridge::new(story("no-task")).with_model(Box::new(|_| None));
    play(
        &game,
        &mut bridge,
        "wow.units.player = { name = 'Ada', level = 12, player = true }
         wow.zone, wow.subzone = 'Elwynn Forest', 'Goldshire'
         wow.Fire('PLAYER_ENTERING_WORLD')",
    );

    play(
        &game,
        &mut bridge,
        "wow.units.target = { name = 'Innkeeper Farley' }
         wow.Slash('/quest', '')",
    );

    let printed = game.printed();
    let shown = "|cffc8a064Timeways|r: Innkeeper Farley has no task for you now.";
    assert!(printed.iter().any(|line| line == shown), "{printed:?}");
    assert!(
        !printed.iter().any(|line| line.contains("Narrator")),
        "{printed:?}"
    );
}
