#![allow(clippy::unwrap_used)]

use timeways_story::line_check::{Grounds, LineFault};
use timeways_story::moments::{Moment, SlotKind};
use timeways_story::narrator::{Telling, Who};
use timeways_story::narrator_build::{Answered, Built, Offer, Setup, answered, offer, setup_foe};
use timeways_story::narrator_templates::Kind;
use timeways_story::pack::{Deed, Link, Origin, Passage, SetupFor};
use timeways_story::places::InstanceKind;
use timeways_story::race_class::{Class, Race};

fn who(race: Race, class: Class) -> Who {
    Who {
        race: Some(race),
        class: Some(class),
        titles: vec!["Bookworm".to_string()],
    }
}

fn setup(moment: Moment, who: Who, turn: usize) -> Setup {
    Setup {
        moment,
        who,
        turn,
        recent: Vec::new(),
        setup_foe: None,
    }
}

/// The verdict on `answer` for the moment of `setup`, told from `lore`.
fn told(setup: &Setup, lore: &str, answer: &str) -> (Offer, Answered) {
    let offer = offer(setup).unwrap();
    let telling = Telling {
        moment: &setup.moment,
        lore: Some(lore),
        who: &setup.who,
    };
    let grounds = Grounds::of(&telling, setup.turn);
    let verdict = answered(answer, setup, &offer, &grounds, "");
    (offer, verdict)
}

fn line(verdict: Answered) -> Built {
    match verdict {
        Answered::Line(built) => built,
        other => panic!("expected a line, got {other:?}"),
    }
}

fn faults(verdict: Answered) -> Vec<LineFault> {
    match verdict {
        Answered::Refused(faults) => faults,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn hogger() -> Moment {
    Moment::FirstKill {
        foe: "Hogger".to_string(),
        zone: Some("Elwynn Forest".to_string()),
        creature: None,
    }
}

const HOGGER_LORE: &str =
    "Hogger leads the Riverpaw gnolls of Elwynn Forest, and Stormwind has a price on his head.";
const HOGGER: &str = "Hogger and his Riverpaw gnolls raided Elwynn Forest for years, and Stormwind still offers gold for his head.";

#[test]
fn an_arrival_answer_becomes_its_lore_alone() {
    let durotar = Moment::NewZone {
        zone: "Durotar".to_string(),
    };
    let lore = "Thrall named Durotar after his father, Durotan. The orcs who settled its red canyons hold them now.";
    let history = "Thrall named Durotar for a father he never knew. The orcs who spent years in human camps hold its red canyons now.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (_, verdict) = told(&setup(durotar, Who::default(), 0), lore, &answer);

    let built = line(verdict);
    assert_eq!(built.line, history);
    assert_eq!(built.shape, "f.place");
}

/// The checks read the history on one line, so the line that the player sees must be
/// that line too, with no line break and no run of spaces.
#[test]
fn a_history_with_a_line_break_shows_on_one_line() {
    let durotar = Moment::NewZone {
        zone: "Durotar".to_string(),
    };
    let lore = "Thrall named Durotar after his father, Durotan. The orcs who settled its red canyons hold them now.";
    let answer = "{\"lore\": \"Thrall named Durotar for a father he never knew.\\n\\nThe orcs who \
                  spent years in human camps  hold its red canyons now.\"}";

    let (_, verdict) = told(&setup(durotar, Who::default(), 0), lore, answer);

    let built = line(verdict);
    assert_eq!(
        built.line,
        "Thrall named Durotar for a father he never knew. The orcs who spent years in human \
         camps hold its red canyons now."
    );
}

#[test]
fn a_kill_prefers_an_unnamed_part_on_a_named_turn() {
    let answer = format!(
        "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"the Riverpaw\", \"leads_number\": \"many\"}}"
    );

    let (_, verdict) = told(
        &setup(hogger(), who(Race::Human, Class::Paladin), 0),
        HOGGER_LORE,
        &answer,
    );

    let built = line(verdict);
    assert!(!built.line.contains("$N"), "{}", built.line);
    assert!(built.line.starts_with(HOGGER), "{}", built.line);
}

/// A review run told a deed history of 2 sentences "Use 1 to 3." (2026-10-06): the reason
/// must name the limit that the check holds.
#[test]
fn a_deed_history_of_two_sentences_is_told_to_write_one() {
    let answer = "{\"lore\": \"Hogger and his gnolls raided Elwynn Forest for years. Stormwind still offers gold for his head.\", \"there\": false, \"leads\": \"none\", \"leads_number\": \"one\"}";

    let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, answer);

    let faults = faults(verdict);
    assert!(
        faults.contains(&LineFault::HistorySentences(2, 1)),
        "{faults:?}"
    );
    let reasons: Vec<String> = faults.iter().map(ToString::to_string).collect();
    assert!(
        reasons
            .contains(&"The history has 2 sentences. Write one sentence of history.".to_string()),
        "{reasons:?}"
    );
}

#[test]
fn the_model_never_names_the_hero() {
    let answer = "{\"lore\": \"Hogger raided the farms of Elwynn until $N came.\", \"there\": false, \"leads\": \"none\", \"leads_number\": \"one\"}";

    let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, answer);

    assert!(faults(verdict).contains(&LineFault::HeroInHistory));
}

