//! MSP, the profiles of roleplay addons (GAMEPLAY.md 3.7.1): the wire of `LibMSP` and Chomp,
//! the switch, the answers, the requests, the tooltip, and the import.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

fn text(game: &Game, code: &str) -> String {
    game.eval(code)
}

#[test]
fn the_version_is_the_crc32c_of_the_text_in_upper_case_hex() {
    let game = Game::new();

    assert_eq!(text(&game, "ns.MspWire.Version('123456789')"), "E3069283");
    assert_eq!(text(&game, "ns.MspWire.Version('a')"), "C1D04330");
    assert_eq!(text(&game, "ns.MspWire.Version('')"), "");
}

#[test]
fn a_field_carries_its_version_and_an_empty_field_is_bare() {
    let game = Game::new();

    assert_eq!(
        text(&game, "ns.MspWire.Field('NA', '123456789')"),
        "NAE3069283:123456789"
    );
    assert_eq!(text(&game, "ns.MspWire.Field('NA', '')"), "NA");
    assert_eq!(text(&game, "ns.MspWire.Field('NA', nil)"), "NA");
}

#[test]
fn a_backtick_in_a_text_becomes_a_quote() {
    let game = Game::new();

    let field = text(&game, "ns.MspWire.Field('MO', 'It`s me')");

    assert!(field.ends_with(":It's me"), "{field}");
}

#[test]
fn the_tooltip_lists_its_fields_in_order_and_ends_with_its_version() {
    let game = Game::new();

    let (block, version): (String, String) = game
        .lua
        .load("ns.MspWire.Tooltip({ VP = '3', NA = 'Ada', NT = 'Lady' })")
        .eval()
        .unwrap();

    let contents = "VP:3`VA`NA:Ada`NH`NI`NT:Lady`RA`CU`FR`FC";
    assert_eq!(block, format!("{contents}`TT{version}"));
    let expected: String = game.eval(&format!("ns.MspWire.Version('{contents}')"));
    assert_eq!(version, expected);
}

/// The commands of a message, as `kind field version text`.
fn commands(game: &Game, message: &str) -> Vec<String> {
    let parse: mlua::Function = game.eval(
        "return function(message)
             local out = {}
             for _, c in ipairs(ns.MspWire.Commands(message)) do
                 table.insert(out, table.concat({ c.kind, c.field, c.version, c.text or '-' }, ' '))
             end
             return out
         end",
    );
    parse.call(message).unwrap()
}

#[test]
fn a_message_holds_requests_fields_and_same_versions() {
    let game = Game::new();

    let parsed = commands(&game, "?NA`?DE1A2B`NA5F:Mary Sue`DE`!HI0C0FFEE`TTABC");

    assert_eq!(
        parsed,
        [
            "request NA  -",
            "request DE 1A2B -",
            "field NA 5F Mary Sue",
            "field DE  -",
            "same HI 0C0FFEE -",
            "field TT ABC -",
        ]
    );
}

#[test]
fn a_broken_command_is_skipped_and_a_version_of_zero_is_none() {
    let game = Game::new();

    let parsed = commands(&game, "na`N`?NAxyz`#NA`?DE0`CU:``");

    assert_eq!(parsed, ["request DE  -", "field CU  -"]);
}

#[test]
fn a_logged_text_escapes_four_letters_and_decodes_only_those() {
    let game = Game::new();

    let escaped = text(&game, r"ns.MspWire.Escape('a|b\\c~d\ne')");
    let decoded = text(&game, "ns.MspWire.Unescape('~7C~5C~7E~0A~41')");

    assert_eq!(escaped, "a~7Cb~5Cc~7Ed~0Ae");
    assert_eq!(decoded, "|\\~\n~41");
}

/// The parts of a text, from session 5.
fn split(game: &Game, code: &str) -> Vec<String> {
    game.eval(&format!("ns.MspParts.Split({code}, 5)"))
}

#[test]
fn a_part_starts_with_the_header_of_chomp() {
    let game = Game::new();

    let parts = split(&game, "'?TT'");

    assert_eq!(parts, ["00A005001001?TT"]);
}

