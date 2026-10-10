//! The checks of the style guide that hold for every narrator text
//! (docs/plans/narrator-style.md 10.1). Each rule has a refused line and a near miss that
//! passes, because a false refusal costs a good line.

use timeways_story::check::slop_in;
use timeways_story::inside_hero::{inside_hero_in, recognition_in};
use timeways_story::prose::{MOST_WORDS, ProseFault, prose_faults};

fn hero(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_string()).collect()
}

fn faults(text: &str) -> Vec<ProseFault> {
    prose_faults(text, &[])
}

fn words(count: usize) -> String {
    let mut sentence = vec!["word"; count].join(" ");
    sentence.push('.');
    sentence
}

#[test]
fn an_order_never_grows_inside_the_hero() {
    let druid = hero(&["druid"]);

    assert_eq!(
        inside_hero_in("Its power grows in the druid.", &druid).as_deref(),
        Some("grows in the druid")
    );
    assert_eq!(
        inside_hero_in("That old craft grows sharper in $N.", &[]).as_deref(),
        Some("grows sharper in n")
    );
    assert_eq!(
        inside_hero_in(
            "Naralex's dream of a green Barrens lives on in the hero.",
            &[]
        )
        .as_deref(),
        Some("lives on in the hero")
    );
}

#[test]
fn a_group_that_grows_stronger_beside_the_hero_passes() {
    let warlock = hero(&["forsaken", "warlock"]);

    for line in [
        "The warlocks of the Horde grow stronger. $N has reached level 20.",
        "The dwarves grew strong in Ironforge long before the Second War.",
        "The Light burns in the paladins of the Silver Hand.",
        "The plague still lives in the Forsaken's crypts beneath Brill.",
        "The plague lives in the Forsaken of the Undercity.",
        "Thrall grew up a slave in Durnholde Keep.",
    ] {
        assert_eq!(inside_hero_in(line, &warlock), None, "{line}");
    }
}

#[test]
fn a_race_or_class_word_before_a_name_is_a_person_of_the_lore() {
    let paladin = hero(&["paladin"]);

    assert_eq!(
        inside_hero_in("The Light burned in the paladin Uther.", &paladin),
        None
    );
    assert_eq!(
        recognition_in("Stormwind honors the paladin Uther.", &paladin),
        None
    );
}

#[test]
fn a_people_never_knows_the_hero() {
    let paladin = hero(&["paladin"]);

    assert_eq!(
        recognition_in("The Riverpaw remember the paladin.", &paladin).as_deref(),
        Some("remember the paladin")
    );
    assert_eq!(
        recognition_in(
            "The dwarves of Ironforge greet $N as one of their own.",
            &[]
        )
        .as_deref(),
        Some("greet n")
    );
}

#[test]
fn a_people_that_knows_another_people_passes() {
    let dwarf = hero(&["dwarf"]);

    for line in [
        "The Scarlet Crusade fears the Scourge more than any foe.",
        "Ironforge honors the dwarves who fell at Grim Batol.",
        "The night elves remember the War of the Ancients.",
    ] {
        assert_eq!(recognition_in(line, &dwarf), None, "{line}");
    }
}

#[test]
fn a_line_never_cites_its_source() {
    let line = "Apothecary Renferrel spoke of the Royal Apothecary Society's plague. In the \
        Undercity, its masters keep their vats below the throne.";

    assert!(slop_in(line, "").contains(&"spoke of"));
    assert!(
        faults(line)
            .iter()
            .any(|fault| matches!(fault, ProseFault::SourceShape(_))),
        "{line}"
    );
    for cited in [
        "As the tome tells it",
        "According to the dwarves",
        "It is said that",
    ] {
        assert!(
            !slop_in(&format!("{cited}, Ironforge stood."), "").is_empty(),
            "{cited}"
        );
    }
}

#[test]
fn a_place_after_a_plain_sentence_is_no_source_shape() {
    for line in [
        "Apothecary Renferrel brews in Brill. In the Undercity, its masters keep their vats.",
        "Renferrel spoke of the plague. In the Undercity, the masters keep their vats.",
    ] {
        let shapes: Vec<ProseFault> = faults(line)
            .into_iter()
            .filter(|fault| matches!(fault, ProseFault::SourceShape(_)))
            .collect();
        assert!(shapes.is_empty(), "{line}");
    }
}

#[test]
fn a_line_never_claims_fame() {
    for line in [
        "Twenty levels later, the Scarlet Crusade knows the name.",
        "The Riverpaw will remember the name of the paladin.",
        "The bards of Lakeshire sing songs of the deed.",
    ] {
        assert!(!slop_in(line, "").is_empty(), "{line}");
    }
    assert!(slop_in("The Scarlet Crusade knows the Scourge well.", "").is_empty());
}

