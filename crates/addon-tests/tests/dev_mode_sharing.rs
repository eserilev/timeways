//! While dev mode is on, the addon talks to no real player: nothing goes on the wire, and a
//! real player's message is ignored (GAMEPLAY.md 5.15). Fake players keep working.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, ada_and_corvin, deliver, exchange};

const ON_LINE: &str = "|cffc8a064Timeways|r: Dev mode is on. Nothing is shared with other players.";
const PROFILE_LINE: &str = "Sharing is off in dev mode.";

/// A journal page with a sheet, and the dev mark of the desktop.
fn journal(dev: bool) -> String {
    format!(
        r#"{{"type":"journal","page":0,"pages":1,"dev":{dev},"hero":{{"sheet":[{{"field":"title","text":"Keeper of the Flame"}}],"entries":[]}},"hero_refused":null}}"#
    )
}

fn dev_on(player: &Player) {
    player.game.reply(&journal(true));
}

fn dev_off(player: &Player) {
    player.game.reply(&journal(false));
}

fn twdev(player: &Player, message: &str) {
    let slash: mlua::Function = player.eval("return function(m) wow.Slash('/twdev', m) end");
    slash.call::<()>(message).unwrap();
}

fn run_timers(player: &Player) {
    for _ in 0..3 {
        player.run(
            "local timers = wow.after
             wow.after = {}
             for _, timer in ipairs(timers) do timer.callback() end",
        );
    }
}

fn give_corvin_a_quest(ada: &Player) {
    ada.run(
        "ns.PlayerTasks.Give({ title = 'Wolf Pelts', text = '', reward = '', steps = {
             { kind = 'kill', target = 'Prowler', count = 1 } } }, 'Corvin-Stormrage')",
    );
}

fn tick_a_minute(player: &Player) {
    for _ in 0..60 {
        player.tick();
    }
}

/// Bo, a player with a roleplay addon, asks for the title.
fn bo_asks_for_the_title(player: &Player) {
    player.hear("MSP2", "00A001001001?NT", "WHISPER", "Bo-Stormrage");
}

fn profile_rows(player: &Player) -> Vec<String> {
    player.run("ns.Journal.Select('hero', 'profile')");
    player.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('hero').cards.rows) do
             local action = row.action and (' [' .. row.action.label .. ']') or ''
             table.insert(out, row.text .. action)
         end
         return out",
    )
}

#[test]
fn in_dev_mode_a_call_a_quest_and_a_story_send_nothing() {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    dev_on(&ada);

    ada.run("ns.PlayerTasks.Call()");
    give_corvin_a_quest(&ada);
    ada.run("ns.PlayerStories.Ask('Corvin-Stormrage', function() end)");
    ada.run("ns.PlayerStories.Send('Corvin-Stormrage', 'The Bridge', 'You held the line.')");
    tick_a_minute(&ada);

    assert!(ada.eval::<bool>("return #wow.addonSent == 0"));
}

#[test]
fn in_dev_mode_msp_answers_nobody_and_asks_nobody() {
    let ada = Player::new("Ada");
    ada.run("ns.MspProfile.SetSharing(true)");
    dev_on(&ada);

    bo_asks_for_the_title(&ada);
    ada.run("wow.units.mouseover = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");
    ada.game
        .eval::<Vec<String>>("return wow.ShowTooltip('mouseover')");
    tick_a_minute(&ada);

    assert!(ada.eval::<bool>("return #wow.addonSent == 0"));
}

#[test]
fn in_dev_mode_a_quest_from_a_real_player_is_ignored() {
    let (ada, corvin) = ada_and_corvin();
    dev_on(&corvin);

    give_corvin_a_quest(&ada);
    exchange(&ada, &corvin);

    assert!(corvin.eval::<bool>("return #ns.PlayerTasks.Received() == 0"));
}