#[test]
fn a_long_text_splits_into_parts_of_at_most_255_bytes_and_joins_again() {
    let game = Game::new();

    let parts = split(&game, "string.rep('\\195\\169', 300)");

    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|part| part.len() <= 255));
    assert!(parts[0].starts_with("00A005001003"), "{}", parts[0]);
    let whole: String = game.eval(
        "local collector = ns.MspParts.NewCollector()
         local whole
         for _, part in ipairs(ns.MspParts.Split(string.rep('\\195\\169', 300), 5)) do
             whole = ns.MspParts.Add(collector, 'Bo-Realm', part, false, 0)
         end
         return whole",
    );
    assert_eq!(whole, "é".repeat(300));
}

#[test]
fn a_split_never_cuts_an_escape_in_two() {
    let game = Game::new();

    let ok: bool = game.eval(
        "local text = ns.MspWire.Escape(string.rep('|', 200))
         for _, part in ipairs(ns.MspParts.Split(text, 1)) do
             local body = part:sub(13)
             if body:find('~%x?$') then return false end
         end
         return true",
    );

    assert!(ok);
}

#[test]
fn a_text_past_64_parts_has_no_parts() {
    let game = Game::new();

    let none: bool = game.eval("return ns.MspParts.Split(string.rep('a', 64 * 243 + 1), 1) == nil");

    assert!(none);
}

/// Adds each part for Bo, and returns the whole text and whether it came logged.
fn collect(game: &Game, parts: &[(&str, bool)]) -> (Option<String>, Option<bool>) {
    let add: mlua::Function = game.eval(
        "collector = collector or ns.MspParts.NewCollector()
         return function(part, logged)
             return ns.MspParts.Add(collector, 'Bo-Realm', part, logged, 0)
         end",
    );
    let mut last = (None, None);
    for (part, logged) in parts {
        last = add.call((*part, *logged)).unwrap();
    }
    last
}

#[test]
fn a_part_with_no_header_or_a_flag_of_a_broadcast_is_dropped() {
    let game = Game::new();

    assert_eq!(collect(&game, &[("?TT", false)]), (None, None));
    assert_eq!(collect(&game, &[("01A001001001?TT", false)]), (None, None));
    assert_eq!(collect(&game, &[("00A001002001?TT", false)]), (None, None));
    assert_eq!(
        collect(&game, &[("00A001001001?TT", false)]),
        (Some("?TT".to_string()), Some(false))
    );
}

#[test]
fn a_logged_part_is_decoded_and_one_unlogged_part_makes_the_message_unlogged() {
    let game = Game::new();

    let logged = collect(&game, &[("00A001001001NA:a~7Cb", true)]);
    let mixed = collect(
        &game,
        &[("00A002001002NA:a", true), ("00A002002002~7Cb", false)],
    );

    assert_eq!(logged, (Some("NA:a|b".to_string()), Some(true)));
    assert_eq!(mixed, (Some("NA:a~7Cb".to_string()), Some(false)));
}

mod players;

use players::{LOGGED, Player};

