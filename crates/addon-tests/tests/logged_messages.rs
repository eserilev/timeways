//! Text that a player wrote goes on the logged addon channel, so Blizzard support can read a
//! reported message (GAMEPLAY.md 4.7 and 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{LOGGED, NORMAL, Player, accepted_task, ada_and_corvin, exchange, received_key};

const STEPS: &str = "{ { kind = 'place', target = 'Agamand Mills', count = 1 } }";

/// The type of a one-part message, such as "offer" for "12:1:1:1;offer;...".
fn type_of(text: &str) -> String {
    let fields: Vec<&str> = text.split(';').collect();
    fields[1].to_string()
}

/// The type and the event of each message that the game sent since the last delivery.
fn sent_types(player: &Player) -> Vec<(String, String)> {
    player
        .take_sent()
        .into_iter()
        .map(|sent| (type_of(&sent.text), sent.event))
        .collect()
}

fn pair(kind: &str, event: &str) -> (String, String) {
    (kind.to_string(), event.to_string())
}

fn give(ada: &Player) -> String {
    ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'A Walk', text = 'Go to the mills.', reward = '',
             steps = {STEPS} }}, 'Corvin-Stormrage')"
    ))
}

fn received_count(corvin: &Player) -> usize {
    corvin
        .eval("local n = 0 for _ in pairs(ns.TaskStore.Data().received) do n = n + 1 end return n")
}

fn waiting_stories(corvin: &Player) -> usize {
    corvin.eval("return #ns.PlayerStories.Waiting()")
}

fn party() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    (ada, corvin)
}

#[test]
fn an_offer_goes_on_the_logged_channel() {
    let (ada, _corvin) = ada_and_corvin();

    give(&ada);

    assert_eq!(sent_types(&ada), [pair("offer", LOGGED)]);
}

#[test]
fn a_story_goes_on_the_logged_channel() {
    let (ada, _corvin) = party();

    ada.run("ns.PlayerStories.Send('Corvin-Stormrage', '', 'We held the bridge.')");

    assert_eq!(sent_types(&ada), [pair("story", LOGGED)]);
}

#[test]
fn the_question_of_a_story_and_its_room_go_on_the_normal_channel() {
    let (ada, corvin) = party();

    ada.run("ns.PlayerStories.Ask('Corvin-Stormrage', function() end)");
    let asked = sent_types(&ada);
    corvin.hear("Timeways", "1:1:1:1;story_ask", "WHISPER", &ada.full_name());

    assert_eq!(asked, [pair("story_ask", NORMAL)]);
    assert_eq!(sent_types(&corvin), [pair("story_room", NORMAL)]);
}

#[test]
fn a_step_and_a_turn_in_go_on_the_logged_channel() {
    let (_ada, corvin, id) = accepted_task(STEPS);
    let key = received_key(&id);

    corvin.run(&format!(
        "ns.PlayerTasks.Claim(ns.TaskStore.Data().received['{key}'], 1)
         ns.PlayerTasks.AskTurnIn('{key}')"
    ));

    assert_eq!(
        sent_types(&corvin),
        [pair("step", LOGGED), pair("turnin", LOGGED)]
    );
}

#[test]
fn a_control_message_stays_normal() {
    let (ada, corvin) = ada_and_corvin();
    give(&ada);
    exchange(&ada, &corvin);

    corvin.run("ns.PlayerTasks.Accept(ns.PlayerTasks.Received()[1].key)");
    ada.run("ns.PlayerTasks.Call()");

    assert_eq!(sent_types(&corvin), [pair("accept", NORMAL)]);
    assert_eq!(sent_types(&ada), [pair("hello", NORMAL)]);
}

#[test]
fn an_offer_that_arrives_on_the_normal_channel_is_dropped() {
    let (ada, corvin) = ada_and_corvin();
    let offer = "1:1:1:1;offer;k1;Hi;Go.;;1;npc;Renee;1";

    corvin.hear("Timeways", offer, "WHISPER", &ada.full_name());
    let unlogged = received_count(&corvin);
    corvin.hear_logged("Timeways", offer, "WHISPER", &ada.full_name());

    assert_eq!(unlogged, 0);
    assert_eq!(received_count(&corvin), 1);
}

#[test]
fn a_story_that_arrives_on_the_normal_channel_is_dropped() {
    let (ada, corvin) = party();
    let story = "1:1:1:1;story;a1;;A tale.";

    corvin.hear("Timeways", story, "WHISPER", &ada.full_name());
    let unlogged = waiting_stories(&corvin);
    corvin.hear_logged("Timeways", story, "WHISPER", &ada.full_name());

    assert_eq!(unlogged, 0);
    assert_eq!(waiting_stories(&corvin), 1);
}

#[test]
fn a_control_message_on_the_logged_channel_is_taken() {
    let (ada, corvin) = ada_and_corvin();

    corvin.hear_logged("Timeways", "1:1:1:1;hello", "GUILD", &ada.full_name());

    assert_eq!(sent_types(&corvin), [pair("here", NORMAL)]);
}

#[test]
fn the_parts_of_one_message_on_two_channels_never_join() {
    let (ada, corvin) = ada_and_corvin();
    let text = "x".repeat(300);

    corvin.hear_logged(
        "Timeways",
        &format!("5:1:2:1;offer;k1;Hi;{}", &text[..230]),
        "WHISPER",
        &ada.full_name(),
    );
    corvin.hear(
        "Timeways",
        &format!("5:2:2:{};;1;npc;Renee;1", &text[230..]),
        "WHISPER",
        &ada.full_name(),
    );

    assert_eq!(received_count(&corvin), 0);
}

/// Every printable ASCII byte but `|`, which the wire refuses.
fn printable_ascii() -> String {
    (32u8..=126)
        .filter(|byte| *byte != b'|')
        .map(char::from)
        .collect()
}

/// Letters of two, three, and four bytes, so the cuts between parts fall inside letters.
const LETTERS: &str = "é✓😀";

#[test]
fn an_offer_with_every_allowed_byte_survives_the_logged_channel() {
    let (ada, corvin) = ada_and_corvin();
    let ascii = printable_ascii();
    let title = LETTERS.repeat(6);
    let text = format!("{ascii} {}", LETTERS.repeat(30));
    let reward = format!("{} {}", &ascii[..90], LETTERS.repeat(11));
    let target = format!("{}é", &ascii[1..60]);

    let give: mlua::Function = ada.eval(
        "return function(title, text, reward, target)
             return ns.PlayerTasks.Give({ title = title, text = text, reward = reward,
                 steps = { { kind = 'other', target = target, count = 1 } } }, 'Corvin-Stormrage')
         end",
    );
    let id: String = give
        .call((
            title.as_str(),
            text.as_str(),
            reward.as_str(),
            target.as_str(),
        ))
        .unwrap();
    exchange(&ada, &corvin);

    let got: Vec<String> = corvin.eval(&format!(
        "local task = ns.TaskStore.Data().received['{}']
         return {{ task.title, task.text, task.reward, task.steps[1].target }}",
        received_key(&id)
    ));
    assert_eq!(got, [title, text, reward, target]);
}