#[test]
fn a_fragment_is_refused() {
    for (line, fragment) in [
        ("Level 10. The order grows stronger.", "Level 10."),
        ("Gone.", "Gone."),
    ] {
        assert!(
            faults(line).contains(&ProseFault::Fragment(fragment.to_string())),
            "{line}"
        );
    }
}

#[test]
fn a_sentence_of_four_words_is_no_fragment() {
    assert!(faults("Hogger is dead now.").is_empty());
    assert!(faults("Mr. Smite guards the ship.").is_empty());
}

#[test]
fn a_sentence_over_thirty_words_is_refused_and_thirty_pass() {
    assert!(faults(&words(MOST_WORDS)).is_empty());
    assert_eq!(
        faults(&words(MOST_WORDS + 1)),
        [ProseFault::LongSentence(["word"; 8].join(" "))]
    );
}

#[test]
fn the_not_x_but_y_pivot_is_refused() {
    let line = "The Defias are not bandits, but the stonemasons Stormwind never paid.";

    assert_eq!(
        faults(line),
        [ProseFault::Pivot("not bandits, but".to_string())]
    );
    assert!(slop_in("Westfall is not just farmland.", "").contains(&"not just"));
}

#[test]
fn a_but_before_not_or_far_from_it_is_no_pivot() {
    let far = format!("The keep did not fall {} but it burned.", "x ".repeat(31));
    for line in [
        "Arugal called the worgen into Silverpine to fight the Scourge, but he could not \
         control them.",
        "The Defias did not stop. But the militia held Sentinel Hill.",
        "Arugal cannot hold the keep, but his worgen do.",
        far.as_str(),
    ] {
        let pivots: Vec<ProseFault> = faults(line)
            .into_iter()
            .filter(|fault| matches!(fault, ProseFault::Pivot(_)))
            .collect();
        assert!(pivots.is_empty(), "{line}");
    }
}

#[test]
fn a_text_that_opens_with_the_level_is_refused() {
    for line in [
        "Level 10 came to the farms of Northshire Abbey.",
        "Level ten came on a muddy road at the edge of Elwynn.",
    ] {
        assert!(faults(line).contains(&ProseFault::LevelOpener), "{line}");
    }
}

/// A saga and a tale tell every level, not only each tenth.
#[test]
fn a_text_that_opens_with_any_level_in_words_is_refused() {
    for line in [
        "Level fifteen came on the road to the farms of Westfall.",
        "Level thirteen found the paladin in the mines of Redridge.",
        "Level fifty-five came in the Burning Steppes, below Blackrock Spire.",
    ] {
        assert!(faults(line).contains(&ProseFault::LevelOpener), "{line}");
    }
}

#[test]
fn a_level_later_in_the_text_passes() {
    for line in [
        "The Silver Hand once burned the dead of Lordaeron. $N has reached level 20.",
        "The level of Lake Everstill rose after the bridge of Lakeshire fell.",
    ] {
        assert!(faults(line).is_empty(), "{line}");
    }
}

#[test]
fn a_lame_deed_verb_is_refused_but_a_war_that_ended_passes() {
    assert_eq!(
        slop_in("Hogger raided Elwynn until $N ended him.", ""),
        ["ended him"]
    );
    for line in [
        "The Second War ended. It left the fields of Westfall fallow.",
        "When the Second War ended, it left the fields of Westfall fallow.",
    ] {
        assert!(slop_in(line, "").is_empty(), "{line}");
    }
}

#[test]
fn a_people_that_came_to_a_place_passes_but_came_into_is_refused() {
    assert!(slop_in("The Scourge came to Lordaeron and raised its dead.", "").is_empty());
    assert_eq!(
        slop_in("One more stranger came into those fields.", ""),
        ["one more stranger", "came into"]
    );
}

#[test]
fn a_place_that_acts_like_a_person_is_refused_unless_the_lore_holds_it() {
    assert_eq!(
        slop_in("Blackrock Mountain looms over the Burning Steppes.", ""),
        ["looms"]
    );
    assert!(
        slop_in(
            "The Crossroads stands where the roads of the Barrens meet.",
            ""
        )
        .is_empty()
    );
    let lore = "The Great Forge burns at the heart of Ironforge.";
    assert!(slop_in("The Great Forge burns at the heart of Ironforge.", lore).is_empty());
}

#[test]
fn a_place_with_a_heart_is_slop() {
    assert_eq!(
        slop_in("The Great Forge still burns at its heart.", ""),
        ["at its heart"]
    );
    assert_eq!(
        slop_in("The trolls keep their shrines at their heart.", ""),
        ["at their heart"]
    );
}