/// A journal page with this sheet and nothing else.
fn sheet_reply(fields: &[(&str, &str)]) -> String {
    let sheet: Vec<String> = fields
        .iter()
        .map(|(field, text)| format!(r#"{{"field":"{field}","text":"{text}"}}"#))
        .collect();
    format!(
        r#"{{"type":"journal","page":0,"pages":1,"hero":{{"sheet":[{}],"entries":[]}},"hero_refused":null}}"#,
        sheet.join(",")
    )
}

const ADA_SHEET: &[(&str, &str)] = &[
    ("origin", "Lordaeron"),
    ("background", "A farm girl who took up the sword."),
    ("goal", "Find my brother."),
    ("traits", "Loud."),
    ("name", "Ada Brightwater"),
    ("title", "Keeper of the Flame"),
    ("currently", "Reading by the fire."),
    ("appearance", "Tall, with a scar."),
    ("age", "Thirty winters."),
    ("motto", "Light and steel."),
];

/// Ada with her sheet from the desktop, and the switch on.
fn sharing_ada() -> Player {
    let ada = Player::new("Ada");
    ada.game.reply(&sheet_reply(ADA_SHEET));
    ada.run("ns.MspProfile.SetSharing(true)");
    ada
}

/// The MSP messages that this game sent since the last call, as (logged, text), each one
/// joined from its parts and decoded.
fn msp_sent(player: &Player) -> Vec<(bool, String)> {
    let mut messages: Vec<(bool, String)> = Vec::new();
    let mut open: Option<(String, bool, String)> = None;
    for sent in player.take_sent() {
        if sent.prefix != "MSP2" {
            continue;
        }
        let logged = sent.event == LOGGED;
        let (header, body) = sent.text.split_at(12);
        let body = if logged {
            let unescape: mlua::Function = player.eval("ns.MspWire.Unescape");
            unescape.call::<String>(body).unwrap()
        } else {
            body.to_string()
        };
        let session = header[3..6].to_string();
        let last = header[6..9] == header[9..12];
        let text = match open.take() {
            Some((open_session, _, before)) if open_session == session => before + &body,
            _ => body,
        };
        if last {
            messages.push((logged, text));
        } else {
            open = Some((session, logged, text));
        }
    }
    messages
}

/// Bo, a player with a roleplay addon, whispers this MSP message to the player.
fn bo_asks(player: &Player, message: &str) {
    player.hear(
        "MSP2",
        &format!("00A001001001{message}"),
        "WHISPER",
        "Bo-Stormrage",
    );
}

#[test]
fn the_switch_is_off_by_default_and_then_no_request_gets_an_answer() {
    let ada = Player::new("Ada");
    ada.game.reply(&sheet_reply(ADA_SHEET));

    bo_asks(&ada, "?NA`?DE");
    ada.tick();

    assert!(msp_sent(&ada).is_empty());
    assert_eq!(ada.eval::<String>("ns.MspProfile.State()"), "Not shared");
}

#[test]
fn a_shared_profile_answers_each_shared_field_on_the_logged_channel() {
    let ada = sharing_ada();

    bo_asks(&ada, "?DE`?AG`?MO`?HB`?HI");
    ada.tick();

    let sent = msp_sent(&ada);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let (logged, text) = &sent[0];
    assert!(*logged);
    let fields = commands(&ada.game, text);
    let version = |text: &str| -> String { ada.eval(&format!("ns.MspWire.Version('{text}')")) };
    assert_eq!(
        fields,
        [
            format!(
                "field DE {} Tall, with a scar.",
                version("Tall, with a scar.")
            ),
            format!("field AG {} Thirty winters.", version("Thirty winters.")),
            format!("field MO {} Light and steel.", version("Light and steel.")),
            format!("field HB {} Lordaeron", version("Lordaeron")),
            format!(
                "field HI {} A farm girl who took up the sword.",
                version("A farm girl who took up the sword.")
            ),
        ]
    );
}

#[test]
fn the_tooltip_fields_come_together_with_the_version_of_the_tooltip() {
    let ada = sharing_ada();

    bo_asks(&ada, "?NA");
    ada.tick();

    let (_, text) = &msp_sent(&ada)[0];
    assert!(
        text.starts_with("VP:3`VA:Timeways/1`NA:Ada Brightwater`NH`NI`NT:Keeper of the Flame`RA`CU:Reading by the fire.`FR`FC`TT"),
        "{text}"
    );
}

#[test]
fn the_goal_bond_flaw_traits_and_notes_never_go_out() {
    let ada = sharing_ada();

    bo_asks(&ada, "?GO`?BO`?FL`?TR`?NO");
    ada.tick();

    let (_, text) = &msp_sent(&ada)[0];
    assert!(
        !text.contains("brother") && !text.contains("Loud"),
        "{text}"
    );
}

#[test]
fn an_unchanged_field_gets_not_changed_and_a_changed_one_comes_again() {
    let ada = sharing_ada();
    let old: String = ada.eval("ns.MspWire.Version('Thirty winters.')");

    bo_asks(&ada, &format!("?AG{old}"));
    ada.tick();
    let same = msp_sent(&ada);
    ada.run("ns.Hero.Set('age', 'Forty winters.')");
    ada.tick();
    ada.tick();
    ada.tick();
    ada.tick();
    ada.tick();
    ada.tick();
    bo_asks(&ada, &format!("?AG{old}"));
    ada.tick();
    let changed = msp_sent(&ada);

    assert_eq!(same, [(false, format!("!AG{old}"))]);
    assert_eq!(changed.len(), 1);
    assert!(
        changed[0].0 && changed[0].1.ends_with(":Forty winters."),
        "{changed:?}"
    );
}

#[test]
fn the_same_request_again_within_five_seconds_gets_no_answer() {
    let ada = sharing_ada();

    bo_asks(&ada, "?DE");
    bo_asks(&ada, "?DE");
    ada.tick();

    assert_eq!(msp_sent(&ada).len(), 1);
}

#[test]
fn a_request_from_yourself_or_a_part_with_no_header_gets_no_answer() {
    let ada = sharing_ada();

    ada.hear("MSP2", "00A001001001?DE", "WHISPER", "Ada-Stormrage");
    ada.hear("MSP2", "?DE", "WHISPER", "Bo-Stormrage");
    ada.tick();

    assert!(msp_sent(&ada).is_empty());
}

#[test]
fn stop_sharing_clears_the_saved_copy_and_answers_nothing() {
    let ada = sharing_ada();
    let saved: usize = ada.eval(
        "local count = 0 for _ in pairs(TimewaysProfile.fields) do count = count + 1 end return count",
    );

    ada.run("ns.MspProfile.SetSharing(false)");
    bo_asks(&ada, "?DE");
    ada.tick();

    assert_eq!(saved, 8);
    assert!(ada.eval::<bool>(
        "return next(TimewaysProfile.fields) == nil and TimewaysProfile.share == false"
    ));
    assert!(msp_sent(&ada).is_empty());
}

#[test]
fn the_saved_copy_holds_only_the_eight_shared_fields() {
    let ada = sharing_ada();

    let codes: Vec<String> = ada.eval(
        "local codes = {} for code in pairs(TimewaysProfile.fields) do table.insert(codes, code) end
         table.sort(codes) return codes",
    );

    assert_eq!(codes, ["AG", "CU", "DE", "HB", "HI", "MO", "NA", "NT"]);
}

#[test]
fn the_saved_copy_answers_while_the_desktop_is_off() {
    let ada = Player::new("Ada");
    ada.run("TimewaysProfile = { share = true, fields = { AG = 'Thirty winters.' } }");

    bo_asks(&ada, "?AG");
    ada.tick();

    let (_, text) = &msp_sent(&ada)[0];
    assert!(text.ends_with(":Thirty winters."), "{text}");
}

#[test]
fn a_broken_saved_profile_keeps_only_its_clean_fields() {
    let ada = Player::new("Ada");
    ada.run(
        "TimewaysProfile = { share = 'yes', fields = { AG = 'Thirty.', NA = 5, XX = 'odd',
             MO = 'a\\nb', NT = string.rep('a', 121) } }",
    );

    let state: String = ada.eval("ns.MspProfile.State()");
    let codes: Vec<String> = ada.eval(
        "local codes = {} for code in pairs(TimewaysProfile.fields) do table.insert(codes, code) end
         return codes",
    );

    assert_eq!(state, "Not shared");
    assert_eq!(codes, ["AG"]);
}