#[test]
fn a_choice_outside_the_list_is_refused() {
    let answer = format!(
        "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"none\", \"leads_number\": \"several\"}}"
    );

    let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, &answer);

    assert_eq!(
        faults(verdict),
        [LineFault::UnknownChoice("leads_number".to_string())]
    );
}

#[test]
fn there_needs_the_lore_to_name_the_zone() {
    let lore =
        "Hogger led the Riverpaw gnolls in raids, and Stormwind still has a price on his head.";
    let answer = format!(
        "{{\"lore\": \"{lore}\", \"there\": true, \"leads\": \"none\", \"leads_number\": \"one\"}}"
    );

    let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, &answer);

    assert!(faults(verdict).contains(&LineFault::ChoiceNotInLore("there".to_string())));
}

#[test]
fn leads_needs_the_lore_to_name_the_group() {
    let answer = format!(
        "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"the Defias\", \"leads_number\": \"many\"}}"
    );

    let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, &answer);

    assert!(faults(verdict).contains(&LineFault::ChoiceNotInLore("leads".to_string())));
}

/// The group goes into the line as the model wrote it. A review found "The Riverpaw! Have
/// lost their leader." and "Gnolls raided Elwynn Forest have lost their leader." built from
/// words that the history holds.
#[test]
fn leads_is_a_name_as_the_history_writes_it() {
    for leads in [
        "the Riverpaw!",
        "the riverpaw",
        "gnolls raided Elwynn Forest",
        "the Riverpaw, Hogger",
    ] {
        let answer = format!(
            "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"{leads}\", \"leads_number\": \"many\"}}"
        );

        let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, &answer);

        assert!(
            faults(verdict).contains(&LineFault::ChoiceNotInLore("leads".to_string())),
            "{leads}"
        );
    }
}

#[test]
fn leads_takes_a_group_of_several_words_with_or_without_the() {
    for leads in ["the Riverpaw", "Riverpaw gnolls", "the Riverpaw gnolls"] {
        let answer = format!(
            "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"{leads}\", \"leads_number\": \"many\"}}"
        );

        let (_, verdict) = told(&setup(hogger(), Who::default(), 0), HOGGER_LORE, &answer);

        assert!(matches!(verdict, Answered::Line(_)), "{leads}: {verdict:?}");
    }
}

fn ram() -> Moment {
    Moment::FirstMount {
        mount: "Gray Ram".to_string(),
        people: Some("Ironforge".to_string()),
    }
}

const RAM_LORE: &str = "The Mountaineers of Ironforge patrol the passes of Khaz Modan on rams bred in the snows of Dun Morogh.";

