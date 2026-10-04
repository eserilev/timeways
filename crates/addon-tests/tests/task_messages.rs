//! The addon messages of player tasks: the wire format, the parts of 255 bytes, and the
//! rate limits (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use common::Game;
use players::{Player, ada_and_corvin, deliver};

const OFFER: &str = "{ type = 'offer', id = 'k3x9', title = 'Trouble at Agamand Mills',
    text = 'Gregor still walks the mills; bring me 100% of the linen.', reward = '5 gold',
    steps = { { kind = 'place', target = 'Agamand Mills', count = 1 },
              { kind = 'kill', target = 'Gregor Agamand', count = 1 },
              { kind = 'item', target = 'Linen Cloth', count = 10 } } }";

/// The message after a trip through the wire, as a line of its fields.
fn round_trip(game: &Game, message: &str) -> String {
    game.eval(&format!(
        "local message = ns.TaskWire.Decode(ns.TaskWire.Encode({message}))
         local steps = {{}}
         for _, step in ipairs(message.steps or {{}}) do
             table.insert(steps, step.kind .. ' ' .. step.target .. ' x' .. step.count)
         end
         return message.type .. ' | ' .. message.title .. ' | ' .. message.text .. ' | '
             .. message.reward .. ' | ' .. table.concat(steps, ', ')"
    ))
}

/// Why the wire refuses a text, or "ok".
fn decode(game: &Game, text: &str) -> String {
    let decode: mlua::Function = game.eval(
        "return function(text)
             local message, reason = ns.TaskWire.Decode(text)
             return message and 'ok' or reason
         end",
    );
    decode.call(text).unwrap()
}

#[test]
fn an_offer_comes_through_the_wire_as_it_went_in() {
    let game = Game::new();

    let offer = round_trip(&game, OFFER);

    assert_eq!(
        offer,
        "offer | Trouble at Agamand Mills | Gregor still walks the mills; bring me 100% of the linen. \
         | 5 gold | place Agamand Mills x1, kill Gregor Agamand x1, item Linen Cloth x10"
    );
}

#[test]
fn a_well_formed_message_of_each_type_is_taken() {
    let game = Game::new();

    for text in [
        "1;hello",
        "1;here",
        "1;accept;k3x9",
        "1;decline;k3x9",
        "1;block;k3x9",
        "1;cancel;k3x9",
        "1;step;k3x9;2;1790000000;Tirisfal Glades",
        "1;turnin;k3x9;1;1;1790000000;",
        "1;result;k3x9;notyet",
        "1;offer;a1;Title;Text;;1;npc;Innkeeper Renee;1",
        "1;offer;a1;Title;;5 gold;1;other;Find my lost ring;1",
    ] {
        assert_eq!(decode(&game, text), "ok", "{text}");
    }
}

#[test]
fn a_broken_message_is_refused_with_its_reason() {
    let game = Game::new();

    let cases = [
        ("2;hello", "unknown version"),
        ("1;mail;k3x9", "unknown type"),
        ("1;accept", "bad id"),
        ("1;accept;k3 x9", "bad id"),
        ("1;accept;k3x9;more", "too many fields"),
        ("1;step;k3x9;6;1790000000;Brill", "bad index"),
        ("1;step;k3x9;0;1790000000;Brill", "bad index"),
        ("1;step;k3x9;1;-5;Brill", "bad at"),
        ("1;result;k3x9;maybe", "bad verdict"),
        ("1;offer;a1;;Text;;1;npc;Renee;1", "bad title"),
        ("1;offer;a1;Title;Text;;0", "bad steps"),
        ("1;offer;a1;Title;Text;;6", "bad steps"),
        ("1;offer;a1;Title;Text;;1;fly;Renee;1", "bad steps"),
        ("1;offer;a1;Title;Text;;1;kill;Renee;251", "bad steps"),
        ("1;offer;a1;Title;Text;;2;kill;Renee;1", "bad steps"),
    ];
    for (text, reason) in cases {
        assert_eq!(decode(&game, text), reason, "{text}");
    }
}