#[test]
fn an_empty_name_answers_with_the_name_in_the_game() {
    let ada = Player::new("Ada");
    ada.game.reply(&sheet_reply(&[("age", "Thirty.")]));
    ada.run("ns.MspProfile.SetSharing(true)");

    bo_asks(&ada, "?NA");
    ada.tick();

    let (_, text) = &msp_sent(&ada)[0];
    assert!(text.contains("`NA:Ada`"), "{text}");
}

#[test]
fn the_game_fields_tell_the_race_and_the_class() {
    let ada = sharing_ada();

    bo_asks(&ada, "?GC`?GR`?GS`?GF");
    ada.tick();

    let (_, text) = &msp_sent(&ada)[0];
    let fields: Vec<String> = commands(&ada.game, text)
        .into_iter()
        .map(|command| command.rsplit(' ').next().unwrap().to_string())
        .collect();
    assert_eq!(fields, ["PALADIN", "Human", "3", "Alliance"]);
}

/// Carries the messages both ways until none is left, a second at a time.
fn talk(a: &Player, b: &Player) {
    for _ in 0..60 {
        let carried = players::deliver(a, b) + players::deliver(b, a);
        let waiting: usize =
            a.eval::<usize>("ns.Msp.Waiting()") + b.eval::<usize>("ns.Msp.Waiting()");
        if carried == 0 && waiting == 0 {
            return;
        }
        a.tick();
        b.tick();
    }
    panic!("the two players never stopped talking");
}

