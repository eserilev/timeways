#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;
use timeways_story::seen::TextKind;

const NOW: Tick = Tick(1_790_000_000);

/// In Goldshire, as Ada, with Innkeeper Farley as the "npc" unit.
fn game() -> Game {
    let game = Game::new();
    game.run(
        "wow.zone = 'Elwynn Forest'
         wow.units.player = { name = 'Ada', player = true }
         wow.units.npc = { name = 'Innkeeper Farley' }",
    );
    game
}

fn seen(kind: TextKind, title: Option<&str>, npc: Option<&str>, text: impl Into<String>) -> Input {
    Input::TextSeen {
        at: NOW,
        kind,
        title: title.map(str::to_string),
        npc: npc.map(str::to_string),
        zone: Some("Elwynn Forest".to_string()),
        text: text.into(),
    }
}

fn met(name: &str) -> Input {
    Input::NpcMet {
        at: NOW,
        name: name.to_string(),
    }
}

/// The text inputs that went out after the next flush.
fn sent_texts(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter(|input| matches!(input, Input::TextSeen { .. }))
        .collect()
}

#[test]
fn gossip_goes_out_after_the_meeting_with_its_npc() {
    let game = game();

    game.run("wow.text.gossip = 'Welcome to the Lion\\'s Pride.'; wow.Fire('GOSSIP_SHOW')");

    game.run("wow.RunTickers()");
    let gossip = seen(
        TextKind::Gossip,
        None,
        Some("Innkeeper Farley"),
        "Welcome to the Lion's Pride.",
    );
    assert_eq!(game.sent_inputs(), [met("Innkeeper Farley"), gossip]);
}

#[test]
fn a_quest_goes_out_with_its_title_and_objectives() {
    let game = game();

    game.run(
        "wow.text.title = 'Wanted: Hogger'
         wow.text.quest = 'Hogger must die.'
         wow.text.objectives = 'Kill Hogger.'
         wow.Fire('QUEST_DETAIL')",
    );

    let quest = seen(
        TextKind::Quest,
        Some("Wanted: Hogger"),
        Some("Innkeeper Farley"),
        "Hogger must die.\n\nKill Hogger.",
    );
    assert_eq!(sent_texts(&game), [quest]);
}

#[test]
fn the_progress_and_the_reward_of_a_quest_go_out() {
    let game = game();

    game.run(
        "wow.text.title = 'Wanted: Hogger'
         wow.text.progress = 'Is he dead yet?'
         wow.Fire('QUEST_PROGRESS')
         wow.text.reward = 'Well done.'
         wow.Fire('QUEST_COMPLETE')",
    );

    let title = Some("Wanted: Hogger");
    let npc = Some("Innkeeper Farley");
    assert_eq!(
        sent_texts(&game),
        [
            seen(TextKind::Quest, title, npc, "Is he dead yet?"),
            seen(TextKind::Quest, title, npc, "Well done."),
        ]
    );
}

#[test]
fn the_greeting_of_an_npc_with_many_quests_is_gossip() {
    let game = game();

    game.run("wow.text.greeting = 'So much to do.'; wow.Fire('QUEST_GREETING')");

    let greeting = seen(
        TextKind::Gossip,
        None,
        Some("Innkeeper Farley"),
        "So much to do.",
    );
    assert_eq!(sent_texts(&game), [greeting]);
}

#[test]
fn a_book_goes_out_with_the_name_of_its_item() {
    let game = game();

    game.run(
        "wow.text.item = 'The Kingdom of Stormwind'
         wow.text.page = 'Long ago, the humans came.'
         wow.Fire('ITEM_TEXT_READY')",
    );

    let book = seen(
        TextKind::Book,
        Some("The Kingdom of Stormwind"),
        None,
        "Long ago, the humans came.",
    );
    assert_eq!(sent_texts(&game), [book]);
}

#[test]
fn a_letter_that_a_player_wrote_never_goes_out() {
    let game = game();

    game.run(
        "wow.text.item = 'Plain Letter'
         wow.text.page = 'Meet me in Goldshire.'
         wow.text.creator = 'Grimtusk'
         wow.Fire('ITEM_TEXT_READY')",
    );

    assert!(sent_texts(&game).is_empty());
    assert!(!game.sent().concat().contains("Goldshire"));
}

#[test]
fn the_name_of_your_character_becomes_a_mark_in_any_case() {
    let game = game();

    game.run("wow.text.gossip = 'Ada! Well met, ada.'; wow.Fire('GOSSIP_SHOW')");

    let gossip = seen(
        TextKind::Gossip,
        None,
        Some("Innkeeper Farley"),
        "$N! Well met, $N.",
    );
    assert_eq!(sent_texts(&game), [gossip]);
    assert!(!game.sent().concat().contains("Ada!"));
}

#[test]
fn the_name_of_your_character_inside_another_word_stays() {
    let game = game();

    game.run(
        "wow.units.player.name = 'Ed'
         wow.text.gossip = 'Ed, you killed the wolf. Well met, ed.'
         wow.Fire('GOSSIP_SHOW')",
    );

    let gossip = seen(
        TextKind::Gossip,
        None,
        Some("Innkeeper Farley"),
        "$N, you killed the wolf. Well met, $N.",
    );
    assert_eq!(sent_texts(&game), [gossip]);
}

#[test]
fn the_same_text_goes_out_once() {
    let game = game();

    game.run(
        "wow.text.gossip = 'Welcome.'
         wow.Fire('GOSSIP_SHOW')
         wow.Fire('GOSSIP_SHOW')",
    );

    assert_eq!(sent_texts(&game).len(), 1);
}

#[test]
fn the_words_of_a_player_who_shares_a_quest_go_out_without_their_name() {
    let game = game();

    game.run(
        "wow.units.npc = { name = 'Grimtusk', player = true }
         wow.text.title = 'Wanted: Hogger'
         wow.text.quest = 'Hogger must die.'
         wow.Fire('QUEST_DETAIL')",
    );

    let quest = seen(
        TextKind::Quest,
        Some("Wanted: Hogger"),
        None,
        "Hogger must die.",
    );
    assert_eq!(sent_texts(&game), [quest]);
    assert!(!game.sent().concat().contains("Grimtusk"));
}

#[test]
fn a_hidden_or_empty_text_never_goes_out() {
    let game = game();

    game.run(
        "wow.text.gossip = 'Secret words.'
         wow.secrets['Secret words.'] = true
         wow.Fire('GOSSIP_SHOW')
         wow.text.greeting = '   '
         wow.Fire('QUEST_GREETING')",
    );

    assert!(sent_texts(&game).is_empty());
}

#[test]
fn a_long_text_is_cut_at_a_whole_character() {
    let game = game();
    game.run("linkLimit = 100000");

    game.run("wow.text.gossip = string.rep('a', 1999) .. 'é' .. 'tail'; wow.Fire('GOSSIP_SHOW')");

    let Some(Input::TextSeen { text, .. }) = sent_texts(&game).pop() else {
        panic!("expected a seen text");
    };
    assert_eq!(text, "a".repeat(1999));
}
