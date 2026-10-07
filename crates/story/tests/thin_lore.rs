#![allow(clippy::unwrap_used)]

mod wiki_dump;

use hourglass::Tick;
use std::path::Path;
use timeways_story::character::Character;
use timeways_story::moments::Moment;
use timeways_story::narrator::Who;
use timeways_story::narrator_lore::{is_thin, lore_of_moment, lore_subjects};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::pack_sources::{Sources, from_dump};
use timeways_story::race_class::{Class, Race};
use timeways_story::seen::{SeenIndex, SeenText, TextKind};
use wiki_dump::{article, fresh, long, write_dump};

fn passage(text: &str, about: Option<&str>, links: Vec<Link>) -> Passage {
    Passage {
        text: text.to_string(),
        source: "a test".to_string(),
        links,
        origin: Origin::Pack,
        about: about.map(str::to_string),
    }
}

fn gath() -> Moment {
    Moment::FirstKill {
        foe: "Gath'Ilzogg".to_string(),
        zone: None,
        creature: None,
    }
}

fn thin(moment: &Moment, who: &Who, lore: Option<&Passage>) -> bool {
    is_thin(&lore_subjects(moment, who), lore)
}

#[test]
fn a_kill_with_no_lore_is_thin() {
    assert!(thin(&gath(), &Who::default(), None));
}

#[test]
fn a_kill_with_lore_about_another_subject_is_thin() {
    let redridge = passage(
        "The Blackrock orcs raid the farms of Redridge from Stonewatch Keep.",
        Some("Redridge Mountains"),
        vec![Link::Place("Redridge Mountains".to_string())],
    );

    assert!(thin(&gath(), &Who::default(), Some(&redridge)));
}

#[test]
fn a_kill_with_lore_that_names_the_foe_is_not_thin() {
    let lore = passage(
        "Gath'Ilzogg led the Blackrock orcs into Stonewatch Keep.",
        None,
        Vec::new(),
    );

    assert!(!thin(&gath(), &Who::default(), Some(&lore)));
}

#[test]
fn a_passage_linked_to_the_foe_is_about_it() {
    let lore = passage(
        "The orcs took the keep above the lake.",
        None,
        vec![Link::Npc("Gath'Ilzogg".to_string())],
    );

    assert!(!thin(&gath(), &Who::default(), Some(&lore)));
}

#[test]
fn a_name_past_the_cut_of_the_prompt_does_not_count() {
    let filler = "The orcs held the keep for years. ".repeat(20);
    let lore = passage(&format!("{filler}Gath'Ilzogg led them."), None, Vec::new());

    assert!(thin(&gath(), &Who::default(), Some(&lore)));
}

#[test]
fn a_big_upgrade_with_no_item_story_is_thin() {
    let upgrade = Moment::BigUpgrade {
        item: "Gutwrencher".to_string(),
        zone: Some("Searing Gorge".to_string()),
        slot: None,
    };
    let gorge = passage(
        "The Dark Iron dwarves hold the Searing Gorge.",
        Some("Searing Gorge"),
        vec![Link::Place("Searing Gorge".to_string())],
    );

    assert!(thin(&upgrade, &Who::default(), Some(&gorge)));
}

#[test]
fn a_tenth_level_takes_the_lore_of_the_people_of_the_hero() {
    let level = Moment::LevelUp {
        level: 30,
        zone: Some("Stranglethorn Vale".to_string()),
    };
    let orc = Who {
        race: Some(Race::Orc),
        class: Some(Class::Warlock),
        titles: Vec::new(),
    };
    let jungle = passage(
        "The Gurubashi trolls once ruled Stranglethorn Vale.",
        Some("Stranglethorn Vale"),
        Vec::new(),
    );
    let orgrimmar = passage(
        "Thrall built Orgrimmar and named it after Orgrim Doomhammer.",
        Some("Orgrimmar"),
        Vec::new(),
    );

    assert!(lore_subjects(&level, &orc).contains(&"Orgrimmar".to_string()));
    assert!(thin(&level, &orc, Some(&jungle)));
    assert!(!thin(&level, &orc, Some(&orgrimmar)));
}

#[test]
fn a_tenth_level_with_no_race_has_no_subject_and_is_thin() {
    let level = Moment::LevelUp {
        level: 10,
        zone: None,
    };
    let any = passage("Thrall built Orgrimmar.", Some("Orgrimmar"), Vec::new());

    assert!(thin(&level, &Who::default(), Some(&any)));
}

#[test]
fn a_title_has_no_lore_and_is_thin() {
    let title = Moment::Titled {
        title: "Bookworm".to_string(),
    };

    assert!(title.is_deed());
    assert!(thin(&title, &Who::default(), None));
}

#[test]
fn an_arrival_and_a_flavor_moment_are_no_deeds() {
    let arrival = Moment::NewZone {
        zone: "Westfall".to_string(),
    };
    let flavor = Moment::Flavor {
        what: "The player danced.".to_string(),
        book: None,
    };

    assert!(!arrival.is_deed());
    assert!(!flavor.is_deed());
}

fn human() -> Who {
    Who {
        race: Some(Race::Human),
        class: Some(Class::Paladin),
        titles: Vec::new(),
    }
}

fn level(level: i64) -> Moment {
    Moment::LevelUp { level, zone: None }
}