#[test]
fn breed_needs_the_lore_to_name_the_breed() {
    let lore = "The Mountaineers of Ironforge have held the passes of Khaz Modan since the Second War, and they hold them now.";
    let answer = format!("{{\"lore\": \"{lore}\", \"breed\": true}}");

    let (_, verdict) = told(
        &setup(ram(), who(Race::Dwarf, Class::Hunter), 0),
        RAM_LORE,
        &answer,
    );

    assert!(faults(verdict).contains(&LineFault::ChoiceNotInLore("breed".to_string())));
}

#[test]
fn a_mount_with_its_breed_can_tell_one_such_ram() {
    let lore = "The Mountaineers of Ironforge patrol the passes of Khaz Modan on rams bred in the snows of Dun Morogh.";
    let answer = format!("{{\"lore\": \"{lore}\", \"breed\": true}}");
    let mut seen = Vec::new();

    for turn in 0..8 {
        let (_, verdict) = told(
            &setup(ram(), who(Race::Dwarf, Class::Hunter), turn),
            RAM_LORE,
            &answer,
        );
        seen.push(line(verdict).line);
    }

    assert!(
        seen.iter().any(|line| line.contains("One such ram")),
        "{seen:?}"
    );
    assert!(
        seen.iter().any(|line| line.contains("a Gray Ram")),
        "{seen:?}"
    );
}

fn level(level: i64) -> Moment {
    Moment::LevelUp {
        level,
        zone: Some("Westfall".to_string()),
    }
}

const SILVER_HAND_LORE: &str = "Uther the Lightbringer founded the Order of the Silver Hand in the Second War, to carry the Light into battle.";

#[test]
fn a_horde_paladin_never_gets_the_silver_hand() {
    let (orc, _) = told(
        &setup(level(10), who(Race::Orc, Class::Paladin), 0),
        SILVER_HAND_LORE,
        "SILENCE",
    );
    let (human, _) = told(
        &setup(level(10), who(Race::Human, Class::Paladin), 0),
        SILVER_HAND_LORE,
        "SILENCE",
    );

    assert!(!orc.group_ids().contains(&"o.silver_hand".to_string()));
    assert!(orc.group_ids().contains(&"s.light".to_string()));
    assert!(human.group_ids().contains(&"o.silver_hand".to_string()));
    assert!(orc.strange);
    assert!(!human.strange);
}

#[test]
fn a_forsaken_paladin_gets_the_dead_who_wield_the_light() {
    let (offer, _) = told(
        &setup(level(20), who(Race::Forsaken, Class::Paladin), 0),
        SILVER_HAND_LORE,
        "SILENCE",
    );

    let dead = offer
        .groups
        .iter()
        .find(|group| group.id == "s.dead_light")
        .unwrap();
    assert_eq!(dead.text, "the dead who wield the Light");
    assert!(!offer.group_ids().contains(&"s.light".to_string()));
}

#[test]
fn a_group_takes_its_short_form_when_the_lore_names_it() {
    let lore = "The paladins of the Silver Hand took their vows from Uther, and they carried the Light against the Scourge.";
    let answer = format!("{{\"lore\": \"{lore}\", \"group\": \"o.silver_hand\"}}");

    let (_, verdict) = told(
        &setup(level(30), who(Race::Human, Class::Paladin), 0),
        SILVER_HAND_LORE,
        &answer,
    );

    let built = line(verdict);
    assert!(
        built.line.contains("the order") || built.line.contains("The order"),
        "{}",
        built.line
    );
    assert_eq!(
        built.line.matches("Silver Hand").count(),
        1,
        "{}",
        built.line
    );
}

#[test]
fn a_group_needs_the_lore_to_name_its_anchor() {
    let lore = "Uther the Lightbringer led the paladins of Lordaeron against the Horde in the Second War, and their order still stands.";
    let answer = format!("{{\"lore\": \"{lore}\", \"group\": \"o.silver_hand\"}}");

    let (_, verdict) = told(
        &setup(level(30), who(Race::Human, Class::Paladin), 0),
        SILVER_HAND_LORE,
        &answer,
    );

    assert!(faults(verdict).contains(&LineFault::ChoiceNotInLore("group".to_string())));
}

