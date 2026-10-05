//! The MSP check against `MyRolePlay` (docs/plans/msp.md, section 4): the MSP code that
//! `MyRolePlay` ships (`LibMSP`, Chomp, `ChatThrottleLib`) talks to Timeways in the fake game.
//! These tests need the pinned release: run `scripts/fetch-mrp.sh` first, then
//! `cargo test -p addon-tests --test msp_mrp -- --ignored`. CI does both.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use common::Game;
use players::Player;
use std::path::PathBuf;

const NEEDS_MRP: &str = "needs MyRolePlay: run scripts/fetch-mrp.sh";

/// The files of `MyRolePlay` that speak MSP, in the order of its TOC.
const LIBRARIES: [&str; 7] = [
    "Libs/LibStub/LibStub.lua",
    "Libs/CallbackHandler-1.0/CallbackHandler-1.0.lua",
    "Libs/ChatThrottleLib/ChatThrottleLib.lua",
    "Libs/Chomp/Internal.lua",
    "Libs/Chomp/Public.lua",
    "Libs/Chomp/StringManip.lua",
    "Libs/LibMSP/LibMSP.lua",
];

/// The script at the end of Chomp.xml, which ends the load of Chomp.
const CHOMP_LOADED: &str = r#"local Chomp = LibStub:GetLibrary("Chomp", true)
    if Chomp and Chomp.Internal then Chomp.Internal.LOADING = nil end"#;

fn mrp_folder() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/mrp/MyRolePlay");
    assert!(root.is_dir(), "{NEEDS_MRP}");
    root
}

/// Loads the MSP code of `MyRolePlay` into the game, before the login, and sets the global
/// that `MyRolePlay.lua` sets.
fn load_mrp(game: &Game) {
    game.lua
        .load(include_str!("mrp/shim.lua"))
        .set_name("shim.lua")
        .exec()
        .unwrap();
    let folder = mrp_folder();
    for file in LIBRARIES {
        if file == "Libs/LibMSP/LibMSP.lua" {
            game.run(CHOMP_LOADED);
        }
        let source = std::fs::read_to_string(folder.join(file)).unwrap();
        game.lua
            .load(source.as_str())
            .set_name(file)
            .call::<()>(("MyRolePlay", game.lua.create_table().unwrap()))
            .unwrap();
    }
    game.run("msp_RPAddOn = 'MyRolePlay'; wow.strictFrames = false");
}

/// Fills `msp.my` as `mrp:SetCurrentProfile` of Profile.lua does, from a Lua table.
fn set_mrp_profile(game: &Game, fields: &str) {
    game.run(&format!(
        "wipe(msp.my)
         for field, text in pairs({fields}) do msp.my[field] = text end
         msp.my.VP = tostring(msp.protocolversion)
         msp.my.VA = 'MyRolePlay/12.1.0.657'
         msp.my.GU = UnitGUID('player')
         msp.my.GS = tostring(UnitSex('player'))
         msp.my.GC = select(2, UnitClass('player'))
         msp.my.GR = select(2, UnitRace('player'))
         msp.my.TR = '0'
         msp:Update()"
    ));
}

const BO_PROFILE: &str = "{ NA = 'Bo Ironhand', NT = 'Smith of Ironforge', CU = 'At the forge.',
    DE = 'Broad, with a red beard.', AG = 'Ninety.', MO = 'Hammer first.', HB = 'Dun Morogh',
    HI = 'Forty years at the anvil.' }";

/// Bo, a player with `MyRolePlay` and no Timeways.
fn bo() -> Player {
    let game = Game::without_addon();
    game.run(
        "wow.units.player = { name = 'Bo', player = true, guid = 'Player-1-Bo' }
         wow.realm = 'Stormrage'",
    );
    load_mrp(&game);
    game.run("wow.Login()");
    set_mrp_profile(&game, BO_PROFILE);
    Player { game, name: "Bo" }
}

const ADA_SHEET: &str = r#"{"type":"journal","page":0,"pages":1,"hero":{"sheet":[{"field":"origin","text":"Lordaeron"},{"field":"background","text":"A farm girl who took up the sword."},{"field":"goal","text":"Find my brother."},{"field":"title","text":"Keeper of the Flame"},{"field":"currently","text":"Reading by the fire."},{"field":"appearance","text":"Tall, with a scar ~ and a | mark."},{"field":"age","text":"Thirty winters."},{"field":"motto","text":"Light and steel."}],"entries":[]},"hero_refused":null}"#;