#[test]
fn in_dev_mode_the_profile_of_a_real_player_is_ignored() {
    let ada = Player::new("Ada");
    ada.run("ns.MspProfile.SetSharing(true)");
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    dev_on(&corvin);

    let text: String = ada
        .eval("return select(1, ns.MspWire.Tooltip({ NA = 'Ada', NT = 'Keeper of the Flame' }))");
    let parts: Vec<String> = ada.eval(&format!("return ns.MspParts.Split({text:?}, 7)"));
    for part in parts {
        corvin.hear_logged("MSP2", &part, "WHISPER", "Ada-Stormrage");
    }

    assert!(corvin.eval::<bool>("return ns.Msp.TooltipLine('Ada-Stormrage') == nil"));
}

#[test]
fn a_quest_that_waits_for_a_real_player_never_goes_after_dev_mode_ends() {
    let (ada, corvin) = ada_and_corvin();
    ada.run(
        "wow.guild = { { name = 'Corvin-Stormrage', online = false } }
         wow.Fire('GUILD_ROSTER_UPDATE', false)",
    );
    dev_on(&ada);
    give_corvin_a_quest(&ada);
    let waited: usize = ada.eval("ns.TaskChannel.Waiting()");

    dev_off(&ada);
    ada.in_guild_with(&[&corvin]);
    tick_a_minute(&ada);
    deliver(&ada, &corvin);

    assert_eq!(waited, 1);
    assert!(corvin.eval::<bool>("return #ns.PlayerTasks.Received() == 0"));
}

#[test]
fn the_share_setting_survives_dev_mode_and_sharing_comes_back_after_it() {
    let ada = Player::new("Ada");
    ada.run("ns.MspProfile.SetSharing(true)");
    dev_on(&ada);
    dev_off(&ada);

    bo_asks_for_the_title(&ada);
    ada.tick();

    assert!(ada.eval::<bool>("return TimewaysProfile.share"));
    let sent: Vec<String> = ada.eval(
        "local texts = {} for _, m in ipairs(wow.addonSent) do texts[#texts + 1] = m.text end return texts",
    );
    assert!(
        sent.iter().any(|text| text.contains("Keeper of the Flame")),
        "{sent:?}"
    );
}

#[test]
fn a_profile_edited_in_dev_mode_stays_out_of_the_saved_copy() {
    let ada = Player::new("Ada");
    dev_off(&ada);
    ada.run("ns.MspProfile.SetSharing(true)");

    dev_on(&ada);
    ada.game.reply(
        r#"{"type":"journal","page":0,"pages":1,"dev":true,"hero":{"sheet":[{"field":"title","text":"Fake Title"}],"entries":[]},"hero_refused":null}"#,
    );
    ada.run("ns.MspProfile.Edited('motto', 'Fake motto.')");

    let title: String = ada.eval("return TimewaysProfile.fields.NT");
    assert_eq!(title, "Keeper of the Flame");
    assert!(ada.eval::<bool>("return TimewaysProfile.fields.MO == nil"));
}

#[test]
fn dev_mode_says_once_that_nothing_is_shared() {
    let ada = Player::new("Ada");

    dev_on(&ada);
    dev_on(&ada);

    let lines: Vec<String> = ada.printed();
    assert_eq!(lines.iter().filter(|line| *line == ON_LINE).count(), 1);
}

#[test]
fn the_profile_page_says_sharing_is_off_in_dev_mode_in_place_of_the_switch() {
    let ada = Player::new("Ada");
    ada.run("ns.MspProfile.SetSharing(true)");
    dev_on(&ada);
    ada.run("wow.Slash('/hero', '')");

    let on = profile_rows(&ada);
    dev_off(&ada);
    let off = profile_rows(&ada);

    assert_eq!(on[0], PROFILE_LINE);
    assert_eq!(
        off[0],
        "Players with roleplay addons like Total RP 3 see this. [Stop sharing]"
    );
}