#[test]
fn a_singular_group_takes_a_singular_verb() {
    let lore = "King Barathen Wrynn scattered the gnoll packs of Elwynn, and his line rules Stormwind from its keep now.";
    let answer = format!("{{\"lore\": \"{lore}\", \"group\": \"g.people\"}}");
    let verbs = ["grows", "gains", "is stronger", "stands", "has grown"];

    for turn in 0..8 {
        let (_, verdict) = told(
            &setup(level(30), who(Race::Human, Class::Paladin), turn),
            lore,
            &answer,
        );
        let built = line(verdict);
        assert!(
            verbs.iter().any(|verb| built.line.contains(verb)),
            "{}",
            built.line
        );
    }
}

#[test]
fn now_never_shows_twice_in_a_line() {
    let lore = "King Barathen Wrynn scattered the gnoll packs of Elwynn, and his line rules Stormwind from its keep now.";
    let answer = format!("{{\"lore\": \"{lore}\", \"group\": \"g.people\"}}");

    for turn in 0..64 {
        let (_, verdict) = told(
            &setup(level(30), who(Race::Human, Class::Paladin), turn),
            lore,
            &answer,
        );
        let built = line(verdict);
        let deed = built.line.strip_prefix(lore).unwrap();
        let nows = deed.to_lowercase().matches("now").count();
        assert!(nows <= 1, "{}", built.line);
    }
}

#[test]
fn an_unnamed_turn_builds_no_hero() {
    let answer = format!(
        "{{\"lore\": \"{HOGGER}\", \"there\": false, \"leads\": \"none\", \"leads_number\": \"one\"}}"
    );
    // Turn 2 of the naming rotation names nobody.
    let (_, verdict) = told(
        &setup(hogger(), who(Race::Human, Class::Paladin), 2),
        HOGGER_LORE,
        &answer,
    );

    assert!(!line(verdict).line.contains("$N"));
}

#[test]
fn a_title_moment_never_names_the_hero_by_the_title() {
    let titled = Moment::Titled {
        title: "Bookworm".to_string(),
    };
    let lore = "The Royal Library of Stormwind keeps the histories of Arathor, and its scholars still copy them by hand.";
    let answer = format!("{{\"lore\": \"{lore}\"}}");

    for turn in 0..8 {
        let (_, verdict) = told(
            &setup(titled.clone(), who(Race::Human, Class::Paladin), turn),
            lore,
            &answer,
        );
        let built = line(verdict);
        assert!(!built.line.contains("the Bookworm"), "{}", built.line);
    }
}

#[test]
fn a_dry_tone_with_no_dry_shape_takes_a_plain_one() {
    let slap = Moment::Slapped {
        npc: "Innkeeper Farley".to_string(),
        times: 1,
        zone: Some("Elwynn Forest".to_string()),
    };
    let lore = "Goldshire grew up around the Lion's Pride Inn on the road to Stormwind, and its innkeeper still keeps the rooms.";
    let answer = format!("{{\"lore\": \"{lore}\", \"tone\": \"dry\", \"there\": false}}");

    let (_, verdict) = told(
        &setup(slap, who(Race::Human, Class::Paladin), 2),
        lore,
        &answer,
    );

    let built = line(verdict);
    assert!(!built.line.contains("slaps for"), "{}", built.line);
}

#[test]
fn the_lore_budget_leaves_room_for_the_longest_shape() {
    let long = "Lord Commander Highlord Grand Marshal Garithos of the Ruins of Lordaeron Keep";
    let kill = Moment::FirstKill {
        foe: long.to_string(),
        zone: Some("Western Plaguelands".to_string()),
        creature: None,
    };

    let offer = offer(&setup(kill, who(Race::Human, Class::Paladin), 0)).unwrap();

    assert!(
        offer.budget + 1 + 2 * long.len() + "In Western Plaguelands, ".len() <= 300 + long.len()
    );
    assert!(offer.budget < 300 - long.len());
}

