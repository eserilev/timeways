//! Stories that players tell about each other, between two games with Timeways (GAMEPLAY.md
//! 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, ada_and_corvin, exchange};
use timeways_story::input::Input;

const PREFIX: &str = "|cffc8a064Timeways|r: ";

/// Ada and Corvin, in one party and one guild.
fn party() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    (ada, corvin)
}

/// Ada targets Corvin and tells the story, and the message arrives.
fn tell(ada: &Player, corvin: &Player, text: &str) {
    ada.run("wow.units.target = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");
    ada.run(&format!("wow.Slash('/story', '{text}')"));
    exchange(ada, corvin);
}

fn waiting(corvin: &Player) -> usize {
    corvin.eval("return #ns.PlayerStories.Waiting()")
}

/// The inputs that the game sent to the desktop, after the outbox flushed.
fn sent_inputs(player: &Player) -> Vec<Input> {
    player.run("wow.RunTickers()");
    player.game.sent_inputs()
}

#[test]
fn a_story_from_a_party_member_waits_for_an_answer() {
    let (ada, corvin) = party();

    tell(&ada, &corvin, "Corvin held the bridge alone.");

    assert_eq!(waiting(&corvin), 1);
    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Ada told a story about you. Type /story to read it."
    )));
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}You told Corvin a story about them."))
    );
}

#[test]
fn a_story_needs_a_target_in_your_group() {
    let (ada, corvin) = ada_and_corvin();

    tell(&ada, &corvin, "Corvin held the bridge alone.");

    assert_eq!(waiting(&corvin), 0);
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Target a player in your group first."))
    );
}

#[test]
fn a_story_from_outside_the_group_is_dropped() {
    let (_, corvin) = ada_and_corvin();

    corvin.hear("Timeways", "1;story;a1;A tale.", "WHISPER", "Ada-Stormrage");

    assert_eq!(waiting(&corvin), 0);
}

#[test]
fn a_story_from_a_blocked_player_is_dropped() {
    let (ada, corvin) = party();
    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");

    tell(&ada, &corvin, "Corvin held the bridge alone.");

    assert_eq!(waiting(&corvin), 0);
}

#[test]
fn a_player_holds_at_most_three_stories_from_one_author() {
    let (ada, corvin) = party();

    for n in 1..=5 {
        tell(&ada, &corvin, &format!("Tale number {n}."));
    }

    assert_eq!(waiting(&corvin), 3);
}

#[test]
fn an_accepted_story_goes_to_the_desktop_without_names_and_the_author_hears() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Corvin and Ada held the bridge.");

    corvin.run("wow.Slash('/story', 'accept')");
    exchange(&ada, &corvin);

    let inputs = sent_inputs(&corvin);
    assert!(
        inputs.iter().any(|input| matches!(
            input,
            Input::StoryAccepted { number: 1, text, .. }
                if text == "$N and my friend held the bridge."
        )),
        "{inputs:?}"
    );
    assert_eq!(waiting(&corvin), 0);
    let author: String = corvin.eval("return ns.PlayerStories.AuthorOf(1)");
    assert_eq!(author, "Ada-Stormrage");
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
}

#[test]
fn a_declined_story_never_reaches_the_desktop_and_the_author_hears() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Corvin fell in the river.");

    corvin.run("wow.Slash('/story', 'decline')");
    exchange(&ada, &corvin);

    let inputs = sent_inputs(&corvin);
    assert!(
        !inputs
            .iter()
            .any(|input| matches!(input, Input::StoryAccepted { .. })),
        "{inputs:?}"
    );
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin declined your story."))
    );
}

#[test]
fn the_answer_of_a_story_never_touches_a_quest_with_the_same_id() {
    let (ada, corvin) = party();
    let id: String = ada.eval(
        "return ns.PlayerTasks.Give({ title = 'A Walk', text = '', reward = '',
             steps = { { kind = 'place', target = 'Brill', count = 1 } } }, 'Corvin-Stormrage')",
    );
    exchange(&ada, &corvin);

    ada.hear(
        "Timeways",
        &format!("1;story_accept;{id}"),
        "WHISPER",
        "Corvin-Stormrage",
    );

    let status: String = ada.eval(&format!("return ns.TaskStore.Data().given['{id}'].status"));
    assert_eq!(status, "offered");
}

#[test]
fn a_story_comes_through_the_wire_as_it_went_in() {
    let (ada, _) = party();

    let text: String = ada.eval(
        "local message = ns.TaskWire.Decode(ns.TaskWire.Encode({ type = 'story', id = 'a1',
             text = 'We held; 100% of it.' }))
         return message.text",
    );

    assert_eq!(text, "We held; 100% of it.");
}
