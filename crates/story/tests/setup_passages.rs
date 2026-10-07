//! Setup passages: the detector of the builder, the deed and the instance of each one, the
//! window that a prompt shows, and the gate of the spoiler limit (GAMEPLAY.md 5.10). Every
//! page is invented, except the names of the game.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::collections::BTreeMap;
use timeways_story::character::Character;
use timeways_story::input::GameQuestKind;
use timeways_story::outcome_passages::{Chain, Npc, PageKind, Quest, Stance, Title};
use timeways_story::pack::{Deed, Link, Origin, Passage, SetupFor};
use timeways_story::setup_passages::{
    Found, Instances, setup_for, setup_sentences, tells_an_end, window,
};
use timeways_story::spoiler::{may_show, setup_allowed};
use timeways_story::wikitext::Cites;

const STOUTMANTLE_ASKS: &str = "The Defias hid beneath Moonbrook. Gryan Stoutmantle sent \
                                adventurers to kill Edwin VanCleef in the Deadmines.";

fn npc(name: &str, stance: Stance) -> PageKind {
    PageKind::Npc(Npc {
        name: name.to_string(),
        stance,
    })
}

/// The pages of a small invented wiki.
fn kind_of(title: &str) -> Option<PageKind> {
    match title {
        "Edwin VanCleef" => Some(npc("Edwin VanCleef", Stance::Foe)),
        "Mr. Smite" => Some(npc("Mr. Smite", Stance::Foe)),
        "Moira Thaurissan" => Some(npc("Moira Thaurissan", Stance::Foe)),
        "Gryan Stoutmantle" => Some(npc("Gryan Stoutmantle", Stance::NoFoe)),
        "Test Raid" => Some(npc("Test Raider", Stance::Foe)),
        "Into the Testvault" => Some(PageKind::Quest(Quest {
            name: "Into the Testvault".to_string(),
            chain: Chain::End,
            title: Title::Own,
        })),
        _ => None,
    }
}

fn instances() -> Instances {
    let bosses = BTreeMap::from([
        (
            "Edwin VanCleef".to_string(),
            vec!["The Deadmines".to_string()],
        ),
        ("Mr. Smite".to_string(), vec!["The Deadmines".to_string()]),
    ]);
    Instances {
        names: vec!["The Deadmines".to_string(), "Testvault".to_string()],
        bosses,
    }
}

fn links(names: &[&str]) -> Cites {
    Cites {
        links: names.iter().map(ToString::to_string).collect(),
        refs: Vec::new(),
    }
}

fn found_setup(text: &str, cites: &Cites, page: &str, places: &[&str]) -> Option<SetupFor> {
    let links: Vec<Link> = places
        .iter()
        .map(|place| Link::Place((*place).to_string()))
        .collect();
    let found = Found {
        text,
        cites,
        page,
        links: &links,
    };
    setup_for(&found, &instances(), kind_of)
}

fn vancleef_setup() -> SetupFor {
    SetupFor {
        deed: Deed::Foe("Edwin VanCleef".to_string()),
        instance: "The Deadmines".to_string(),
    }
}

#[test]
fn someone_who_sent_adventurers_to_kill_a_foe_sets_up_a_deed() {
    let found = setup_sentences(STOUTMANTLE_ASKS);

    assert_eq!(
        found,
        vec!["Gryan Stoutmantle sent adventurers to kill Edwin VanCleef in the Deadmines."]
    );
}

#[test]
fn a_bounty_a_threat_and_a_plot_set_up_a_deed() {
    assert!(!setup_sentences("Stormwind put a bounty on the head of Edwin VanCleef.").is_empty());
    assert!(!setup_sentences("The Brotherhood threatens every farm of Westfall.").is_empty());
    assert!(!setup_sentences("VanCleef hatched a plan of revenge on the nobles.").is_empty());
    assert!(!setup_sentences("The magistrate wants VanCleef dead.").is_empty());
}

#[test]
fn a_foe_that_seeks_to_destroy_a_city_sets_up_a_deed() {
    assert!(!setup_sentences("VanCleef sought to overthrow the Kingdom of Stormwind.").is_empty());
}

#[test]
fn a_journey_to_a_place_is_no_setup() {
    assert!(setup_sentences("The king sent his son to Stormwind for the winter.").is_empty());
    assert!(setup_sentences("Gryan sent them to the Deadmines with a letter.").is_empty());
}

#[test]
fn an_order_with_no_deed_against_a_foe_is_no_setup() {
    assert!(
        setup_sentences("Blackhand ordered the shaman to transform his young sons into soldiers.")
            .is_empty()
    );
}

#[test]
fn a_plot_of_land_is_no_plot() {
    assert!(setup_sentences("The farmer bought a plot of land near Goldshire.").is_empty());
}

#[test]
fn a_setup_after_the_end_of_the_deed_is_history() {
    let text = "Adventurers killed Edwin VanCleef. Gryan Stoutmantle sent adventurers to kill \
                Edwin VanCleef.";

    assert!(setup_sentences(text).is_empty());
}

#[test]
fn a_result_after_be_tells_no_end() {
    assert!(!tells_an_end(
        "The Crusade believes that outsiders must be destroyed."
    ));
    assert!(tells_an_end("Outsiders were destroyed in the square."));
}