#[test]
fn a_text_that_could_start_a_wow_escape_is_refused() {
    let game = Game::new();

    let colored = decode(&game, "1;offer;a1;|cffff0000Red|r;Text;;1;npc;Renee;1");
    let escaped = decode(&game, "1;offer;a1;%7Cc;Text;;1;npc;Renee;1");
    let newline = decode(&game, "1;offer;a1;Title;Line%0Abreak;;1;npc;Renee;1");

    assert_eq!(
        [colored, escaped, newline],
        ["bad title", "bad title", "bad text"]
    );
}

#[test]
fn a_text_that_the_logged_channel_refuses_is_refused() {
    let game = Game::new();

    let broken_letter = decode(&game, "1;offer;a1;Caf%C3;Text;;1;npc;Renee;1");
    let lone_continuation = decode(&game, "1;offer;a1;Title;%A9t%C3%A9;;1;npc;Renee;1");
    let banned_sign = decode(&game, "1;offer;a1;Title;Text;%E5%8D%8D;1;npc;Renee;1");
    let not_a_letter = decode(&game, "1;story;a1;End%EF%BF%BF");

    assert_eq!(
        [broken_letter, lone_continuation, banned_sign, not_a_letter],
        ["bad title", "bad text", "bad reward", "bad text"]
    );
}

#[test]
fn a_text_with_letters_past_ascii_is_taken() {
    let game = Game::new();

    let taken = decode(&game, "1;offer;a1;Café Tirisfal;Ça va? ✓ 🐺;;1;npc;Renée;1");

    assert_eq!(taken, "ok");
}

#[test]
fn a_backslash_goes_escaped() {
    let game = Game::new();

    let wire: String =
        game.eval("return ns.TaskWire.Encode({ type = 'story', id = 'a1', text = [[C:\\Wow]] })");

    assert_eq!(wire, "1;story;a1;C:%5CWow");
}

#[test]
fn the_form_takes_out_a_sign_that_blizzard_bans() {
    let game = Game::new();

    let title: String = game.eval("return ns.TaskForm.Clean('Bad 卍 sign 卐')");

    assert_eq!(title, "Bad   sign");
}

#[test]
fn a_text_past_its_limit_is_refused() {
    let game = Game::new();
    let title = "a".repeat(61);
    let text = "a".repeat(401);

    let long_title = decode(&game, &format!("1;offer;a1;{title};Text;;1;npc;Renee;1"));
    let long_text = decode(&game, &format!("1;offer;a1;Title;{text};;1;npc;Renee;1"));

    assert_eq!([long_title, long_text], ["bad title", "bad text"]);
}

#[test]
fn a_reward_past_its_limit_is_refused() {
    let game = Game::new();
    let fits = "a".repeat(200);
    let long = "a".repeat(201);

    let fitting = decode(
        &game,
        &format!("1;offer;a1;Title;Text;{fits};1;npc;Renee;1"),
    );
    let too_long = decode(
        &game,
        &format!("1;offer;a1;Title;Text;{long};1;npc;Renee;1"),
    );

    assert_eq!([fitting, too_long], ["ok", "bad reward"]);
}

/// The parts of a text, and the text that the collector puts back together from them in
/// this order.
fn split_and_join(game: &Game, text: &str, order: &str) -> (Vec<String>, Option<String>) {
    let run: mlua::Function = game.eval(&format!(
        "return function(text)
             local parts = ns.TaskChunks.Split(text, 7)
             local collector, whole = ns.TaskChunks.NewCollector(), nil
             for _, n in ipairs({order}) do
                 whole = ns.TaskChunks.Add(collector, 'Ada-Stormrage', parts[n], 0) or whole
             end
             return parts, whole
         end"
    ));
    run.call(text).unwrap()
}

#[test]
fn a_long_message_goes_in_parts_of_at_most_255_bytes() {
    let game = Game::new();
    let text = "x".repeat(1000);

    let (parts, whole) = split_and_join(&game, &text, "{ 1, 2, 3, 4, 5 }");

    assert_eq!(parts.len(), 5);
    assert!(parts.iter().all(|part| part.len() <= 255));
    assert_eq!(whole.as_deref(), Some(text.as_str()));
}

