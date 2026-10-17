//! On WoW Forever a character has a first name and, often, a surname (GAMEPLAY.md 5.11).
//! Timeways shows the whole name, and no part of a name reaches a model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use proptest::prelude::*;
use timeways_story::aliases::words_in;
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

/// Kobee Bean on WoW Forever, with Leborc James in the group.
fn kobee_bean() -> Game {
    let game = Game::new();
    game.run(
        "wow.regional = true
         wow.units.player = { name = 'Kobee', realm = 'Bean', player = true, guid = 'Player-1-Kobee', level = 12 }
         wow.units.party1 = { name = 'Leborc', realm = 'James', player = true, guid = 'Player-1-Leborc' }",
    );
    game
}

fn marked(game: &Game, text: &str) -> String {
    let mark: mlua::Function =
        game.eval("return function(text) return (ns.TaskNames.Marked(text)) end");
    mark.call(text).unwrap()
}

#[test]
fn your_name_is_your_first_name_and_surname() {
    let game = kobee_bean();

    assert_eq!(game.eval::<String>("ns.PlayerNames.Mine()"), "Kobee Bean");
}

#[test]
fn your_world_is_named_by_your_whole_name() {
    let game = kobee_bean();

    game.run("ns.Watch.Login() ns.Outbox.Flush()");

    let sent = game.sent().join("\n");
    assert!(sent.contains(r#""name":"Kobee Bean""#), "{sent}");
}

#[test]
fn a_character_with_no_surname_shows_the_first_name_alone() {
    let game = kobee_bean();
    game.run("wow.units.player.realm = nil");

    assert_eq!(game.eval::<String>("ns.PlayerNames.Mine()"), "Kobee");
}

#[test]
fn a_line_from_the_desktop_shows_your_full_name_for_the_hero_mark() {
    let game = kobee_bean();

    let line: String = game.eval("ns.WithName('$N stands at the gate.')");

    assert_eq!(line, "Kobee Bean stands at the gate.");
}

#[test]
fn the_roleplay_profile_names_you_with_your_surname() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.MspProfile.Fields().NA"),
        "Kobee Bean"
    );
}

#[test]
fn another_player_shows_with_a_surname() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.PlayerNames.Shown('Leborc-James')"),
        "Leborc James"
    );
}

#[test]
fn a_name_from_chat_with_a_space_becomes_the_key_of_the_game() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.TaskPeople.Full('Leborc James')"),
        "Leborc-James"
    );
}

#[test]
fn outside_forever_a_player_of_your_realm_shows_without_the_realm() {
    let game = Game::new();

    assert_eq!(
        game.eval::<String>("ns.PlayerNames.Shown('Bram-' .. GetNormalizedRealmName())"),
        "Bram"
    );
}

#[test]
fn your_typed_full_name_becomes_the_hero_mark_with_no_surname_left() {
    let game = kobee_bean();

    assert_eq!(marked(&game, "help Kobee Bean now"), "help $N now");
}

#[test]
fn a_typed_full_name_of_a_player_of_your_group_keeps_no_surname_and_the_mark_carries_it() {
    let game = kobee_bean();

    assert_eq!(
        marked(&game, "ask Leborc James, then leborc."),
        "ask {Leborc-James}, then {Leborc-James}."
    );
}

#[test]
fn npc_text_with_your_full_name_keeps_no_part_of_it() {
    let game = kobee_bean();

    game.run(
        "ns.Seen.Read('gossip', nil, 'Innkeeper Renee', 'Welcome, Kobee Bean. Sit, Kobee.')
         ns.Outbox.Flush()",
    );

    let sent = game.sent().join("\n");
    assert!(sent.contains("Welcome, $N. Sit, $N."), "{sent}");
    assert!(!sent.contains("Bean"), "{sent}");
}