/// A pack with three passages of the own page of Stormwind City.
fn stormwind_pack(name: &str) -> Pack {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("thin-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let page = |text: &str| Passage {
        text: text.to_string(),
        source: "the wiki page \"Stormwind City\"".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: Some("Stormwind City".to_string()),
    };
    let passages = [
        page("King Barathen Wrynn scattered the gnolls, and his line rules Stormwind City."),
        page("Garona killed King Llane in his keep, and the Horde burned Stormwind City."),
        page("The stonemasons rebuilt Stormwind City after the Second War."),
    ];
    Pack::write(&path, &passages).unwrap();
    Pack::open(&path).unwrap()
}

fn lore_at(pack: &Pack, moment: &Moment) -> Option<String> {
    let seen = SeenIndex::new(&[]).unwrap();
    lore_of_moment(pack, &seen, &Character::new(), moment, &human())
        .unwrap()
        .map(|passage| passage.text)
}

#[test]
fn two_tenth_levels_never_use_one_passage() {
    let pack = stormwind_pack("two-levels");

    let tenth = lore_at(&pack, &level(10)).unwrap();
    let twentieth = lore_at(&pack, &level(20)).unwrap();
    let thirtieth = lore_at(&pack, &level(30)).unwrap();

    assert!(tenth.contains("Barathen"), "{tenth}");
    assert!(twentieth.contains("Llane"), "{twentieth}");
    assert!(thirtieth.contains("stonemasons"), "{thirtieth}");
}

#[test]
fn a_tenth_level_with_the_lore_of_its_people_is_not_thin() {
    let pack = stormwind_pack("people");
    let seen = SeenIndex::new(&[]).unwrap();
    let moment = level(40);

    let lore = lore_of_moment(&pack, &seen, &Character::new(), &moment, &human()).unwrap();

    assert!(!is_thin(&lore_subjects(&moment, &human()), lore.as_ref()));
}

#[test]
fn a_tenth_level_with_no_passage_is_silent() {
    let pack = Pack::empty().unwrap();
    let moment = level(20);

    let lore = lore_at(&pack, &moment);

    assert_eq!(lore, None);
    assert!(is_thin(&lore_subjects(&moment, &human()), None));
}

#[test]
fn the_quest_text_that_the_player_read_is_about_its_quest() {
    let read = SeenText {
        kind: TextKind::Quest,
        title: Some("The Tome of Divinity".to_string()),
        npc: Some("Brother Sarno".to_string()),
        zone: Some("Stormwind City".to_string()),
        text: "Seek out the tome, and learn what the Light asks of you.".to_string(),
    };
    let moment = Moment::ClassQuestDone {
        title: "The Tome of Divinity".to_string(),
    };

    let passage = read.passage();

    assert!(!is_thin(&lore_subjects(&moment, &human()), Some(&passage)));
}

const PEOPLE_SOURCES: &str = r#"
[books]
index = "Test History"
chapters = []

[[pages]]
title = "Undercity"
sections = ["History"]
common = true

[[pages]]
title = "Forsaken"
sections = ["History"]
common = true

[[pages]]
title = "Darkspear tribe"
sections = ["History"]
places = ["Durotar"]
"#;

/// A pack built from invented pages of the Undercity, the Forsaken, and the Darkspear.
fn people_pack(name: &str) -> Pack {
    let page = |text: &str| format!("Lead.\n==History==\n{}\n", long(text));
    let undercity = page("The Undercity was dug beneath the old capital of the test kingdom.");
    let forsaken = page("The Forsaken broke free of the test lich and chose a test queen.");
    let darkspear = page("The Darkspear left their test isles to follow the test warchief.");
    let dump = write_dump(
        name,
        &[
            article("Test History", "Intro."),
            article("Undercity", &undercity),
            article("Forsaken", &forsaken),
            article("Darkspear tribe", &darkspear),
        ],
    );
    let built = from_dump(&dump, &Sources::parse(PEOPLE_SOURCES).unwrap()).unwrap();
    let path = fresh(&format!("{name}.sqlite"));
    Pack::write(&path, &built.passages).unwrap();
    Pack::open(&path).unwrap()
}

/// A hero who has been in Durotar, where every troll starts.
fn lore_of(pack: &Pack, moment: &Moment, race: Race) -> Option<Passage> {
    let seen = SeenIndex::new(&[]).unwrap();
    let mut character = Character::new();
    character.enter_zone(Tick(1), "Durotar", None).unwrap();
    let who = Who {
        race: Some(race),
        class: None,
        titles: Vec::new(),
    };
    lore_of_moment(pack, &seen, &character, moment, &who).unwrap()
}

#[test]
fn a_forsaken_tenth_level_gets_a_passage_of_its_own_pages() {
    let pack = people_pack("people-forsaken");

    let lore = lore_of(&pack, &level(10), Race::Forsaken).unwrap();

    assert!(lore.text.contains("The Undercity was dug"), "{}", lore.text);
    assert_eq!(lore.about.as_deref(), Some("Undercity"));
}

#[test]
fn an_undercity_arrival_gets_the_passage_of_the_undercity_page() {
    let pack = people_pack("people-undercity");
    let arrival = Moment::NewZone {
        zone: "Undercity".to_string(),
    };

    let lore = lore_of(&pack, &arrival, Race::Forsaken).unwrap();

    assert_eq!(lore.about.as_deref(), Some("Undercity"));
}

#[test]
fn a_troll_tenth_level_gets_the_passage_of_the_darkspear_page() {
    let pack = people_pack("people-troll");

    let lore = lore_of(&pack, &level(10), Race::Troll).unwrap();

    assert!(lore.text.contains("The Darkspear left"), "{}", lore.text);
}
