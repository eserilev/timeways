//! The names of real players never reach a model (GAMEPLAY.md 4.7, 5.11): "Help me write
//! this" takes each name that the addon knows out of the idea, in any case and with any
//! realm.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::Player;
use proptest::prelude::*;

/// Ada, with Corvin in her guild. `setup` puts other players where the addon can know them.
fn scrubbed(setup: &str, idea: &str) -> String {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_guild_with(&[&corvin]);
    ada.run(setup);
    let scrub: mlua::Function = ada.eval("return ns.TaskNames.WithoutNames");
    scrub.call(idea).unwrap()
}

#[test]
fn a_name_in_any_case_or_with_its_realm_becomes_my_friend() {
    let idea = scrubbed(
        "",
        "tell corvin, CORVIN and Corvin-Stormrage to help Ada. Corvin's axe.",
    );

    assert_eq!(
        idea,
        "tell my friend, my friend and my friend to help $N. my friend's axe."
    );
}

#[test]
fn a_word_that_holds_a_name_stays() {
    let idea = scrubbed("", "Corvinus and Adamant stay");

    assert_eq!(idea, "Corvinus and Adamant stay");
}

#[test]
fn each_player_that_the_addon_knows_loses_the_name() {
    let setup = "wow.units.party1 = { name = 'Bram', player = true }
         wow.units.raid7 = { name = 'Lysa', realm = 'Argent Dawn', player = true }
         table.insert(wow.guild, { name = 'Ömer-Stormrage', online = false })
         wow.Fire('GUILD_ROSTER_UPDATE', false)
         wow.friends = { { name = 'Zoë', connected = false } }
         ns.TaskForm.Draft().doer = 'Élise-Stormrage'
         ns.TaskForm.Draft().steps = { { kind = 'meet', target = 'Tomas-Stormrage', count = 1 } }";

    let idea = scrubbed(setup, "bram lysa ömer ZOË élise tomas");

    assert_eq!(
        idea,
        "my friend my friend my friend my friend my friend my friend"
    );
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

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn no_known_name_reaches_the_model(name in name(), form: u8, place: u8) {
        let idea = format!("please ask {} to check the mill", typed(&name, form));

        let scrubbed = scrubbed(&known_as(&name, place), &idea);

        prop_assert!(
            !words(&scrubbed).contains(&name.to_lowercase()),
            "{idea} became {scrubbed}"
        );
    }
}