#[test]
fn a_cut_never_splits_a_letter() {
    let game = Game::new();
    let text = format!("a{}", "é".repeat(300));

    let (parts, whole) = split_and_join(&game, &text, "{ 1, 2, 3 }");

    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|part| part.len() <= 255));
    assert_eq!(whole.as_deref(), Some(text.as_str()));
}

#[test]
fn parts_that_come_out_of_order_make_the_whole_message() {
    let game = Game::new();
    let text = "y".repeat(600);

    let (_, whole) = split_and_join(&game, &text, "{ 3, 1, 2 }");

    assert_eq!(whole.as_deref(), Some(text.as_str()));
}

#[test]
fn a_part_that_comes_twice_counts_once() {
    let game = Game::new();
    let text = "z".repeat(600);

    let (_, whole) = split_and_join(&game, &text, "{ 1, 1, 2 }");

    assert_eq!(whole, None);
}

#[test]
fn a_message_too_long_for_sixteen_parts_is_never_cut() {
    let game = Game::new();

    let parts: Option<Vec<String>> =
        game.eval("return ns.TaskChunks.Split(string.rep('x', 4000), 1)");

    assert_eq!(parts, None);
}

#[test]
fn a_part_older_than_a_minute_is_dropped() {
    let game = Game::new();

    let whole: Option<String> = game.eval(
        "local collector = ns.TaskChunks.NewCollector()
         ns.TaskChunks.Add(collector, 'Ada-Stormrage', '1:1:2:ab', 0)
         ns.TaskChunks.Add(collector, 'Bram-Stormrage', '1:1:1:x', 61)
         return ns.TaskChunks.Add(collector, 'Ada-Stormrage', '1:2:2:cd', 61)",
    );

    assert_eq!(whole, None);
}

#[test]
fn a_peer_with_too_many_open_messages_loses_the_oldest() {
    let game = Game::new();

    let wholes: Vec<String> = game.eval(
        "local collector = ns.TaskChunks.NewCollector()
         for number = 1, 5 do
             ns.TaskChunks.Add(collector, 'Ada-Stormrage', number .. ':1:2:a', number)
         end
         return {
             ns.TaskChunks.Add(collector, 'Ada-Stormrage', '1:2:2:b', 6) or '',
             ns.TaskChunks.Add(collector, 'Ada-Stormrage', '5:2:2:b', 6) or '',
         }",
    );

    assert_eq!(wholes, ["", "ab"]);
}

#[test]
fn a_part_with_a_broken_header_is_dropped() {
    let game = Game::new();

    for chunk in [
        "",
        "hello",
        "1:0:1:x",
        "1:2:1:x",
        "1:1:17:x",
        "12345:1:1:x",
        "a:1:1:x",
    ] {
        let whole: Option<String> = game.eval(&format!(
            "return ns.TaskChunks.Add(ns.TaskChunks.NewCollector(), 'Ada-Stormrage', '{chunk}', 0)"
        ));
        assert_eq!(whole, None, "{chunk}");
    }
}

#[test]
fn the_addon_listens_on_its_own_prefix() {
    let game = Game::new();

    let registered: bool = game.eval("wow.prefixes.Timeways == true");

    assert!(registered);
}

#[test]
fn a_message_on_another_prefix_is_ignored() {
    let (ada, corvin) = ada_and_corvin();

    corvin.hear("GnomishRelay", "1;hello", "GUILD", &ada.full_name());

    assert!(corvin.take_sent().is_empty());
}

#[test]
fn a_player_with_timeways_answers_a_call_of_the_guild() {
    let (ada, corvin) = ada_and_corvin();

    ada.run("ns.PlayerTasks.Call()");
    deliver(&ada, &corvin);
    deliver(&corvin, &ada);

    let recipients: Vec<String> =
        ada.eval("local out = {} for _, r in ipairs(ns.PlayerTasks.Recipients()) do table.insert(out, r.name .. ' ' .. r.relation) end return out");
    assert_eq!(recipients, ["Corvin-Stormrage guild"]);
}

