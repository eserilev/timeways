//! Outcome passages: the detector of the builder, what each one depends on, and the gate
//! of the spoiler limit (GAMEPLAY.md 5.10). Every page is invented, except the names of
//! the game.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::character::Character;
use timeways_story::input::GameQuestKind;
use timeways_story::narrator_lore::lore_about;
use timeways_story::outcome_passages::{
    Chain, Npc, PageKind, Quest, Stance, Title, dependency, is_outcome, page_kind,
    with_known_bosses,
};
use timeways_story::pack::{Dependency, Link, Origin, Pack, Passage};
use timeways_story::seen::SeenIndex;
use timeways_story::spoiler::{may_show, outcome_allowed};
use timeways_story::wikitext::Cites;

const VANCLEEF_FALLS: &str = "After years of tyranny, the adventurers liberated Moonbrook \
                              after they killed the leader of the Brotherhood, Edwin VanCleef.";

fn npc(name: &str, stance: Stance) -> PageKind {
    PageKind::Npc(Npc {
        name: name.to_string(),
        stance,
    })
}

fn quest(name: &str, chain: Chain, title: Title) -> PageKind {
    PageKind::Quest(Quest {
        name: name.to_string(),
        chain,
        title,
    })
}

fn cites(links: &[&str], refs: &[&str]) -> Cites {
    Cites {
        links: links.iter().map(ToString::to_string).collect(),
        refs: refs.iter().map(ToString::to_string).collect(),
    }
}

/// The pages of a small invented wiki.
fn kind_of(title: &str) -> Option<PageKind> {
    match title {
        "Edwin VanCleef" => Some(npc("Edwin VanCleef", Stance::Foe)),
        "Mr. Smite" => Some(npc("Mr. Smite", Stance::Foe)),
        "Gryan Stoutmantle" => Some(npc("Gryan Stoutmantle", Stance::NoFoe)),
        "The Defias Brotherhood (7)" => {
            Some(quest("The Defias Brotherhood", Chain::End, Title::Shared))
        }
        "Red Linen Goods" => Some(quest("Red Linen Goods", Chain::End, Title::Own)),
        "The People's Militia (quest)" => {
            Some(quest("The People's Militia", Chain::Continues, Title::Own))
        }
        "Moonbrook" => Some(PageKind::Other),
        _ => None,
    }
}

#[test]
fn adventurers_who_killed_a_foe_tell_a_deed() {
    assert!(is_outcome(VANCLEEF_FALLS));
}

#[test]
fn a_group_of_adventurers_and_the_heroes_of_a_side_tell_a_deed() {
    assert!(is_outcome(
        "A group of adventurers slew the beast of the lake."
    ));
    assert!(is_outcome(
        "The heroes of the Alliance destroyed the Brotherhood."
    ));
    assert!(is_outcome("The heroes of the Horde rescued the elder."));
}

#[test]
fn adventurers_who_often_visit_the_inn_tell_no_deed() {
    assert!(!is_outcome(
        "Adventurers often visit the inn of Goldshire on their way to Stormwind."
    ));
}

#[test]
fn the_adventurers_guild_is_no_adventurer() {
    assert!(!is_outcome(
        "The adventurer's guild of Testvale was destroyed in the war."
    ));
    assert!(!is_outcome(
        "The adventurers' hall was destroyed in the war."
    ));
}

#[test]
fn a_request_to_kill_is_no_result() {
    assert!(!is_outcome(
        "Gryan Stoutmantle sent adventurers to kill the Defias of Moonbrook."
    ));
}

#[test]
fn the_doers_and_the_result_must_share_a_sentence() {
    assert!(!is_outcome(
        "Adventurers came to Moonbrook. Years later, the mine was destroyed by a flood."
    ));
}

#[test]
fn a_foe_that_the_deed_names_is_what_the_passage_depends_on() {
    let links = cites(&["Gryan Stoutmantle", "Mr. Smite", "Edwin VanCleef"], &[]);

    let found = dependency(VANCLEEF_FALLS, &links, "Moonbrook", kind_of);

    assert_eq!(found, Dependency::Foe("Edwin VanCleef".to_string()));
}

#[test]
fn a_friend_that_the_deed_names_is_no_foe() {
    let text = "The adventurers sent by Gryan Stoutmantle succeeded.";

    let found = dependency(
        text,
        &cites(&["Gryan Stoutmantle"], &[]),
        "Moonbrook",
        kind_of,
    );

    assert_eq!(found, Dependency::Unresolved);
}

#[test]
fn a_cited_quest_that_ends_its_chain_ties_the_deed() {
    let text = "An adventurer returned the stolen linen to the tailor.";
    let citations = cites(&[], &["The People's Militia (quest)", "Red Linen Goods"]);

    let found = dependency(text, &citations, "Moonbrook", kind_of);

    assert_eq!(found, Dependency::Quest("Red Linen Goods".to_string()));
}

#[test]
fn a_quest_in_the_middle_of_a_chain_ties_nothing() {
    let text = "The heroes of the Alliance succeeded.";

    let found = dependency(
        text,
        &cites(&[], &["The People's Militia (quest)"]),
        "Moonbrook",
        kind_of,
    );

    assert_eq!(found, Dependency::Unresolved);
}

#[test]
fn a_quest_whose_title_other_quests_share_ties_nothing() {
    let text = "The heroes of the Alliance succeeded.";

    let found = dependency(
        text,
        &cites(&[], &["The Defias Brotherhood (7)"]),
        "Moonbrook",
        kind_of,
    );

    assert_eq!(found, Dependency::Unresolved);
}