fn corvin_sees(ada: &Player) -> Player {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    corvin.run("wow.units.mouseover = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");
    corvin
        .game
        .eval::<Vec<String>>("return wow.ShowTooltip('mouseover')");
    talk(ada, &corvin);
    corvin
}

#[test]
fn two_timeways_players_see_the_name_and_the_title_in_the_tooltip() {
    let ada = sharing_ada();

    let corvin = corvin_sees(&ada);

    let lines: Vec<String> = corvin.eval("return wow.ShowTooltip('mouseover')");
    assert_eq!(lines, ["Ada Brightwater, Keeper of the Flame"]);
}

#[test]
fn timeways_asks_for_each_shared_field_and_keeps_its_text_in_memory() {
    let ada = sharing_ada();
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");

    corvin.run("ns.Msp.Ask('Ada', { 'NA', 'NT', 'CU', 'DE', 'AG', 'MO', 'HB', 'HI' })");
    talk(&ada, &corvin);

    let fields: Vec<String> = corvin.eval(
        "local out = {}
         local fields = ns.Msp.FieldsOf('Ada-Stormrage')
         for _, code in ipairs({ 'NA', 'NT', 'CU', 'DE', 'AG', 'MO', 'HB', 'HI' }) do
             table.insert(out, code .. '=' .. tostring(fields[code]))
         end
         return out",
    );
    assert_eq!(
        fields,
        [
            "NA=Ada Brightwater",
            "NT=Keeper of the Flame",
            "CU=Reading by the fire.",
            "DE=Tall, with a scar.",
            "AG=Thirty winters.",
            "MO=Light and steel.",
            "HB=Lordaeron",
            "HI=A farm girl who took up the sword.",
        ]
    );
}

#[test]
fn a_field_asked_again_within_thirty_seconds_is_not_asked() {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");

    corvin.run("ns.Msp.Ask('Ada', { 'DE' })");
    corvin.run("ns.Msp.Ask('Ada', { 'DE' })");

    assert_eq!(msp_sent(&corvin), [(false, "?DE".to_string())]);
}

#[test]
fn a_player_who_never_answered_is_asked_again_only_after_five_minutes() {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");

    corvin.run("ns.Msp.Ask('Ada', { 'DE' })");
    corvin.run("wow.now = wow.now + 60; ns.Msp.Ask('Ada', { 'HI' })");
    let early = msp_sent(&corvin);
    corvin.run("wow.now = wow.now + 300; ns.Msp.Ask('Ada', { 'HI' })");
    let late = msp_sent(&corvin);

    assert_eq!(early, [(false, "?DE".to_string())]);
    assert_eq!(late, [(false, "?HI".to_string())]);
}

