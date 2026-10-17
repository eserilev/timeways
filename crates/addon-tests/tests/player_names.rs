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
fn a_surname_alone_with_a_capital_gets_a_mark_of_its_own() {
    let game = two_kobees();

    let text = marked(
        &game,
        "Bean is home. Will you come to the James house? Moss came. the will clan",
    );

    assert_eq!(
        text,
        "Bean is home. {~Kobee-Will} you come to the {~Leborc-James} house? {~Kobee-Moss} came. the will clan"
    );
}

#[test]
fn a_surname_mark_carries_the_surname_as_the_game_writes_it() {
    let game = two_kobees();

    let text = marked(&game, "WILL rode off");

    assert_eq!(text, "{~Kobee-Will} rode off");
}

#[test]
fn a_text_of_the_hero_names_the_player_of_a_surname_alone() {
    let game = two_kobees();

    let names: Vec<String> = game.eval("return (ns.TaskNames.Named('The Will family wronged me'))");

    assert_eq!(names, ["Kobee-Will"]);
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

    /// GAMEPLAY.md 5.11: no first name of a known player or of yours, and no surname with a
    /// capital, reaches the prompt. Each mark becomes the ID of its own player or surname,
    /// and each `$N` stays one. A surname in lower case is a word. Names often stand side by
    /// side, and some worlds learned the players before.
    #[test]
    fn no_known_first_name_or_capital_surname_reaches_a_prompt(
        players in proptest::collection::vec(known_player(), 1..5),
        named in proptest::collection::vec((0usize..6, any::<u8>(), "[c-k]{0,4}"), 1..8),
        teach in any::<bool>(),
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
                let name = written(FIRSTS[first], SURNAMES[surname], *form);
                if filler.is_empty() { name } else { format!("{name} {filler}") }
            })
            .collect();
        let text = text.join(" ");
        let whole_names: Vec<String> = people
            .iter()
            .map(|(f, s)| format!("{} {}", FIRSTS[*f], SURNAMES[*s]))
            .collect();
        let teaching = teach.then(|| marked(&game, &whole_names.join(", ")));

        let marked = marked(&game, &text);
        let (idea, cards) = draft_in_the_prompt(teaching.as_deref(), &marked);

        for word in words_in(&idea) {
            let first = people.iter().any(|(f, _)| FIRSTS[*f].eq_ignore_ascii_case(word));
            let capital = word.chars().next().is_some_and(char::is_uppercase);
            let surname = capital && people.iter().any(|(_, s)| SURNAMES[*s].eq_ignore_ascii_case(word));
            prop_assert!(!first && !surname, "{} in {} from {}", word, idea, text);
        }
        // Each mark becomes an ID of its own, in order: a surname mark the ID of a surname.
        let marks = marks_of(&marked);
        let ids = ids_of(&idea);
        prop_assert_eq!(ids.len(), marks.len(), "{} from {}", idea, marked);
        for (mark, id) in marks.iter().zip(&ids) {
            let is_surname = cards.get(id).is_some_and(|card| card.contains("a surname of"));
            prop_assert_eq!(mark.starts_with('~'), is_surname, "{} as {} in {}", mark, id, idea);
        }
        // Each `$N` stays one, and each run of your surname alone with a capital becomes one.
        let expected = marked.matches("$N").count() + runs_of_your_surname(&marked);
        prop_assert_eq!(idea.matches("$N").count(), expected, "{} from {}", idea, marked);
    }
}

/// The inner of each mark of a text, in order: "Kobee-Will", "~Kobee-Will".
fn marks_of(text: &str) -> Vec<String> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}'))
        .map(|(inner, _)| inner.to_string())
        .collect()
}

/// The IDs of a text, in order: "P1".
fn ids_of(text: &str) -> Vec<String> {
    marks_of(text)
        .into_iter()
        .filter(|inner| inner.starts_with('P'))
        .collect()
}

/// The runs of your surname Bean alone with a capital, side by side: the desktop makes each
/// run one `$N`.
fn runs_of_your_surname(text: &str) -> usize {
    let is_bean = |word: &str| word.eq_ignore_ascii_case("Bean") && word.starts_with('B');
    let words: Vec<&str> = text.split_whitespace().collect();
    let starts = words
        .iter()
        .enumerate()
        .filter(|(n, word)| is_bean(word) && (*n == 0 || !is_bean(words[n - 1])));
    starts.count()
}

/// The idea of a draft as a model reads it, and the card of each ID, from a desktop with
/// the character Kobee Bean. A text that teaches the players goes first when there is one.
fn draft_in_the_prompt(
    teaching: Option<&str>,
    marked: &str,
) -> (String, std::collections::HashMap<String, String>) {
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
    let mut prompt = String::new();
    for (id, idea) in (1..).zip(teaching.into_iter().chain([marked])) {
        let asked = Input::DraftAsked {
            id: MessageId(id),
            at: Tick(id),
            idea: idea.to_string(),
        };
        let Some(Output::ModelCall { prompt: asked, .. }) =
            story.handle(asked).unwrap().into_iter().next()
        else {
            panic!("no draft call for {idea}");
        };
        prompt = asked;
    }
    let (_, after) = prompt.split_once("The idea of the player:\n<<<\n").unwrap();
    let idea = after.split_once("\n>>>").unwrap().0.to_string();
    let cards = prompt
        .lines()
        .filter_map(|line| line.strip_prefix("- {")?.split_once("}: "))
        .map(|(id, card)| (id.to_string(), card.to_string()))
        .collect();
    (idea, cards)
}