#[test]
fn a_shared_quest_title_leaves_the_deed_to_a_linked_foe() {
    let text = "He was killed at the hands of adventurers.";
    let citations = cites(&["Edwin VanCleef"], &["The Defias Brotherhood (7)"]);

    let found = dependency(text, &citations, "Moonbrook", kind_of);

    assert_eq!(found, Dependency::Foe("Edwin VanCleef".to_string()));
}

#[test]
fn the_page_of_a_foe_is_what_its_own_deed_depends_on() {
    let text = "Years later, he was eliminated by adventurers.";

    let found = dependency(text, &cites(&["Mr. Smite"], &[]), "Edwin VanCleef", kind_of);

    assert_eq!(found, Dependency::Foe("Edwin VanCleef".to_string()));
}

#[test]
fn a_deed_with_no_foe_and_no_quest_is_unresolved() {
    let text = "The adventurers returned to Orgrimmar.";

    let found = dependency(text, &Cites::default(), "Orgrimmar", kind_of);

    assert_eq!(found, Dependency::Unresolved);
}

#[test]
fn an_npc_infobox_of_a_side_is_no_foe_and_a_combat_one_is() {
    let gryan = "{{Npcbox\n| name = Gryan Stoutmantle\n| faction = Alliance\n}}\nText.";
    let smite = "{{Npcbox\n| name = Mr. Smite\n| faction = Combat\n}}\nText.";
    let golem = "{{Npcbox\n| faction = Neutral\n| aggro = {{Aggro|-1|-1}}\n}}";

    assert_eq!(
        page_kind("Gryan Stoutmantle", gryan),
        npc("Gryan Stoutmantle", Stance::NoFoe)
    );
    assert_eq!(page_kind("Mr. Smite", smite), npc("Mr. Smite", Stance::Foe));
    assert_eq!(
        page_kind("Test Golem (Classic)", golem),
        npc("Test Golem", Stance::Foe)
    );
}

#[test]
fn a_quest_infobox_gives_the_title_of_the_game_and_the_end_of_its_chain() {
    let end = "{{Questbox\n| name = The Defias Brotherhood\n| previous = [[X (6)]]\n}}";
    let middle = "{{Questbox\n| name = The People's Militia\n| next = [[Y (2)]]\n}}";

    assert_eq!(
        page_kind("The Defias Brotherhood (7)", end),
        quest("The Defias Brotherhood", Chain::End, Title::Shared)
    );
    assert_eq!(
        page_kind("The People's Militia (quest)", middle),
        quest("The People's Militia", Chain::Continues, Title::Own)
    );
    assert_eq!(page_kind("Moonbrook", "A town."), PageKind::Other);
}

#[test]
fn a_known_boss_is_a_foe_whatever_its_infobox_says() {
    let neutral = npc("Edwin VanCleef", Stance::NoFoe);

    let known = with_known_bosses(neutral, &["Edwin VanCleef".to_string()]);

    assert_eq!(known, npc("Edwin VanCleef", Stance::Foe));
}

fn vancleef_falls() -> Passage {
    Passage {
        text: VANCLEEF_FALLS.to_string(),
        source: "the wiki page \"Moonbrook\"".to_string(),
        links: vec![Link::Place("Moonbrook".to_string())],
        origin: Origin::Pack,
        about: Some("Moonbrook".to_string()),
        depends_on: Some(Dependency::Foe("Edwin VanCleef".to_string())),
        setup_for: None,
    }
}

fn in_moonbrook() -> Character {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Westfall", Some("Moonbrook"))
        .unwrap();
    character
}

#[test]
fn an_unmet_outcome_never_passes_the_spoiler_limit() {
    let mut character = in_moonbrook();
    character.defeat_npc(Tick(2), "Mr. Smite").unwrap();

    assert!(!may_show(&character, &vancleef_falls()));
}

#[test]
fn an_outcome_passes_after_the_player_defeats_its_foe() {
    let mut character = in_moonbrook();
    character.defeat_npc(Tick(2), "Edwin VanCleef").unwrap();

    assert!(may_show(&character, &vancleef_falls()));
}

#[test]
fn a_quest_outcome_passes_after_the_player_turns_the_quest_in() {
    let mut character = Character::new();
    let linen = Dependency::Quest("Red Linen Goods".to_string());
    assert!(!outcome_allowed(&character, Some(&linen)));

    character
        .finish_game_quest(Tick(1), "Red Linen Goods", GameQuestKind::Normal)
        .unwrap();

    assert!(outcome_allowed(&character, Some(&linen)));
}

#[test]
fn an_unresolved_outcome_never_passes_whatever_the_player_did() {
    let mut character = in_moonbrook();
    character.defeat_npc(Tick(2), "Edwin VanCleef").unwrap();

    assert!(!outcome_allowed(&character, Some(&Dependency::Unresolved)));
}

#[test]
fn a_passage_with_no_deed_is_not_gated_by_the_outcome_rule() {
    assert!(outcome_allowed(&Character::new(), None));
}

#[test]
fn the_narrator_gets_the_lore_of_a_deed_only_after_the_player_did_it() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("outcome-narrator.sqlite");
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[vancleef_falls()]).unwrap();
    let pack = Pack::open(&path).unwrap();
    let seen = SeenIndex::new(&[]).unwrap();
    let mut character = in_moonbrook();

    let before = lore_about(&pack, &seen, &character, "Moonbrook").unwrap();
    character.defeat_npc(Tick(2), "Edwin VanCleef").unwrap();
    let after = lore_about(&pack, &seen, &character, "Moonbrook").unwrap();

    assert_eq!(before, None);
    assert_eq!(after, Some(vancleef_falls()));
}