#[test]
fn an_item_with_no_known_slot_has_no_shape_and_is_silent() {
    let item = Moment::FirstEpicItem {
        item: "Gutwrencher".to_string(),
        zone: None,
        slot: None,
    };

    assert_eq!(offer(&setup(item, Who::default(), 0)), None);
}

#[test]
fn a_weapon_is_carried_and_armor_is_worn() {
    let lore = "Gutwrencher came from the Blackrock Depths, where the Dark Iron dwarves still forge for Ragnaros.";
    let answer = format!("{{\"lore\": \"{lore}\", \"there\": false}}");
    let item = |slot| Moment::BigUpgrade {
        item: "Gutwrencher".to_string(),
        zone: None,
        slot: Some(slot),
    };

    let (_, weapon) = told(
        &setup(item(SlotKind::Weapon), Who::default(), 0),
        lore,
        &answer,
    );
    let (_, worn) = told(
        &setup(item(SlotKind::Worn), Who::default(), 0),
        lore,
        &answer,
    );

    assert!(!line(weapon).line.contains("wears"));
    assert!(line(worn).line.contains("wears"));
}

const DEPTHS: &str =
    "The Dark Iron dwarves forge weapons in the Blackrock Depths for Ragnaros, their master.";

fn worn(item: &str) -> Setup {
    let moment = Moment::BigUpgrade {
        item: item.to_string(),
        zone: None,
        slot: Some(SlotKind::Worn),
    };
    setup(moment, Who::default(), 0)
}

fn depths_answer(history: &str) -> String {
    format!("{{\"lore\": \"{history}\", \"there\": false}}")
}

#[test]
fn a_slop_word_in_the_name_of_an_item_passes_in_its_slot() {
    let (_, verdict) = told(&worn("Dawn's Edge"), DEPTHS, &depths_answer(DEPTHS));

    assert!(line(verdict).line.contains("Dawn's Edge"));
}

#[test]
fn the_name_of_an_item_never_unlocks_a_cutoff_name() {
    let (_, verdict) = told(&worn("Edge of Pandaria"), DEPTHS, &depths_answer(DEPTHS));

    assert!(faults(verdict).contains(&LineFault::LaterName("Pandaria".to_string())));
}

const HAND: &str = "The Silver Hand trains its paladins in the Cathedral of Light in Stormwind.";

fn class_quest(title: &str) -> Setup {
    let moment = Moment::ClassQuestDone {
        title: title.to_string(),
    };
    setup(moment, who(Race::Human, Class::Paladin), 0)
}

#[test]
fn a_slop_word_in_a_quest_title_passes_in_its_slot() {
    let answer = format!("{{\"lore\": \"{HAND}\"}}");

    let (_, verdict) = told(&class_quest("The Legendary Hammer"), HAND, &answer);

    assert!(line(verdict).line.contains("The Legendary Hammer"));
}

#[test]
fn a_quest_title_never_unlocks_its_slop_word_in_the_history() {
    let history = "The Silver Hand keeps a legendary hammer in the Cathedral of Light.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (_, verdict) = told(&class_quest("The Legendary Hammer"), HAND, &answer);

    assert!(faults(verdict).contains(&LineFault::Banned("legendary".to_string())));
}

#[test]
fn a_quest_title_never_unlocks_a_cutoff_name() {
    let answer = format!("{{\"lore\": \"{HAND}\"}}");

    let (_, verdict) = told(&class_quest("The Road to Pandaria"), HAND, &answer);

    assert!(faults(verdict).contains(&LineFault::LaterName("Pandaria".to_string())));
}

