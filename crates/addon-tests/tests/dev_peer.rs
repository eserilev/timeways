//! Fake players of dev mode: their messages come through the real receive path, and the
//! messages to them never go on the wire (TESTING.md, "Dev mode").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::Player;
use timeways_story::dev_mode::DevMode;
use timeways_story::input::Input;
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const KOBEE: &str = "Kobee-Devrealm";

/// Ada, alone, with dev mode on at the desktop.
fn ada() -> Player {
    let ada = Player::new("Ada");
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(DevMode::On);
    let character = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
    let _ = serve::line(&mut story, character.as_bytes().to_vec());
    let ask = r#"{"type":"journal_asked","id":1}"#;
    ada.game
        .reply(&serve::line(&mut story, ask.as_bytes().to_vec()).lines[0]);
    ada
}

fn twdev(player: &Player, message: &str) {
    let slash: mlua::Function = player.eval("return function(m) wow.Slash('/twdev', m) end");
    slash.call::<()>(message).unwrap();
}

/// The answers of the fake players come after a moment, as one-shot timers.
fn run_timers(player: &Player) {
    for _ in 0..3 {
        player.run(
            "local timers = wow.after
             wow.after = {}
             for _, timer in ipairs(timers) do timer.callback() end",
        );
    }
}

fn waiting_authors(player: &Player) -> Vec<String> {
    player.eval(
        "local authors = {}
         for _, story in ipairs(ns.PlayerStories.Waiting()) do authors[#authors + 1] = story.author end
         return authors",
    )
}

fn printed(player: &Player) -> String {
    player.game.printed().join("\n")
}

/// No message of the game went to a fake player on the wire.
fn assert_nothing_on_the_wire_to_fake_players(player: &Player) {
    for sent in player.take_sent() {
        let target = sent.target.unwrap_or_default();
        assert!(!target.contains("Devrealm"), "{target}: {}", sent.text);
    }
}

#[test]
fn a_fake_player_tells_a_story_that_waits_for_your_answer() {
    let ada = ada();

    twdev(&ada, "peer Kobee story The Bridge at Pyrewood");

    assert_eq!(waiting_authors(&ada), [KOBEE]);
    assert!(printed(&ada).contains("told a story about you: The Bridge at Pyrewood"));
}

#[test]
fn accept_keeps_the_story_at_the_desktop_and_answers_the_fake_player_in_the_game() {
    let ada = ada();
    twdev(&ada, "peer Kobee story The Bridge at Pyrewood");

    ada.run("ns.PlayerStories.Accept(1)");
    ada.run("wow.RunTickers()");

    assert!(waiting_authors(&ada).is_empty());
    let accepted = ada
        .game
        .sent_inputs()
        .into_iter()
        .any(|input| matches!(input, Input::StoryAccepted { .. }));
    assert!(accepted);
    assert!(printed(&ada).contains("Kobee-Devrealm got: story_accept"));
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn decline_answers_the_fake_player_in_the_game() {
    let ada = ada();
    twdev(&ada, "peer Kobee story The Bridge at Pyrewood");

    ada.run("ns.PlayerStories.Decline(1)");

    assert!(waiting_authors(&ada).is_empty());
    assert!(printed(&ada).contains("Kobee-Devrealm got: story_decline"));
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn a_blocked_fake_player_gets_no_story_in() {
    let ada = ada();
    twdev(&ada, "peer Kobee story The Bridge at Pyrewood");
    ada.run(&format!("ns.PlayerStories.Block('{KOBEE}')"));

    twdev(&ada, "peer Kobee story Another One");

    assert!(waiting_authors(&ada).is_empty());
    assert!(printed(&ada).contains("Kobee-Devrealm got: story_room"));
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn a_full_box_takes_no_story_from_a_new_fake_player() {
    let ada = ada();
    for letter in 'a'..='t' {
        twdev(&ada, &format!("peer Teller{letter} story Tale {letter}"));
    }

    twdev(&ada, "peer Latecomer story One Too Many");

    assert_eq!(waiting_authors(&ada).len(), 20);
    assert!(printed(&ada).contains("Latecomer-Devrealm got: story_room"));
}

/// The room that the box of the fake player answers to Ada's question.
fn asked_room(ada: &Player) -> String {
    ada.run(&format!(
        "lastRoom = nil ns.PlayerStories.Ask('{KOBEE}', function(room) lastRoom = room end)"
    ));
    run_timers(ada);
    ada.eval("return lastRoom")
}

#[test]
fn the_box_of_a_fake_player_answers_open_full_or_blocked() {
    let ada = ada();
    twdev(&ada, "peer Kobee open");
    assert_eq!(asked_room(&ada), "open");

    twdev(&ada, "peer Kobee full");
    assert_eq!(asked_room(&ada), "full");

    twdev(&ada, "peer Kobee blocked");
    assert_eq!(asked_room(&ada), "blocked");
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn write_opens_the_story_scroll_for_a_fake_player_of_your_group() {
    let ada = ada();

    twdev(&ada, "peer Kobee write");

    assert!(ada.eval::<bool>("ns.StoryScroll.IsShown()"));
    assert_eq!(ada.eval::<String>("ns.StoryScroll.Player()"), KOBEE);
}

#[test]
fn inbox_brings_five_stories_from_five_fake_players() {
    let ada = ada();

    twdev(&ada, "inbox");

    assert_eq!(waiting_authors(&ada).len(), 5);
}

#[test]
fn a_fake_player_sends_a_quest_and_accept_answers_it_in_the_game() {
    let ada = ada();

    twdev(&ada, "peer Kobee quest");
    let key: String = ada.eval("return ns.PlayerTasks.Received()[1].key");
    ada.run(&format!("ns.PlayerTasks.Accept('{key}')"));

    let status: String = ada.eval("return ns.PlayerTasks.Received()[1].task.status");
    assert_eq!(status, "accepted");
    assert!(printed(&ada).contains("Kobee-Devrealm got: accept"));
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn a_fake_player_answers_a_call_and_takes_your_quest_through_to_the_turn_in() {
    let ada = ada();
    twdev(&ada, "peer Kobee near");
    ada.run("ns.PlayerTasks.Call()");
    run_timers(&ada);
    let recipients: Vec<String> = ada.eval(
        "local names = {} for _, r in ipairs(ns.PlayerTasks.Recipients()) do names[#names + 1] = r.name end return names",
    );
    assert!(recipients.contains(&KOBEE.to_string()), "{recipients:?}");

    ada.run(&format!(
        "ns.PlayerTasks.Give({{ title = 'Wolf Pelts', text = '', reward = '', steps = {{
             {{ kind = 'kill', target = 'Prowler', count = 1 }} }} }}, '{KOBEE}')"
    ));
    run_timers(&ada);
    twdev(&ada, "peer Kobee step 1");
    twdev(&ada, "peer Kobee turnin 1");

    let level: String = ada.eval("return ns.PlayerTasks.Given()[1].task.claims[1].level");
    assert!(!level.is_empty());
    assert!(ada.eval::<bool>("return ns.PlayerTasks.Given()[1].task.turnInAt ~= nil"));
    ada.run("ns.PlayerTasks.Complete(ns.PlayerTasks.Given()[1].task.id)");
    let status: String = ada.eval("return ns.PlayerTasks.Given()[1].task.status");
    assert_eq!(status, "done");
    assert!(printed(&ada).contains("Kobee-Devrealm got: result"));
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn msp_and_tooltip_show_the_profile_of_a_fake_player() {
    let ada = ada();
    ada.run("ns.MspProfile.SetSharing(true)");

    twdev(&ada, "msp Kobee / Keeper of the Flame");
    twdev(&ada, "tooltip Kobee");

    let lines: Vec<String> = ada.eval("return wow.tooltip.lines");
    assert!(
        lines.contains(&"Kobee of the Reef, Keeper of the Flame".to_string()),
        "{lines:?}"
    );
    assert_nothing_on_the_wire_to_fake_players(&ada);
}

#[test]
fn remember_and_note_mark_a_fake_player_for_its_tooltip() {
    let ada = ada();

    twdev(&ada, "remember Kobee avoid");
    twdev(&ada, "note Kobee");
    twdev(&ada, "tooltip Kobee");

    let lines: Vec<String> = ada.eval("return wow.tooltip.lines");
    assert!(lines.contains(&"Avoid".to_string()), "{lines:?}");
}

#[test]
fn peer_with_dev_mode_off_makes_no_fake_player() {
    let ada = Player::new("Ada");

    twdev(&ada, "peer Kobee story The Bridge");

    assert!(waiting_authors(&ada).is_empty());
    assert!(!ada.eval::<bool>(&format!("ns.DevPeer.Has('{KOBEE}')")));
}