proptest! {
    /// Any first name and surname, in any case, anywhere in a line: no part of your own name
    /// reaches a model (GAMEPLAY.md 5.11).
    #[test]
    fn no_part_of_your_full_name_survives_the_mark(
        first in "[A-Z][a-z]{2,9}",
        surname in "[A-Z][a-z]{2,9}",
        before in "[a-z ,.!]{0,12}",
        after in "[a-z ,.!]{0,12}",
        upper in any::<bool>(),
    ) {
        let (low_first, low_surname) = (first.to_lowercase(), surname.to_lowercase());
        prop_assume!(low_first != low_surname);
        let around = format!("{before} {after}");
        prop_assume!(!around.contains(&low_first) && !around.contains(&low_surname));
        let game = Game::new();
        game.run(&format!(
            "wow.regional = true
             wow.units.player = {{ name = '{first}', realm = '{surname}', player = true, guid = 'Player-1-X', level = 1 }}"
        ));
        let typed = if upper { format!("{first} {surname}").to_uppercase() } else { format!("{first} {surname}") };

        let text = marked(&game, &format!("{before} {typed} {after}"));

        let lower = text.to_lowercase();
        prop_assert!(!lower.contains(&low_first), "{text}");
        prop_assert!(!lower.contains(&low_surname), "{text}");
    }
}

/// Kobee Bean, with Kobee Will in the guild and Leborc James and Kobee Moss as friends, as
/// the friends list writes them.
fn two_kobees() -> Game {
    let game = kobee_bean();
    game.run(
        "wow.units.party1 = nil
         wow.guild = { { name = 'Kobee-Will', online = true } }
         wow.Fire('GUILD_ROSTER_UPDATE', false)
         wow.friends = { { name = 'Leborc James', connected = false }, { name = 'Kobee Moss', connected = false } }",
    );
    game
}

#[test]
fn players_who_share_a_first_name_each_lose_their_own_surname() {
    let game = two_kobees();

    let text = marked(&game, "ask Kobee Will and Kobee Moss, then Kobee-Moss");

    assert_eq!(text, "ask {Kobee-Will} and {Kobee-Moss}, then {Kobee-Moss}");
}

#[test]
fn another_player_with_your_first_name_is_never_you() {
    let game = two_kobees();

    let text = marked(
        &game,
        "Kobee Will healed me. Kobee Bean thanked them. Kobee rests.",
    );

    assert_eq!(text, "{Kobee-Will} healed me. $N thanked them. $N rests.");
}

#[test]
fn a_friend_listed_as_first_and_surname_is_known_by_the_key_of_the_game() {
    let game = two_kobees();

    let text = marked(&game, "ask Leborc James and leborc");
    let relation: String = game.eval("return ns.TaskPeople.Relation('Leborc-James')");

    assert_eq!(text, "ask {Leborc-James} and {Leborc-James}");
    assert_eq!(relation, "friend");
}

#[test]
fn a_surname_in_lower_case_after_a_known_first_name_stays_a_word() {
    let game = two_kobees();

    let text = marked(&game, "Kobee will help, and Leborc james too");

    assert_eq!(text, "$N will help, and {Leborc-James} james too");
}

#[test]
fn a_surname_alone_is_left_for_the_desktop() {
    let game = two_kobees();

    let text = marked(&game, "Bean is my house. Will you come to the James house?");

    assert_eq!(text, "Bean is my house. Will you come to the James house?");
}

#[test]
fn a_first_name_alone_picks_one_player_in_a_steady_order() {
    let game = kobee_bean();
    game.run(
        "wow.units.party1 = nil
         wow.guild = { { name = 'Leborc-James', online = true }, { name = 'Leborc-Ash', online = true } }
         wow.Fire('GUILD_ROSTER_UPDATE', false)",
    );
    let from_the_guild = marked(&game, "ask leborc");
    game.run("wow.units.party1 = { name = 'Leborc', realm = 'James', player = true, guid = 'Player-1-Leborc' }");

    let from_the_group = marked(&game, "ask leborc");

    assert_eq!(from_the_guild, "ask {Leborc-Ash}");
    assert_eq!(from_the_group, "ask {Leborc-James}");
}

#[test]
fn a_whole_name_goes_across_any_run_of_spaces() {
    let game = two_kobees();

    let text = marked(
        &game,
        "ask Leborc  James, Leborc\u{A0}James, Kobee\u{A0}Bean",
    );

    assert_eq!(text, "ask {Leborc-James}, {Leborc-James}, $N");
}

const FIRSTS: [&str; 4] = ["Kobee", "Leborc", "Ada", "Bram"];
const SURNAMES: [&str; 4] = ["Will", "James", "Moss", "Bean"];