#[test]
fn a_stranger_gets_no_answer() {
    let (_ada, corvin) = ada_and_corvin();

    corvin.hear("Timeways", "1:1:1:1;hello", "WHISPER", "Mallory-Stormrage");

    assert!(corvin.take_sent().is_empty());
}

#[test]
fn a_message_from_yourself_is_ignored() {
    let ada = Player::new("Ada");
    ada.in_guild_with(&[]);

    ada.hear("Timeways", "1:1:1:1;hello", "GUILD", "Ada-Stormrage");

    assert!(ada.take_sent().is_empty());
}

#[test]
fn a_whisper_to_a_player_who_is_offline_waits_until_they_come_online() {
    let (ada, corvin) = ada_and_corvin();
    ada.run("wow.guild[2].online = false");

    ada.run("ns.TaskChannel.Whisper('Corvin-Stormrage', { type = 'here' })");
    let while_offline = ada.take_sent().len();
    ada.run("wow.guild[2].online = true");
    for _ in 0..20 {
        ada.tick();
    }

    assert_eq!(while_offline, 0);
    assert_eq!(ada.take_sent().len(), 1);
    drop(corvin);
}

#[test]
fn a_burst_past_the_limit_of_the_game_waits_for_the_next_seconds() {
    let (ada, _corvin) = ada_and_corvin();

    ada.run("for n = 1, 12 do ns.TaskChannel.Whisper('Corvin-Stormrage', { type = 'here' }) end");
    let at_once = ada.take_sent().len();
    ada.tick();
    ada.tick();

    assert_eq!(at_once, 8);
    assert_eq!(ada.take_sent().len(), 2);
}

#[test]
fn a_message_that_the_game_throttles_waits() {
    let (ada, _corvin) = ada_and_corvin();
    ada.run("wow.sendResult = 3");

    ada.run("ns.TaskChannel.Whisper('Corvin-Stormrage', { type = 'here' })");
    ada.run("wow.sendResult = 0");
    let waiting: usize = ada.eval("ns.TaskChannel.Waiting()");
    ada.tick();

    assert_eq!(waiting, 1);
    assert_eq!(ada.take_sent().len(), 1);
}

#[test]
fn a_peer_that_floods_loses_its_parts_and_nobody_elses() {
    let (ada, corvin) = ada_and_corvin();
    let bram = Player::new("Bram");
    corvin.in_guild_with(&[&ada, &bram]);

    for _ in 0..40 {
        corvin.hear("Timeways", "1:1:1:1;hello", "GUILD", &ada.full_name());
    }
    let mut answers_to_ada = 0;
    for _ in 0..30 {
        answers_to_ada += corvin.take_sent().len();
        corvin.tick();
    }
    corvin.hear("Timeways", "1:1:1:1;hello", "GUILD", &bram.full_name());

    assert_eq!(answers_to_ada, 24);
    let answers: Vec<Option<String>> = corvin
        .take_sent()
        .into_iter()
        .map(|sent| sent.target)
        .collect();
    assert_eq!(answers, [Some(bram.full_name())]);
}

#[test]
fn a_part_from_before_a_reload_never_joins_a_new_message() {
    let game = Game::new();

    let whole: Option<String> = game.eval(
        "local collector = ns.TaskChunks.NewCollector()
         ns.TaskChunks.Add(collector, 'Ada-Stormrage', '1:1:2:ab', 0)
         return ns.TaskChunks.Add(collector, 'Ada-Stormrage', '1:2:2:cd', 61)",
    );

    assert_eq!(whole, None);
}

#[test]
fn the_parts_that_the_game_refuses_wait_and_go_later() {
    let (ada, corvin) = ada_and_corvin();
    ada.run(&format!(
        "local offer = {OFFER}
         offer.text = string.rep('Gregor walks. ', 28)
         wow.sendResults = {{ 0, 3 }}
         ns.TaskChannel.Whisper('Corvin-Stormrage', offer)"
    ));
    let first = deliver(&ada, &corvin);

    ada.tick();
    deliver(&ada, &corvin);

    assert_eq!(first, 1);
    let got: bool = corvin.eval("next(ns.TaskStore.Data().received) ~= nil");
    assert!(got);
}

