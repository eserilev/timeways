//! The names of real players never reach a model (GAMEPLAY.md 4.7, 4.8, 5.11): the addon
//! marks each name that it knows, in any case and with any realm, and the story program
//! swaps each mark for an ID.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::Player;
use proptest::prelude::*;

/// Ada, with Corvin in her guild. `setup` puts other players where the addon can know them.
fn marked(setup: &str, idea: &str) -> String {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_guild_with(&[&corvin]);
    ada.run(setup);
    let mark: mlua::Function =
        ada.eval("return function(text) return (ns.TaskNames.Marked(text)) end");
    mark.call(idea).unwrap()
}

#[test]
fn a_name_in_any_case_or_with_its_realm_gets_a_mark_in_the_form_of_the_game() {
    let idea = marked(
        "",
        "tell corvin, CORVIN and Corvin-Stormrage to help Ada. Corvin's axe.",
    );

    assert_eq!(
        idea,
        "tell {Corvin}, {Corvin} and {Corvin} to help $N. {Corvin}'s axe."
    );
}

#[test]
fn a_word_that_holds_a_name_stays() {
    let idea = marked("", "Corvinus and Adamant stay");

    assert_eq!(idea, "Corvinus and Adamant stay");
}

#[test]
fn a_brace_that_a_player_typed_becomes_a_parenthesis() {
    let idea = marked("", "{Corvin} and {P7} stay");

    assert_eq!(idea, "({Corvin}) and (P7) stay");
}

#[test]
fn each_player_that_the_addon_knows_gets_a_mark() {
    let setup = "wow.units.party1 = { name = 'Bram', player = true }
         wow.units.raid7 = { name = 'Lysa', realm = 'Argent Dawn', player = true }
         table.insert(wow.guild, { name = 'Ömer-Stormrage', online = false })
         wow.Fire('GUILD_ROSTER_UPDATE', false)
         wow.friends = { { name = 'Zoë', connected = false } }
         ns.TaskForm.Draft().doer = 'Élise-Stormrage'
         ns.TaskForm.Draft().steps = { { kind = 'meet', target = 'Tomas-Stormrage', count = 1 } }";

    let idea = marked(setup, "bram lysa ömer ZOË élise tomas");

    assert_eq!(idea, "{Bram} {Lysa} {Ömer} {Zoë} {Élise} {Tomas}");
}

#[test]
fn a_player_in_sight_gets_a_line_with_the_race_and_the_class() {
    let ada = Player::new("Ada");
    ada.run(
        "wow.units.party1 = { name = 'Bram', player = true, race = 'Scourge', class = 'Mage' }
         wow.friends = { { name = 'Ömer', connected = false } }",
    );

    let lines: Vec<Vec<String>> = ada.eval(
        "local _, players = ns.TaskNames.Marked('bram and ömer')
         local lines = {}
         for _, line in ipairs(ns.TaskNames.Described(players)) do
             lines[#lines + 1] = { line.name, line.race, line.class }
         end
         return lines",
    );

    assert_eq!(
        lines,
        [vec![
            "Bram".to_string(),
            "Scourge".to_string(),
            "MAGE".to_string()
        ]]
    );
}

/// The game can hide the race and the class of another player
/// (`SecretWhenUnitIdentityRestricted`). A hidden value never goes to the desktop.
#[test]
fn a_race_that_the_game_hides_stays_out_of_the_line() {
    let ada = Player::new("Ada");
    ada.run(
        "wow.units.party1 = { name = 'Bram', player = true, race = 'Scourge', class = 'Mage' }
         wow.secrets['Scourge'] = true",
    );

    let line: Vec<String> = ada.eval(
        "local _, players = ns.TaskNames.Marked('bram')
         local line = ns.TaskNames.Described(players)[1]
         return { tostring(line.race), line.class }",
    );

    assert_eq!(line, ["nil", "MAGE"]);
}

/// The lowercase letters of names: ASCII, and the Latin letters of European realms.
const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzàáâãäåæçèéêëìíîïðñòóôõöøùúûüýþ\
    āăąćĉċčďđēĕėęěĝğġģĥħĩīĭįĵķĺļľŀłńņňŋōŏőœŕŗřśŝşšţťŧũūŭůűųŵŷźżž";

fn name() -> impl Strategy<Value = String> {
    let letters: Vec<char> = LETTERS.chars().collect();
    proptest::collection::vec(proptest::sample::select(letters), 2..=12).prop_map(|letters| {
        let mut name: String = letters[0].to_uppercase().collect();
        name.extend(&letters[1..]);
        name
    })
}

/// The ways that a player types a name: as the game shows it, in lowercase, in uppercase,
/// with its realm, and with a mark after it.
fn typed(name: &str, form: u8) -> String {
    match form % 5 {
        0 => name.to_string(),
        1 => name.to_lowercase(),
        2 => name.to_uppercase(),
        3 => format!("{name}-Stormrage"),
        _ => format!("{name}!"),
    }
}

/// The places where the addon can know a player from.
fn known_as(name: &str, place: u8) -> String {
    match place % 4 {
        0 => format!("wow.units.party1 = {{ name = '{name}', player = true }}"),
        1 => format!(
            "table.insert(wow.guild, {{ name = '{name}-Stormrage', online = false }})
             wow.Fire('GUILD_ROSTER_UPDATE', false)"
        ),
        2 => format!("wow.friends = {{ {{ name = '{name}', connected = false }} }}"),
        _ => format!("ns.TaskForm.Draft().doer = '{name}-Stormrage'"),
    }
}

/// The words of a text outside its marks: the story program swaps each mark for an ID.
fn unmarked_words(text: &str) -> Vec<String> {
    let mut outside = String::new();
    let mut in_mark = false;
    for c in text.chars() {
        match c {
            '{' => in_mark = true,
            '}' => in_mark = false,
            _ if !in_mark => outside.push(c),
            _ => {}
        }
    }
    outside
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_known_name_gets_a_mark(name in name(), form: u8, place: u8) {
        let idea = format!("please ask {} to check the mill", typed(&name, form));

        let marked = marked(&known_as(&name, place), &idea);

        prop_assert!(
            !unmarked_words(&marked).contains(&name.to_lowercase()),
            "{idea} became {marked}"
        );
        prop_assert!(marked.contains(&format!("{{{name}}}")), "{idea} became {marked}");
    }
}