#[test]
fn the_name_of_an_item_never_unlocks_its_slop_word_in_the_history() {
    let history = "The Dark Iron dwarves forge at dawn in the Blackrock Depths for Ragnaros.";

    let (_, verdict) = told(&worn("Dawn's Edge"), DEPTHS, &depths_answer(history));

    assert!(faults(verdict).contains(&LineFault::Banned("dawn".to_string())));
}

const MUTANUS_LORE: &str = "Manifested from Naralex's Nightmare, Mutanus the Devourer has been \
    ordered by whatever force has infected Naralex's dreams to prevent anyone from awakening him.";

fn wailing_caverns_setup(foe: Option<&str>) -> Setup {
    let moment = Moment::FirstInstance {
        zone: "Wailing Caverns".to_string(),
        kind: InstanceKind::Dungeon,
    };
    Setup {
        setup_foe: foe.map(str::to_string),
        ..setup(moment, Who::default(), 0)
    }
}

#[test]
fn a_setup_entry_ends_on_the_code_coda() {
    let history = "Some force infected the dreams of Naralex in the Wailing Caverns, and it sent \
                   Mutanus the Devourer to keep anyone from waking him.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (offer, verdict) = told(
        &wailing_caverns_setup(Some("Mutanus the Devourer")),
        MUTANUS_LORE,
        &answer,
    );

    let built = line(verdict);
    assert_eq!(offer.kind, Kind::Setup);
    assert_eq!(
        built.line,
        format!("{history} Mutanus the Devourer is still alive.")
    );
    assert_eq!(built.shape, "sc.alive");
}

#[test]
fn a_setup_coda_names_the_tagged_foe() {
    let history = "Some force infected the dreams of Naralex in the Wailing Caverns.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (_, verdict) = told(
        &wailing_caverns_setup(Some("Lady Anacondra")),
        MUTANUS_LORE,
        &answer,
    );

    assert!(
        line(verdict)
            .line
            .ends_with("Lady Anacondra is still alive.")
    );
}

#[test]
fn an_arrival_with_no_setup_foe_gets_no_coda() {
    let history = "Some force infected the dreams of Naralex in the Wailing Caverns.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (offer, verdict) = told(&wailing_caverns_setup(None), MUTANUS_LORE, &answer);

    assert_eq!(offer.kind, Kind::Arrival);
    assert_eq!(line(verdict).line, history);
}

#[test]
fn a_setup_history_that_tells_who_holds_the_place_is_refused() {
    let history = "Some force sent Mutanus the Devourer into the Wailing Caverns, and he still \
                   holds them.";
    let answer = format!("{{\"lore\": \"{history}\"}}");

    let (_, verdict) = told(
        &wailing_caverns_setup(Some("Mutanus the Devourer")),
        MUTANUS_LORE,
        &answer,
    );

    assert!(
        faults(verdict)
            .iter()
            .any(|fault| matches!(fault, LineFault::UnsourcedPresent(_)))
    );
}

#[test]
fn a_setup_of_a_quest_gets_no_coda() {
    let moment = Moment::FirstInstance {
        zone: "The Stockade".to_string(),
        kind: InstanceKind::Dungeon,
    };
    let passage = Passage {
        text: "Warden Thelwater enlisted adventurers to end the riot.".to_string(),
        source: "the wiki page \"Stormwind Stockade\"".to_string(),
        links: vec![Link::Place("The Stockade".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: Vec::new(),
        setup_for: Some(SetupFor {
            deed: Deed::Quest("What Comes Around...".to_string()),
            instance: "The Stockade".to_string(),
        }),
    };
    let foe_passage = Passage {
        setup_for: Some(SetupFor {
            deed: Deed::Foe("Bazil Thredd".to_string()),
            instance: "The Stockade".to_string(),
        }),
        ..passage.clone()
    };

    assert_eq!(setup_foe(&moment, Some(&passage)), None);
    assert_eq!(
        setup_foe(&moment, Some(&foe_passage)).as_deref(),
        Some("Bazil Thredd")
    );
}