#[test]
fn a_task_message_outside_a_whisper_is_ignored() {
    let (ada, corvin) = ada_and_corvin();

    corvin.hear(
        "Timeways",
        "1:1:1:1;offer;k1;Hi;Go.;;1;npc;Renee;1",
        "GUILD",
        &ada.full_name(),
    );

    let got: bool = corvin.eval("next(ns.TaskStore.Data().received) ~= nil");
    assert!(!got);
}

#[test]
fn a_call_in_a_group_finder_group_goes_to_the_instance_channel() {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    ada.run("wow.instanceGroup = true");

    ada.run("ns.PlayerTasks.Call()");

    let channels: Vec<String> = ada
        .take_sent()
        .into_iter()
        .map(|sent| sent.channel)
        .collect();
    assert_eq!(channels, ["INSTANCE_CHAT", "GUILD"]);
}

#[test]
fn a_call_that_the_game_refuses_for_good_is_dropped() {
    let (ada, _corvin) = ada_and_corvin();
    ada.run("wow.sendResult = 10");

    ada.run("ns.PlayerTasks.Call()");

    let waiting: usize = ada.eval("ns.TaskChannel.Waiting()");
    assert_eq!(waiting, 0);
}

#[test]
fn a_whisper_to_a_player_from_a_party_that_ended_still_goes() {
    let ada = Player::new("Ada");
    let bram = Player::new("Bram");
    ada.in_party_with(&bram);
    ada.leave_party();

    ada.run("ns.TaskChannel.Whisper('Bram-Stormrage', { type = 'here' })");

    assert_eq!(deliver(&ada, &bram), 1);
}

#[test]
fn a_whisper_that_finds_its_player_offline_tries_again_later() {
    let ada = Player::new("Ada");
    ada.run("wow.offline['Bram-Stormrage'] = true");

    ada.run("ns.TaskChannel.Whisper('Bram-Stormrage', { type = 'here' })");
    ada.run("wow.offline = {}");
    ada.tick();
    let soon = ada.take_sent().len();
    ada.run("wow.now = wow.now + 300");
    ada.tick();

    assert_eq!(soon, 0);
    assert_eq!(ada.take_sent().len(), 1);
}

#[test]
fn a_guild_roster_that_shows_a_member_offline_is_asked_again() {
    let (ada, _corvin) = ada_and_corvin();
    ada.run("wow.guild[2].listed = false");

    ada.run("ns.TaskChannel.Whisper('Corvin-Stormrage', { type = 'here' })");
    let at_once = ada.take_sent().len();
    ada.tick();
    ada.tick();

    assert_eq!(at_once, 0);
    assert_eq!(ada.take_sent().len(), 1);
}

#[test]
fn a_player_of_a_realm_with_a_space_in_its_name_is_found_in_the_group() {
    let ada = Player::new("Ada");
    ada.run("wow.units.party1 = { name = 'Bram', realm = 'Argent Dawn', player = true }");

    let unit: Option<String> = ada.eval("ns.TaskPeople.GroupUnit('Bram-ArgentDawn')");

    assert_eq!(unit.as_deref(), Some("party1"));
}

#[test]
fn the_logged_channel_of_the_test_game_refuses_what_the_game_refuses() {
    let game = Game::new();

    let results: Vec<u32> = game.eval(
        r"local results = {}
         for _, text in ipairs({ 'Plain café ✓', 'a|b', 'a\\b', 'a\nb', 'a\127b', 'Caf\195',
                 '\169t\195\169', '\229\141\141', '\239\191\191', '\237\160\128' }) do
             results[#results + 1] = C_ChatInfo.SendAddonMessageLogged('Timeways', text, 'GUILD')
         end
         return results",
    );

    assert_eq!(results, [0, 2, 2, 2, 2, 2, 2, 2, 2, 2]);
}