/// A known player by the index of a first name and of a surname, in the guild as the key of
/// the game, or a friend as the friends list writes the name.
fn known_player() -> impl Strategy<Value = (usize, usize, bool)> {
    (0..FIRSTS.len(), 0..SURNAMES.len(), any::<bool>())
}

/// One way to write a player: the whole name, with a dash, the first name in any case, or
/// the surname alone, with a capital or in lower case.
fn written(first: &str, surname: &str, form: u8) -> String {
    match form % 7 {
        0 => format!("{first} {surname}"),
        1 => format!("{first}-{surname}"),
        2 => first.to_lowercase(),
        3 => first.to_uppercase(),
        4 => first.to_string(),
        5 => surname.to_string(),
        _ => surname.to_lowercase(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// GAMEPLAY.md 5.11: the addon marks every first name of a known player and of yours,
    /// and the desktop hides every surname with a capital that a mark taught it. A surname
    /// in lower case is a word.
    #[test]
    fn no_known_first_name_or_capital_surname_reaches_a_prompt(
        players in proptest::collection::vec(known_player(), 1..5),
        named in proptest::collection::vec((0usize..6, any::<u8>(), "[c-k]{1,4}"), 1..8),
    ) {
        let game = kobee_bean();
        let guild: Vec<String> = players.iter().filter(|p| p.2)
            .map(|(f, s, _)| format!("{{ name = '{}-{}', online = true }}", FIRSTS[*f], SURNAMES[*s]))
            .collect();
        let friends: Vec<String> = players.iter().filter(|p| !p.2)
            .map(|(f, s, _)| format!("{{ name = '{} {}', connected = false }}", FIRSTS[*f], SURNAMES[*s]))
            .collect();
        game.run(&format!(
            "wow.units.party1 = nil
             wow.guild = {{ {} }} wow.Fire('GUILD_ROSTER_UPDATE', false)
             wow.friends = {{ {} }}",
            guild.join(", "),
            friends.join(", ")
        ));
        // The player is Kobee Bean, index 0 of each list.
        let mut people: Vec<(usize, usize)> = players.iter().map(|(f, s, _)| (*f, *s)).collect();
        people.push((0, 3));
        let text: Vec<String> = named
            .iter()
            .map(|(who, form, filler)| {
                let (first, surname) = people[who % people.len()];
                format!("{} {filler}", written(FIRSTS[first], SURNAMES[surname], *form))
            })
            .collect();
        let text = text.join(" ");

        let marked = marked(&game, &text);
        let idea = idea_in_the_prompt(&marked);

        // The desktop knows the surnames of the marks, and yours.
        let mut taught: Vec<String> = mark_surnames(&marked);
        taught.push("Bean".to_string());
        for word in words_in(&idea) {
            let first = people.iter().any(|(f, _)| FIRSTS[*f].eq_ignore_ascii_case(word));
            let capital = word.chars().next().is_some_and(char::is_uppercase);
            let surname = capital && taught.iter().any(|known| known.eq_ignore_ascii_case(word));
            prop_assert!(!first && !surname, "{} in {} from {}", word, idea, text);
        }
    }
}

/// The surnames that the marks of a text carry: "{Leborc-James}" carries James.
fn mark_surnames(text: &str) -> Vec<String> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}'))
        .filter_map(|(inner, _)| {
            inner
                .split_once('-')
                .map(|(_, surname)| surname.to_string())
        })
        .collect()
}

/// The idea of a draft as a model reads it, from a desktop with the character Kobee Bean.
fn idea_in_the_prompt(marked: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("player-names-pack.sqlite");
    if !path.exists() {
        Pack::write(&path, &[]).unwrap();
    }
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Kobee Bean".to_string(),
        })
        .unwrap();
    let asked = Input::DraftAsked {
        id: MessageId(1),
        at: Tick(1),
        idea: marked.to_string(),
    };
    let Some(Output::ModelCall { prompt, .. }) = story.handle(asked).unwrap().into_iter().next()
    else {
        panic!("no draft call for {marked}");
    };
    let (_, after) = prompt.split_once("The idea of the player:\n<<<\n").unwrap();
    after.split_once("\n>>>").unwrap().0.to_string()
}