#[test]
fn a_known_field_is_asked_with_its_version() {
    let ada = sharing_ada();
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    corvin.run("ns.Msp.Ask('Ada', { 'DE' })");
    talk(&ada, &corvin);

    corvin.run("wow.now = wow.now + 31; ns.Msp.Ask('Ada', { 'DE' })");

    let version: String = corvin.eval("ns.MspWire.Version('Tall, with a scar.')");
    assert_eq!(msp_sent(&corvin), [(false, format!("?DE{version}"))]);
}

#[test]
fn a_text_on_the_normal_channel_is_not_kept() {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");

    corvin.hear("MSP2", "00A001001001NA1:Mallory", "WHISPER", "Bo-Stormrage");

    let name: Option<String> = corvin.eval("ns.Msp.FieldsOf('Bo-Stormrage').NA");
    assert_eq!(name, None);
}

#[test]
fn the_tooltip_line_shows_no_escape_and_cuts_a_long_name() {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    let long = "A".repeat(80);

    corvin.hear_logged(
        "MSP2",
        &format!("00A001001001NA1:~7Ccff00ff00{long}~7Cr`NT1:a~7CHb"),
        "WHISPER",
        "Bo-Stormrage",
    );
    let line: String = corvin.eval("ns.Msp.TooltipLine('Bo-Stormrage')");

    assert_eq!(line, format!("{}, a||Hb", "A".repeat(60)));
}

#[test]
fn with_the_switch_off_the_tooltip_asks_nobody() {
    let corvin = Player::new("Corvin");
    corvin.run("wow.units.mouseover = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");

    corvin
        .game
        .eval::<Vec<String>>("return wow.ShowTooltip('mouseover')");

    assert!(msp_sent(&corvin).is_empty());
}

#[test]
fn msp_takes_at_most_four_parts_in_a_burst() {
    let ada = sharing_ada();
    let long = "a".repeat(1000);
    ada.game.reply(&sheet_reply(&[
        ("appearance", &long),
        ("background", &long),
    ]));

    bo_asks(&ada, "?DE`?HI");

    let parts = ada.take_sent().len();
    assert_eq!(parts, 4);
}

/// The hero edits that the game sent to the desktop, as (field, text).
fn hero_sets(game: &Game) -> Vec<(String, String)> {
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            timeways_story::input::Input::HeroSet { field, text, .. } => Some((field, text)),
            _ => None,
        })
        .collect()
}

/// A game with Total RP 3 loaded: its `LibMSP` global holds the profile of the player.
fn with_trp3() -> Game {
    let game = Game::new();
    game.run(
        "msp = { my = {
             NA = '|cff00ff00Lady Ada|r', NT = 'Keeper', CU = 'Reading\\nby the fire',
             DE = '{h1}Tall{/h1} ' .. string.rep('a', 1200), AG = '30', MO = 'Light',
             HB = 'Gilneas', HI = 'Another story.', VA = 'TotalRP3/2.8' } }
         msp_RPAddOn = 'Total RP 3'",
    );
    game
}

