//! "Help me write this" through the real checks of the bridge and the real story program:
//! the idea goes out, a model answers, and the checked draft comes back to the form
//! (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use fake_bridge::{FakeBridge, Reply};
use std::path::Path;
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::Story;

const DRAFT: &str = r#"{"title": "Trouble at the Mill", "text": "My friend should look in on Bram.",
    "steps": [{"goal": "place", "target": "Mill Pond"}, {"goal": "npc", "target": "Farmer Bram"}]}"#;

/// Each test runs on a thread of its own, so each one gets a pack of its own.
fn story() -> Story {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("task-draft-{n}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

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

/// Gives each batch that the addon sent to the bridge, and each reply back to the addon.
fn pump(game: &Game, bridge: &mut FakeBridge, from: usize) -> usize {
    let receive: mlua::Function = game.eval("ns.Link.Receive");
    let mut done = from;
    loop {
        let sent: Vec<String> = game.eval("sent");
        let Some(text) = sent.get(done) else {
            return done;
        };
        done += 1;
        match bridge.batch(text) {
            Reply::Done(reply) => receive.call::<()>((done, "done", reply)).unwrap(),
            Reply::Error(error) => panic!("the bridge refused a batch: {error}\n{text}"),
        }
    }
}

fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Page('quests').lines) do table.insert(out, line.text) end
         return out",
    )
}

/// Ada stands at Mill Pond, meets Farmer Bram, and asks for help with an idea.
fn ask_for_help(model: fake_bridge::Model) -> Game {
    let game = game();
    let mut bridge = FakeBridge::new(story()).with_model(model);
    game.run(
        "wow.units.player = { name = 'Ada', level = 12, player = true }
         wow.zone, wow.subzone = 'Testvale', 'Mill Pond'
         wow.Fire('PLAYER_ENTERING_WORLD')
         wow.units.npc = { name = 'Farmer Bram' }
         wow.Fire('GOSSIP_SHOW')
         ns.Outbox.Flush()",
    );
    let done = pump(&game, &mut bridge, 0);
    game.run(
        "ns.JournalFrame.Open('quests')
         ns.TaskForm.Open()
         for _, button in ipairs(ns.Journal.Page('quests').buttons) do
             if button.label == 'Help me write this' then button.run() end
         end
         wow.EditBox():SetText('Ada wants a friend to check on bram at the pond')
         ns.Editor.Save()",
    );
    pump(&game, &mut bridge, done);
    game
}

#[test]
fn a_draft_of_the_model_comes_back_to_the_form_through_the_bridge() {
    let game = ask_for_help(Box::new(|prompt| {
        let answer = if prompt.contains("small task for a friend") {
            DRAFT
        } else {
            "Our hero walks on."
        };
        Some(answer.to_string())
    }));

    let page = lines(&game);
    assert!(page.contains(&"Suggestion".to_string()), "{page:?}");
    assert!(page.contains(&"Trouble at the Mill".to_string()));
    assert!(page.contains(&"Go to Mill Pond.".to_string()));
    assert!(page.contains(&"Talk to Farmer Bram.".to_string()));
}

#[test]
fn the_idea_reaches_the_model_without_the_name_of_the_player() {
    let prompts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen = prompts.clone();
    let _game = ask_for_help(Box::new(move |prompt| {
        seen.borrow_mut().push(prompt.to_string());
        Some(DRAFT.to_string())
    }));

    let prompts = prompts.borrow();
    let draft = prompts
        .iter()
        .find(|prompt| prompt.contains("small task for a friend"))
        .unwrap();
    assert!(
        draft.contains("$N wants a friend to check on bram at the pond"),
        "{draft}"
    );
    assert!(!draft.contains("Ada"));
}

#[test]
fn a_draft_with_a_place_that_the_world_does_not_know_never_reaches_the_form() {
    let game = ask_for_help(Box::new(|_| {
        Some(
            r#"{"title": "Far Away", "text": "Go.", "steps": [{"goal": "place", "target": "Stormwind"}]}"#
                .to_string(),
        )
    }));

    let page = lines(&game);
    assert!(!page.contains(&"Suggestion".to_string()));
    assert!(
        page.contains(&"Timeways couldn't turn that into a task. Try other words.".to_string())
    );
}

#[test]
fn with_no_model_the_form_says_that_no_draft_came() {
    let game = ask_for_help(Box::new(|_| None));

    assert!(
        lines(&game)
            .contains(&"Timeways couldn't turn that into a task. Try other words.".to_string())
    );
}