/// Ada with Timeways, her sheet from the desktop, and the switch on.
fn ada() -> Player {
    let ada = Player::new("Ada");
    ada.game.reply(ADA_SHEET);
    ada.run("ns.MspProfile.SetSharing(true)");
    ada
}

/// A second of play: the timers of both sides, and the send queue of `ChatThrottleLib`.
fn second(player: &Player) {
    player.run("wow.now = wow.now + 1; wow.RunTickers(); if wow.Update then wow.Update(1) end");
}

/// Carries every whisper that `from` sent to `to`, on its channel, with `to` as the target,
/// as the game names it in the event.
fn carry(from: &Player, to: &Player) -> usize {
    let fire: mlua::Function = to.eval(
        "return function(event, prefix, text, sender, target)
             wow.Fire(event, prefix, text, 'WHISPER', sender, target, 0, 0, '', 0)
         end",
    );
    let sent = from.take_sent();
    let mut count = 0;
    for message in sent {
        if message.target.as_deref() == Some(to.full_name().as_str()) {
            let arguments = (
                message.event,
                message.prefix,
                message.text,
                from.full_name(),
                to.full_name(),
            );
            fire.call::<()>(arguments).unwrap();
            count += 1;
        }
    }
    count
}

/// Carries the messages both ways until three quiet seconds pass.
fn talk(a: &Player, b: &Player) {
    let mut quiet = 0;
    for _ in 0..120 {
        let carried = carry(a, b) + carry(b, a);
        quiet = if carried == 0 { quiet + 1 } else { 0 };
        if quiet >= 3 {
            return;
        }
        second(a);
        second(b);
    }
    panic!("the two players never stopped talking");
}

const SHARED: [&str; 8] = ["NA", "NT", "CU", "DE", "AG", "MO", "HB", "HI"];

/// The fields of `name` that `MyRolePlay` keeps, as `CODE=text`.
fn mrp_fields(bo: &Player, name: &str) -> Vec<String> {
    let read: mlua::Function =
        bo.eval("return function(name, code) return tostring(msp.char[name].field[code]) end");
    SHARED
        .iter()
        .map(|code| format!("{code}={}", read.call::<String>((name, *code)).unwrap()))
        .collect()
}

