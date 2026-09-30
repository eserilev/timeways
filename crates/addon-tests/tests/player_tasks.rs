//! A task that one player gives another, between two games with Timeways: the offer, the
//! answer, the steps, and the turn-in (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, ada_and_corvin, exchange};

const DRAFT: &str =
    "{ title = 'Trouble at Agamand Mills', text = 'Put Gregor to rest.', reward = '5 gold',
    steps = { { kind = 'place', target = 'Agamand Mills', count = 1 },
              { kind = 'kill', target = 'Gregor Agamand', count = 1 } } }";

/// Ada gives Corvin the task, and the message arrives. Returns its id.
fn give(ada: &Player, corvin: &Player) -> String {
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({DRAFT}, 'Corvin-Stormrage')"
    ));
    exchange(ada, corvin);
    id
}

fn received_key(id: &str) -> String {
    format!("Ada-Stormrage/{id}")
}

fn given_status(ada: &Player, id: &str) -> String {
    ada.eval(&format!("ns.TaskStore.Data().given['{id}'].status"))
}

fn received_status(corvin: &Player, id: &str) -> Option<String> {
    corvin.eval(&format!(
        "local task = ns.TaskStore.Data().received['{}'] return task and task.status",
        received_key(id)
    ))
}

fn accepted() -> (Player, Player, String) {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);
    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);
    (ada, corvin, id)
}

#[test]
fn an_offer_from_a_guild_member_waits_for_the_doer() {
    let (ada, corvin) = ada_and_corvin();

    let id = give(&ada, &corvin);

    assert_eq!(received_status(&corvin, &id).as_deref(), Some("offered"));
    assert!(
        corvin
            .printed()
            .contains(&"|cffc8a064Timeways|r: Ada sent you a task: Trouble at Agamand Mills. Open your journal to read it.".to_string())
    );
}

#[test]
fn an_offer_from_a_stranger_is_dropped() {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_guild_with(&[&corvin]);

    let id = give(&ada, &corvin);

    assert_eq!(received_status(&corvin, &id), None);
}

#[test]
fn an_offer_from_a_party_member_or_a_friend_comes_through() {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    let bram = Player::new("Bram");
    bram.run("wow.friends = { { name = 'Corvin', connected = true } }");
    corvin.run("wow.friends = { { name = 'Bram', connected = true } }");

    let from_party = give(&ada, &corvin);
    let from_friend: String = bram.eval(&format!(
        "return ns.PlayerTasks.Give({DRAFT}, 'Corvin-Stormrage')"
    ));
    exchange(&bram, &corvin);

    assert_eq!(
        received_status(&corvin, &from_party).as_deref(),
        Some("offered")
    );
    let key = format!("Bram-Stormrage/{from_friend}");
    let status: String = corvin.eval(&format!("ns.TaskStore.Data().received['{key}'].status"));
    assert_eq!(status, "offered");
}

#[test]
fn the_giver_learns_that_the_doer_accepted() {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);

    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);

    assert_eq!(given_status(&ada, &id), "accepted");
    assert_eq!(received_status(&corvin, &id).as_deref(), Some("accepted"));
}

#[test]
fn the_giver_learns_that_the_doer_declined() {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);

    corvin.run(&format!("ns.PlayerTasks.Decline('{}')", received_key(&id)));
    exchange(&ada, &corvin);

    assert_eq!(given_status(&ada, &id), "declined");
}

#[test]
fn a_blocked_player_can_send_no_more_tasks() {
    let (ada, corvin) = ada_and_corvin();
    let first = give(&ada, &corvin);

    corvin.run(&format!("ns.PlayerTasks.Block('{}')", received_key(&first)));
    exchange(&ada, &corvin);
    let second: Option<String> = ada.eval(&format!(
        "return ns.PlayerTasks.Give({DRAFT}, 'Corvin-Stormrage')"
    ));
    ada.run(
        "ns.TaskChannel.Whisper('Corvin-Stormrage', { type = 'offer', id = 'x1', title = 'Again',
            text = 'Please.', reward = '', steps = { { kind = 'npc', target = 'Renee', count = 1 } } })",
    );
    exchange(&ada, &corvin);

    assert_eq!(given_status(&ada, &first), "declined");
    assert_eq!(second, None);
    assert_eq!(received_status(&corvin, "x1"), None);
}

#[test]
fn only_the_doer_can_answer_a_task() {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);
    let bram = Player::new("Bram");

    ada.hear(
        "Timeways",
        &format!("1:1:1:1;accept;{id}"),
        "WHISPER",
        &bram.full_name(),
    );

    assert_eq!(given_status(&ada, &id), "offered");
}

#[test]
fn only_the_giver_can_cancel_a_task() {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);

    corvin.hear(
        "Timeways",
        &format!("1:1:1:1;cancel;{id}"),
        "WHISPER",
        "Bram-Stormrage",
    );
    let after_bram = received_status(&corvin, &id);
    ada.run(&format!("ns.PlayerTasks.Cancel('{id}')"));
    exchange(&ada, &corvin);

    assert_eq!(after_bram.as_deref(), Some("offered"));
    assert_eq!(received_status(&corvin, &id).as_deref(), Some("cancelled"));
}