#[test]
fn fake_players_still_get_and_send_messages_in_dev_mode() {
    let ada = Player::new("Ada");
    ada.run("ns.MspProfile.SetSharing(true)");
    dev_on(&ada);

    twdev(&ada, "peer Kobee near");
    ada.run("ns.PlayerTasks.Call()");
    run_timers(&ada);
    twdev(&ada, "msp Kobee / Keeper of the Flame");

    let recipients: Vec<String> = ada.eval(
        "local names = {} for _, r in ipairs(ns.PlayerTasks.Recipients()) do names[#names + 1] = r.name end return names",
    );
    assert_eq!(recipients, ["Kobee-Devrealm"]);
    let line: String = ada.eval("return ns.Msp.TooltipLine('Kobee-Devrealm')");
    assert_eq!(line, "Kobee of the Reef, Keeper of the Flame");
    assert!(ada.eval::<bool>("return #wow.addonSent == 0"));
}

/// The names of the game that send a message to other players. Each send goes through
/// `ToPlayers.lua`, so dev mode can stop it there.
const SENDS_OF_THE_GAME: &[&str] = &[
    "C_ChatInfo.Send",
    "C_ChatInfo[",
    "= C_ChatInfo",
    "C_ChatInfo.PerformEmote(",
    "SendChatMessage",
    "DoEmote",
    "BNSend",
    "C_BattleNet.Send",
    "C_Club.Send",
    "ChatThrottleLib",
];

#[test]
fn every_message_to_other_players_goes_through_the_dev_mode_gate() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../addon/Timeways");
    let mut leaks = Vec::new();

    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let is_lua = path.extension().is_some_and(|extension| extension == "lua");
        if !is_lua || name == "ToPlayers.lua" {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        for send in SENDS_OF_THE_GAME {
            if source.contains(send) {
                leaks.push(format!("{name}: {send}"));
            }
        }
    }

    assert!(leaks.is_empty(), "send through ns.ToPlayers: {leaks:?}");
}

fn waiting_stories(player: &Player) -> usize {
    player.eval("return #ns.PlayerStories.Waiting()")
}

/// Corvin tells Ada a story, as the scroll does.
fn corvin_tells_ada_a_story(ada: &Player, corvin: &Player) {
    ada.in_party_with(corvin);
    corvin.in_party_with(ada);
    corvin.run(
        "ns.PlayerStories.Ask('Ada-Stormrage', function(room)
             if room == 'open' then ns.PlayerStories.Send('Ada-Stormrage', 'The Bridge', 'You held it.') end
         end)",
    );
    exchange(corvin, ada);
}

#[test]
fn a_story_of_a_real_player_stays_out_of_a_dev_world_and_waits_after_it() {
    let (ada, corvin) = ada_and_corvin();
    corvin_tells_ada_a_story(&ada, &corvin);

    dev_on(&ada);
    let in_dev = waiting_stories(&ada);
    dev_off(&ada);

    assert_eq!(in_dev, 0);
    assert_eq!(waiting_stories(&ada), 1);
}

#[test]
fn a_story_of_a_fake_player_never_reaches_the_real_box() {
    let ada = Player::new("Ada");
    dev_on(&ada);
    twdev(&ada, "peer Kobee story The Bridge");
    let in_dev = waiting_stories(&ada);

    dev_off(&ada);

    assert_eq!(in_dev, 1);
    assert_eq!(waiting_stories(&ada), 0);
}

#[test]
fn a_quest_of_a_fake_player_never_reaches_the_real_journal() {
    let ada = Player::new("Ada");
    dev_on(&ada);
    twdev(&ada, "peer Kobee quest");
    let in_dev: usize = ada.eval("return #ns.PlayerTasks.Received()");

    dev_off(&ada);

    assert_eq!(in_dev, 1);
    assert!(ada.eval::<bool>("return #ns.PlayerTasks.Received() == 0"));
}