fn ask_from_mrp(bo: &Player) {
    bo.run("msp:Request('Ada-Stormrage', { 'NA', 'NT', 'CU', 'DE', 'AG', 'MO', 'HB', 'HI' })");
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn myroleplay_gets_each_shared_field_of_timeways() {
    let ada = ada();
    let bo = bo();

    ask_from_mrp(&bo);
    talk(&ada, &bo);

    assert_eq!(
        mrp_fields(&bo, "Ada-Stormrage"),
        [
            "NA=Ada",
            "NT=Keeper of the Flame",
            "CU=Reading by the fire.",
            "DE=Tall, with a scar ~ and a | mark.",
            "AG=Thirty winters.",
            "MO=Light and steel.",
            "HB=Lordaeron",
            "HI=A farm girl who took up the sword.",
        ]
    );
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn myroleplay_never_gets_the_goal_of_timeways() {
    let ada = ada();
    let bo = bo();

    bo.run("msp:Request('Ada-Stormrage', { 'GO', 'PE', 'CO' })");
    talk(&ada, &bo);

    let fields: Vec<String> = bo.eval(
        "local out = {}
         for code, text in pairs(msp.char['Ada-Stormrage'].field) do
             if text:find('brother') then table.insert(out, code) end
         end
         return out",
    );
    assert!(fields.is_empty(), "{fields:?}");
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn timeways_gets_each_field_of_myroleplay() {
    let corvin = ada();
    let bo = bo();

    corvin.run("ns.Msp.Ask('Bo', { 'NA', 'NT', 'CU', 'DE', 'AG', 'MO', 'HB', 'HI' })");
    talk(&corvin, &bo);

    let fields: Vec<String> = corvin.eval(
        "local out = {}
         local fields = ns.Msp.FieldsOf('Bo-Stormrage')
         for _, code in ipairs({ 'NA', 'NT', 'CU', 'DE', 'AG', 'MO', 'HB', 'HI' }) do
             table.insert(out, code .. '=' .. tostring(fields[code]))
         end
         return out",
    );
    assert_eq!(
        fields,
        [
            "NA=Bo Ironhand",
            "NT=Smith of Ironforge",
            "CU=At the forge.",
            "DE=Broad, with a red beard.",
            "AG=Ninety.",
            "MO=Hammer first.",
            "HB=Dun Morogh",
            "HI=Forty years at the anvil.",
        ]
    );
    let lines: Vec<String> = corvin.eval(
        "wow.units.mouseover = { name = 'Bo', player = true, guid = 'Player-1-Bo' }
         return wow.ShowTooltip('mouseover')",
    );
    assert_eq!(lines, ["Bo Ironhand, Smith of Ironforge"]);
}

/// The MSP commands that this game sent since the last call, from every part.
fn msp_commands(player: &Player) -> Vec<String> {
    let sent = player.take_sent();
    let mut commands = Vec::new();
    for message in sent.iter().filter(|message| message.prefix == "MSP2") {
        let body = &message.text[12..];
        commands.extend(body.split('`').map(str::to_string));
    }
    commands
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn myroleplay_gets_only_the_changed_field_of_timeways_again() {
    let ada = ada();
    let bo = bo();
    ask_from_mrp(&bo);
    talk(&ada, &bo);

    ada.run("ns.Hero.Set('age', 'Forty winters.')");
    ada.run("wow.now = wow.now + 31");
    bo.run("wow.now = wow.now + 31");
    msp_commands(&ada);
    bo.run("msp:Request('Ada-Stormrage', { 'DE', 'AG', 'MO' })");
    carry(&bo, &ada);
    second(&ada);
    let replies = msp_commands(&ada);

    assert!(
        replies.iter().any(|reply| reply.starts_with("!DE")),
        "{replies:?}"
    );
    assert!(
        replies.iter().any(|reply| reply.starts_with("!MO")),
        "{replies:?}"
    );
    assert!(
        replies
            .iter()
            .any(|reply| reply.ends_with(":Forty winters.")),
        "{replies:?}"
    );
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn myroleplay_keeps_a_changed_field_of_timeways() {
    let ada = ada();
    let bo = bo();
    ask_from_mrp(&bo);
    talk(&ada, &bo);

    ada.run("ns.Hero.Set('age', 'Forty winters.')");
    ada.run("wow.now = wow.now + 31");
    bo.run("wow.now = wow.now + 31");
    ask_from_mrp(&bo);
    talk(&ada, &bo);

    let fields = mrp_fields(&bo, "Ada-Stormrage");
    assert_eq!(fields[4], "AG=Forty winters.");
    assert_eq!(fields[3], "DE=Tall, with a scar ~ and a | mark.");
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn timeways_gets_only_the_changed_field_of_myroleplay_again() {
    let corvin = ada();
    let bo = bo();
    corvin.run("ns.Msp.Ask('Bo', { 'DE', 'AG' })");
    talk(&corvin, &bo);

    set_mrp_profile(
        &bo.game,
        "{ NA = 'Bo Ironhand', DE = 'Broad, with a grey beard.', AG = 'Ninety.' }",
    );
    corvin.run("wow.now = wow.now + 31; ns.Msp.Ask('Bo', { 'DE', 'AG' })");
    bo.run("wow.now = wow.now + 31");
    carry(&corvin, &bo);
    for _ in 0..5 {
        second(&bo);
    }
    let replies = msp_commands(&bo);

    assert!(
        replies.iter().any(|reply| reply.starts_with("!AG")),
        "{replies:?}"
    );
    assert!(
        replies.iter().any(|reply| reply.contains("grey beard")),
        "{replies:?}"
    );
}

#[test]
#[ignore = "needs MyRolePlay: run scripts/fetch-mrp.sh"]
fn timeways_imports_the_own_profile_of_myroleplay() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");
    load_mrp(&game);
    game.run("wow.Login()");
    set_mrp_profile(
        &game,
        "{ NA = '|cff99ccffAda Brightwater|r', NT = 'Keeper of the Flame', CU = 'Reading.',
           DE = 'Tall.\\nA scar.', AG = 'Thirty.', MO = 'Light and steel.', HB = 'Gilneas' }",
    );

    game.reply(r#"{"type":"journal","page":0,"pages":1,"hero":{"sheet":[{"field":"origin","text":"Lordaeron"}],"entries":[]}}"#);
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    let sets: Vec<(String, String)> = game
        .sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            timeways_story::input::Input::HeroSet { field, text, .. } => Some((field, text)),
            _ => None,
        })
        .collect();
    let expected = [
        ("title", "Keeper of the Flame"),
        ("currently", "Reading."),
        ("appearance", "Tall. A scar."),
        ("age", "Thirty."),
        ("motto", "Light and steel."),
    ];
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(field, text)| ((*field).to_string(), (*text).to_string()))
        .collect();
    assert_eq!(sets, expected);
    assert_eq!(
        game.eval::<String>("ns.MspProfile.State()"),
        "From MyRolePlay"
    );
}