#[test]
fn a_giver_has_at_most_three_offers_waiting() {
    let (ada, corvin) = ada_and_corvin();

    let ids: Vec<String> = (0..4).map(|_| give(&ada, &corvin)).collect();

    let waiting: Vec<Option<String>> = ids.iter().map(|id| received_status(&corvin, id)).collect();
    let offered = Some("offered".to_string());
    assert_eq!(waiting, [offered.clone(), offered.clone(), offered, None]);
}

#[test]
fn a_step_that_the_doer_claims_reaches_the_giver() {
    let (ada, corvin, id) = accepted();

    corvin.run(&format!(
        "ns.PlayerTasks.Claim(ns.TaskStore.Data().received['{}'], 2)",
        received_key(&id)
    ));
    exchange(&ada, &corvin);

    let zone: String = ada.eval(&format!("ns.TaskStore.Data().given['{id}'].claims[2].zone"));
    assert_eq!(zone, "Tirisfal Glades");
}

#[test]
fn a_claim_of_a_step_that_the_task_lacks_is_dropped() {
    let (ada, corvin, id) = accepted();

    ada.hear(
        "Timeways",
        &format!("1:1:1:1;step;{id};5;1790000000;Brill"),
        "WHISPER",
        &corvin.full_name(),
    );

    let claims: usize = ada.eval(&format!(
        "local n = 0 for _ in pairs(ns.TaskStore.Data().given['{id}'].claims) do n = n + 1 end return n"
    ));
    assert_eq!(claims, 0);
}

#[test]
fn a_step_from_the_doer_counts_as_an_accept_that_got_lost() {
    let (ada, corvin) = ada_and_corvin();
    let id = give(&ada, &corvin);

    ada.hear(
        "Timeways",
        &format!("1:1:1:1;step;{id};1;1790000000;Tirisfal Glades"),
        "WHISPER",
        &corvin.full_name(),
    );

    assert_eq!(given_status(&ada, &id), "accepted");
}

#[test]
fn a_turn_in_carries_every_claim_to_the_giver() {
    let (ada, corvin, id) = accepted();
    let key = received_key(&id);
    corvin.run(&format!(
        "local task = ns.TaskStore.Data().received['{key}']
         task.claims[1] = {{ at = 1790000100, zone = 'Tirisfal Glades' }}
         task.claims[2] = {{ at = 1790000200, zone = 'Tirisfal Glades' }}"
    ));

    corvin.run(&format!("ns.PlayerTasks.AskTurnIn('{key}')"));
    exchange(&ada, &corvin);

    let times: Vec<u64> = ada.eval(&format!(
        "local claims = ns.TaskStore.Data().given['{id}'].claims return {{ claims[1].at, claims[2].at }}"
    ));
    assert_eq!(times, [1_790_000_100, 1_790_000_200]);
    let asked: bool = ada.eval(&format!(
        "ns.TaskStore.Data().given['{id}'].turnInAt ~= nil"
    ));
    assert!(asked);
}

#[test]
fn the_giver_completes_a_task_only_face_to_face() {
    let (ada, corvin, id) = accepted();
    ada.run("wow.units.target = { name = 'Corvin', player = true }");

    let far: bool = ada.eval(&format!("ns.PlayerTasks.Complete('{id}')"));
    ada.run("wow.units.target.near = true");
    let near: bool = ada.eval(&format!("ns.PlayerTasks.Complete('{id}')"));
    exchange(&ada, &corvin);

    assert!(!far);
    assert!(near);
    assert_eq!(given_status(&ada, &id), "done");
    assert_eq!(received_status(&corvin, &id).as_deref(), Some("done"));
}

#[test]
fn not_yet_gives_the_task_back_to_the_doer() {
    let (ada, corvin, id) = accepted();
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);

    ada.run(&format!("ns.PlayerTasks.NotYet('{id}')"));
    exchange(&ada, &corvin);

    assert_eq!(received_status(&corvin, &id).as_deref(), Some("accepted"));
    let asked: bool = corvin.eval(&format!(
        "ns.TaskStore.Data().received['{}'].turnInAt ~= nil",
        received_key(&id)
    ));
    assert!(!asked);
    assert!(corvin.printed().contains(
        &"|cffc8a064Timeways|r: Ada says Trouble at Agamand Mills isn't done yet.".to_string()
    ));
}

#[test]
fn a_done_task_cannot_be_canceled() {
    let (ada, corvin, id) = accepted();
    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");
    ada.run(&format!("ns.PlayerTasks.Complete('{id}')"));
    exchange(&ada, &corvin);

    ada.run(&format!("ns.PlayerTasks.Cancel('{id}')"));
    exchange(&ada, &corvin);

    assert_eq!(received_status(&corvin, &id).as_deref(), Some("done"));
}