#[test]
fn a_heart_word_that_gives_no_body_is_no_slop() {
    assert!(slop_in("The Scarlet Crusade holds Hearthglen in the north.", "").is_empty());
    assert!(slop_in("The dwarves sit at its hearth in the evening.", "").is_empty());
    assert!(slop_in("The priests know the old prayers by heart.", "").is_empty());
    assert!(slop_in("The orcs took the heartland of Lordaeron.", "").is_empty());
}

#[test]
fn a_whole_word_of_a_name_is_no_slop() {
    assert!(slop_in("Tyrande Whisperwind leads the night elves.", "").is_empty());
    assert!(slop_in("The trolls of Echo Isles followed Thrall.", "").is_empty());
}

#[test]
fn each_fault_tells_the_retry_what_to_do() {
    let reasons = [
        ProseFault::InsideHero("grows in the druid".to_string()),
        ProseFault::Recognition("remember the paladin".to_string()),
        ProseFault::SourceShape("In the Undercity, its masters keep their vats.".to_string()),
        ProseFault::Fragment("Gone.".to_string()),
        ProseFault::LongSentence("word word".to_string()),
        ProseFault::Pivot("not bandits, but".to_string()),
        ProseFault::LevelOpener,
    ]
    .map(|fault| fault.to_string());

    assert!(reasons[0].contains("\"grows in the druid\""));
    assert!(reasons[3].contains("Write whole sentences"));
    assert!(reasons[6].contains("Open with the history"));
    assert!(reasons.iter().all(|reason| reason.ends_with('.')));
}

#[test]
fn a_text_never_names_the_hero_by_the_word_of_its_people() {
    let line = "Sylvanas created the Royal Apothecary Society to brew a new plague against the Scourge, and its alchemists still work in the Apothecarium of the Undercity. The Society gains strength, and the Forsaken has reached 225 in Alchemy.";

    assert_eq!(
        faults(line),
        [ProseFault::GroupWordForHero("the forsaken has".to_string())]
    );
}

#[test]
fn a_text_never_names_the_hero_by_a_word_that_names_a_group_of_it() {
    let line = "The paladins of the Silver Hand once burned the dead of Lordaeron, and the order is stronger now. The paladin has reached level 30.";

    assert_eq!(
        faults(line),
        [ProseFault::GroupWordForHero("the paladin has".to_string())]
    );
}

#[test]
fn a_text_never_names_the_hero_by_a_title_that_names_a_group_of_it() {
    let line = "The Bookworms of the Royal Library copy the histories of Arathor by hand. The Bookworm finished the last of them.";

    assert_eq!(
        prose_faults(line, &hero(&["Bookworm"])),
        [ProseFault::GroupWordForHero(
            "the bookworm finished".to_string()
        )]
    );
}

#[test]
fn a_text_can_name_the_hero_the_tauren_when_no_tauren_group_is_in_it() {
    let line = "Cairne Bloodhoof led his people out of the Barrens to Mulgore with the help of Thrall. The tauren has reached level 20.";

    assert_eq!(faults(line), []);
}

#[test]
fn a_text_never_names_the_hero_the_tauren_beside_the_tauren_of_mulgore() {
    let line = "The tauren of Mulgore hunt the plains with the help of the Horde. The tauren has reached level 20.";

    assert_eq!(
        faults(line),
        [ProseFault::GroupWordForHero("the tauren has".to_string())]
    );
}

#[test]
fn a_text_can_name_the_hero_the_shaman_when_no_shaman_group_is_in_it() {
    let line = "Thrall freed the orcs from the internment camps and led them across the sea to Kalimdor. The shaman has reached level 20.";

    assert_eq!(faults(line), []);
}

#[test]
fn a_people_with_no_word_for_the_hero_passes() {
    let lines = [
        "The Forsaken of the Undercity hold the ruins of Lordaeron, and their apothecaries brew a new plague there now.",
        "The Forsaken grow stronger, and $N has reached level 30 beneath the ruins of Lordaeron.",
        "The dwarf Muradin Bronzebeard led the dwarves of Ironforge into Northrend, and none of his party came back.",
        "The orc warlord Gul'dan sold the orcs to the demons, and the Horde drank the blood of Mannoroth.",
        "The paladin has reached level 30, and Stormwind gains strength while the Defias still hold Westfall.",
        "The Forsaken's apothecaries brew a new plague beneath the ruins of Lordaeron for the Dark Lady.",
    ];

    for line in lines {
        assert_eq!(faults(line), [], "{line}");
    }
}