#[test]
fn another_roleplay_addon_fills_the_profile_and_leaves_the_questions_alone() {
    let game = with_trp3();

    game.reply(&sheet_reply(&[("origin", "Lordaeron")]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    let sets = hero_sets(&game);
    let fields: Vec<&str> = sets.iter().map(|(field, _)| field.as_str()).collect();
    assert_eq!(
        fields,
        ["name", "title", "currently", "appearance", "age", "motto"]
    );
    assert_eq!(sets[0].1, "Lady Ada");
    assert_eq!(sets[2].1, "Reading by the fire");
    assert_eq!(sets[3].1, format!("Tall {}", "a".repeat(995)));
}

#[test]
fn an_imported_text_goes_once_even_when_the_desktop_keeps_the_old_one() {
    let game = with_trp3();
    game.reply(&sheet_reply(&[]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");
    let first = hero_sets(&game).len();

    game.reply(&sheet_reply(&[]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    assert_eq!(hero_sets(&game).len(), first);
}

#[test]
fn an_imported_text_that_is_not_utf8_stays_out() {
    let game = with_trp3();
    game.run("msp.my = { NT = '\\255Keeper', MO = 'Light' }");

    game.reply(&sheet_reply(&[]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    assert_eq!(
        hero_sets(&game),
        [("motto".to_string(), "Light".to_string())]
    );
}

#[test]
fn an_imported_text_loses_its_unicode_control_characters() {
    let game = with_trp3();
    game.run("msp.my = { NT = 'Keeper\\194\\128of\\194\\159keys' }");

    game.reply(&sheet_reply(&[]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    assert_eq!(
        hero_sets(&game),
        [("title".to_string(), "Keeper of keys".to_string())]
    );
}

#[test]
fn an_unchanged_import_sends_nothing() {
    let game = with_trp3();
    game.run("msp.my = { NT = 'Keeper' }");

    game.reply(&sheet_reply(&[("title", "Keeper")]));
    game.run("wow.now = wow.now + 1; wow.RunTickers()");

    assert!(hero_sets(&game).is_empty());
}

#[test]
fn with_another_roleplay_addon_timeways_answers_nothing() {
    let ada = sharing_ada();
    ada.run("msp = { my = {} } msp_RPAddOn = 'MyRolePlay'");

    bo_asks(&ada, "?DE");
    ada.tick();

    assert!(msp_sent(&ada).is_empty());
    assert_eq!(
        ada.eval::<String>("ns.MspProfile.State()"),
        "From MyRolePlay"
    );
}

/// The lines of the Hero page with `key` open, as `style: text [button]`.
fn page(game: &Game, key: &str) -> Vec<String> {
    game.run(&format!("ns.JournalFrame.Select('{key}')"));
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('hero')) do
             local action = line.action and (' [' .. line.action.label .. ']') or ''
             table.insert(out, line.style .. ': ' .. line.text .. action)
         end
         return out",
    )
}

/// The rows of the list of the Hero page, as `text (detail)`.
fn list(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('hero').list) do
             table.insert(out, row.text .. (row.detail and (' (' .. row.detail .. ')') or ''))
         end
         return out",
    )
}

#[test]
fn the_roleplay_profile_is_folded_until_it_opens() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true }");
    game.reply(&sheet_reply(&[("title", "Keeper")]));
    game.run("wow.Slash('/hero', '')");

    let folded = list(&game);
    page(&game, "roleplay");
    let open = list(&game);

    assert_eq!(folded.last().unwrap(), "Roleplay Profile (Not shared)");
    assert_eq!(open.len(), folded.len() + 6);
    assert_eq!(
        open[open.len() - 6..open.len() - 4],
        ["Name (Ada)", "Title (Keeper)"]
    );
}

#[test]
fn share_turns_the_switch_on_and_stop_sharing_turns_it_off() {
    let game = Game::new();
    game.reply(&sheet_reply(&[]));
    game.run("wow.Slash('/hero', '')");

    let off = page(&game, "roleplay");
    game.run(
        "for _, line in ipairs(ns.Journal.Lines('hero')) do
             if line.action and line.action.label == 'Share' then line.action.run() end
         end",
    );
    let on = page(&game, "roleplay");

    assert_eq!(off[0], "heading: Roleplay Profile");
    assert_eq!(
        off[1],
        "text: Players with roleplay addons like Total RP 3 can see this profile if you share it. [Share]"
    );
    assert_eq!(
        on[1],
        "text: Players with roleplay addons like Total RP 3 can see this profile. [Stop sharing]"
    );
    assert!(game.eval::<bool>("return TimewaysProfile.share"));
}

#[test]
fn a_field_of_the_profile_opens_with_its_question_and_edit() {
    let game = Game::new();
    game.reply(&sheet_reply(&[("age", "Thirty.")]));
    game.run("wow.Slash('/hero', '')");

    let lines = page(&game, "age");

    assert_eq!(
        lines[..3],
        [
            "note: Roleplay Profile",
            "heading: How old is your character? [Edit]",
            "text: Thirty.",
        ]
    );
}

#[test]
fn an_imported_profile_shows_its_addon_and_has_no_edit() {
    let game = with_trp3();
    game.reply(&sheet_reply(&[("age", "30")]));
    game.run("wow.Slash('/hero', '')");

    let lines = page(&game, "age");
    let overview = page(&game, "roleplay");

    assert_eq!(
        lines[..4],
        [
            "note: Roleplay Profile",
            "heading: How old is your character?",
            "text: 30",
            "hint: From Total RP 3",
        ]
    );
    assert_eq!(
        overview[1],
        "help: Total RP 3 shares your profile. Change it there."
    );
}

#[test]
fn a_profile_field_takes_its_own_limit_in_the_editor() {
    let game = Game::new();
    game.reply(&sheet_reply(&[]));
    game.run("wow.Slash('/hero', '')");

    page(&game, "age");
    game.run(
        "for _, line in ipairs(ns.Journal.Lines('hero')) do
             if line.action and line.action.label == 'Edit' then line.action.run() end
         end",
    );

    let limit: u32 = game.eval("return wow.EditBox().maxLetters");
    assert_eq!(limit, 100);
}

/// Ada shares a long profile, the logged channel gives nil, and Bo asks for two long fields.
fn ada_answers_long_fields_with_nil_results() -> Player {
    let ada = sharing_ada();
    let long = "a".repeat(1000);
    ada.game.reply(&sheet_reply(&[
        ("appearance", &long),
        ("background", &long),
    ]));
    ada.run("wow.loggedGivesNil = true");
    bo_asks(&ada, "?DE`?HI");
    ada
}

#[test]
fn a_logged_send_that_gives_nil_still_spends_the_burst() {
    let ada = ada_answers_long_fields_with_nil_results();

    let parts = ada.take_sent().len();

    assert_eq!(parts, 4);
}

#[test]
fn a_logged_reply_goes_out_whole_when_the_send_gives_nil() {
    let ada = ada_answers_long_fields_with_nil_results();

    for _ in 0..20 {
        ada.tick();
    }

    let sent = msp_sent(&ada);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].0 && sent[0].1.ends_with(&"a".repeat(1000)));
}

/// Corvin, sharing, hovers over Ada with these fields in her unit table.
fn corvin_hovers_ada(unit_fields: &str) -> Player {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    corvin.run(&format!(
        "wow.units.mouseover = {{ name = 'Ada', player = true, guid = 'Player-1-Ada', {unit_fields} }}"
    ));
    corvin
        .game
        .eval::<Vec<String>>("return wow.ShowTooltip('mouseover')");
    corvin
}

#[test]
fn the_tooltip_asks_a_player_of_your_faction_on_a_connected_realm() {
    let corvin = corvin_hovers_ada("faction = 'Alliance'");

    assert_eq!(msp_sent(&corvin), [(false, "?TT".to_string())]);
}

#[test]
fn the_tooltip_never_whispers_a_player_who_cannot_get_the_whisper() {
    for unit_fields in ["faction = 'Horde'", "farRealm = true", "offline = true"] {
        let corvin = corvin_hovers_ada(unit_fields);

        assert!(msp_sent(&corvin).is_empty(), "{unit_fields}");
    }
}

#[test]
fn a_full_list_of_players_forgets_only_the_oldest_one() {
    let corvin = Player::new("Corvin");
    corvin.run("ns.MspProfile.SetSharing(true)");
    corvin.hear_logged("MSP2", "00A001001001NA1:Ann", "WHISPER", "Player1-Stormrage");
    corvin.tick();
    corvin.hear_logged("MSP2", "00A001001001NA1:Bee", "WHISPER", "Player2-Stormrage");

    corvin.run(
        "for n = 3, 201 do
             wow.now = wow.now + 1
             ns.Msp.Ask('Player' .. n, { 'DE' })
         end",
    );

    let first: Option<String> = corvin.eval("ns.Msp.FieldsOf('Player1').NA");
    let second: Option<String> = corvin.eval("ns.Msp.FieldsOf('Player2').NA");
    assert_eq!(first, None);
    assert_eq!(second.as_deref(), Some("Bee"));
}