#[test]
fn a_setup_names_its_foe_after_to_and_belongs_to_the_instance_of_the_foe() {
    let found = found_setup(
        STOUTMANTLE_ASKS,
        &links(&["Gryan Stoutmantle", "Edwin VanCleef"]),
        "Moonbrook",
        &["Westfall"],
    );

    assert_eq!(found, Some(vancleef_setup()));
}

#[test]
fn the_one_who_sends_is_never_the_foe() {
    let text = "Mr. Smite sent his crew to kill the miners of Gryan Stoutmantle.";

    let found = found_setup(
        text,
        &links(&["Mr. Smite", "Gryan Stoutmantle"]),
        "Moonbrook",
        &["The Deadmines"],
    );

    assert_eq!(found, None);
}

#[test]
fn of_two_foes_in_one_request_the_first_wins() {
    let text = "Gryan Stoutmantle sent adventurers to kill Edwin VanCleef and find Mr. Smite.";

    let found = found_setup(
        text,
        &links(&["Mr. Smite", "Edwin VanCleef"]),
        "Moonbrook",
        &["Westfall"],
    );

    assert_eq!(found, Some(vancleef_setup()));
}

#[test]
fn a_title_of_another_name_never_names_the_foe() {
    let text = "King Magni sent a team to kill Emperor Thaurissan in Blackrock Depths.";

    let found = found_setup(
        text,
        &links(&["Moira Thaurissan"]),
        "Moira Thaurissan",
        &["The Deadmines"],
    );

    assert_eq!(found, None);
}

#[test]
fn a_commission_with_no_foe_takes_the_quest_that_the_line_cites() {
    let text = "The Explorers' League sent a team to investigate the strange happenings below.";
    let cites = Cites {
        links: Vec::new(),
        refs: vec!["Into the Testvault".to_string()],
    };

    let found = found_setup(text, &cites, "Testvault", &["Testvault"]);

    assert_eq!(
        found,
        Some(SetupFor {
            deed: Deed::Quest("Into the Testvault".to_string()),
            instance: "Testvault".to_string(),
        })
    );
}

#[test]
fn a_setup_outside_every_instance_is_no_setup() {
    let text = "The guards sent adventurers to kill Test Raider in the hills.";

    let found = found_setup(text, &links(&["Test Raid"]), "Testhills", &["Testhills"]);

    assert_eq!(found, None);
}

#[test]
fn the_window_holds_the_setup_and_the_sentence_before_it() {
    let text = "Moonbrook was a mining town. The Defias hid beneath it. Gryan Stoutmantle sent \
                adventurers to kill Edwin VanCleef. The adventurers killed him.";

    let shown = window(text, &vancleef_setup().deed);

    assert_eq!(
        shown,
        Some(
            "The Defias hid beneath it. Gryan Stoutmantle sent adventurers to kill Edwin \
             VanCleef."
        )
    );
}

#[test]
fn a_setup_of_the_first_sentence_is_its_own_window() {
    let text = "VanCleef hatched a plan of revenge on the nobles of Stormwind. He waits below.";

    let shown = window(text, &vancleef_setup().deed);

    assert_eq!(
        shown,
        Some("VanCleef hatched a plan of revenge on the nobles of Stormwind.")
    );
}

fn stoutmantle_asks() -> Passage {
    Passage {
        text: STOUTMANTLE_ASKS.to_string(),
        source: "the wiki page \"Moonbrook\"".to_string(),
        links: vec![Link::Place("Westfall".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
        setup_for: Some(vancleef_setup()),
    }
}

fn in_westfall() -> Character {
    let mut character = Character::new();
    character.enter_zone(Tick(1), "Westfall", None).unwrap();
    character
}

#[test]
fn a_setup_shows_until_the_player_defeats_its_foe() {
    let mut character = in_westfall();
    let before = may_show(&character, &stoutmantle_asks());

    character.defeat_npc(Tick(2), "Edwin VanCleef").unwrap();

    assert!(before);
    assert!(!may_show(&character, &stoutmantle_asks()));
}

#[test]
fn a_setup_for_a_quest_goes_stale_after_its_turn_in() {
    let mut character = Character::new();
    let setup = SetupFor {
        deed: Deed::Quest("Into the Testvault".to_string()),
        instance: "Testvault".to_string(),
    };
    assert!(setup_allowed(&character, Some(&setup)));

    character
        .finish_game_quest(Tick(1), "Into the Testvault", GameQuestKind::Normal)
        .unwrap();

    assert!(!setup_allowed(&character, Some(&setup)));
}

#[test]
fn a_passage_with_no_setup_is_not_gated_by_the_setup_rule() {
    let mut character = Character::new();
    character.defeat_npc(Tick(1), "Edwin VanCleef").unwrap();

    assert!(setup_allowed(&character, None));
}

#[test]
fn a_window_too_long_for_a_prompt_keeps_the_setup_alone() {
    let before = format!("{}.", "The Defias took the mine".repeat(20));
    let text = format!("{before} Gryan Stoutmantle sent adventurers to kill Edwin VanCleef.");

    let shown = window(&text, &vancleef_setup().deed);

    assert_eq!(
        shown,
        Some("Gryan Stoutmantle sent adventurers to kill Edwin VanCleef.")
    );
}
