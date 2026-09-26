use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::flavor::{Flavor, Kind, TOLD_SECONDS, Told, score};

fn dance(at: u64, place: &str, hour: u8) -> Flavor {
    let kind = Kind::Emote {
        emote: "dance".to_string(),
        target: None,
    };
    Flavor {
        at: Tick(at),
        hour: Some(hour),
        place: Some(place.to_string()),
        kind,
    }
}

fn dances(count: usize) -> Vec<Flavor> {
    (0..count)
        .map(|n| dance(n as u64, "Westfall", 12))
        .collect()
}

#[test]
fn the_example_of_the_spec_scores_fourteen() {
    let cow = Flavor {
        at: Tick(100),
        hour: Some(3),
        place: Some("Goldshire".to_string()),
        kind: Kind::Humbled {
            killer: "Cow".to_string(),
            gap: 59,
        },
    };

    assert_eq!(score(&cow, &[], &[], &Character::new()), 5 + 4 + 3 + 2);
}

#[test]
fn the_thirtieth_of_a_kind_scores_one() {
    let thirtieth = dance(100, "Westfall", 12);

    assert_eq!(score(&thirtieth, &dances(29), &[], &Character::new()), 1);
}

#[test]
fn rare_for_you_falls_as_the_kind_grows_common() {
    let character = Character::new();
    let next = dance(1000, "Westfall", 12);

    let scores: Vec<i64> = [0, 1, 3, 10, 25, 50]
        .iter()
        .map(|&earlier| score(&next, &dances(earlier), &[], &character))
        .collect();

    assert_eq!(scores, [5, 4, 3, 2, 1, 0]);
}

#[test]
fn a_famous_place_adds_three_and_an_odd_hour_adds_two() {
    let character = Character::new();
    let earlier = dances(50);

    let goldshire = score(&dance(1000, "Goldshire", 12), &earlier, &[], &character);
    let late = score(&dance(1000, "Westfall", 3), &earlier, &[], &character);

    assert_eq!((goldshire, late), (3, 2));
}

#[test]
fn a_callback_to_an_npc_with_a_past_adds_four_and_a_plain_meeting_adds_nothing() {
    let mut character = Character::new();
    character.slap(Tick(1), "Innkeeper Farley").unwrap();
    character.meet_npc(Tick(2), "Marshal Dughan").unwrap();
    let kiss = |npc: &str| Flavor {
        at: Tick(1000),
        hour: None,
        place: None,
        kind: Kind::Emote {
            emote: "kiss".to_string(),
            target: Some(npc.to_string()),
        },
    };

    let slapped = score(&kiss("Innkeeper Farley"), &dances(1), &[], &character);
    let met = score(&kiss("Marshal Dughan"), &dances(1), &[], &character);

    assert_eq!((slapped, met), (5 + 4, 5));
}

#[test]
fn a_contrast_adds_one_for_each_ten_levels_up_to_four() {
    let character = Character::new();
    let humbled = |gap| Flavor {
        at: Tick(1000),
        hour: None,
        place: None,
        kind: Kind::Humbled {
            killer: "Cow".to_string(),
            gap,
        },
    };
    let earlier: Vec<Flavor> = (0..50).map(|n| humbled(10 + n)).collect();

    let scores: Vec<i64> = [10, 25, 39, 40, 59]
        .iter()
        .map(|&gap| score(&humbled(gap), &earlier, &[], &character))
        .collect();

    assert_eq!(scores, [1, 2, 3, 4, 4]);
}

#[test]
fn each_recent_telling_of_the_kind_takes_three() {
    let character = Character::new();
    let told = |at| Told {
        key: "emote:dance".to_string(),
        at: Tick(at),
    };
    let now = TOLD_SECONDS + 1000;
    let tellings = [told(now - 10), told(now - 20), told(500), told(now + 5)];

    let scored = score(&dance(now, "Westfall", 12), &[], &tellings, &character);

    assert_eq!(scored, 5 - 3 - 3);
}

#[test]
fn a_moment_in_words_names_the_emote_the_place_the_hour_and_the_count() {
    let words = timeways_story::flavor::describe(&dance(1, "Goldshire", 3), 4);

    assert_eq!(
        words,
        "The player used the emote /dance in Goldshire, at 3 o'clock, for the 4th time."
    );
}

#[test]
fn counts_read_as_english_ordinals() {
    let words: Vec<String> = [1, 2, 3, 11, 12, 13, 21, 22, 101]
        .iter()
        .map(|&count| timeways_story::flavor::describe(&dance(1, "Westfall", 12), count))
        .collect();
    let ordinals: Vec<&str> = words
        .iter()
        .map(|words| words.split("the ").last().unwrap())
        .collect();

    assert_eq!(
        ordinals,
        [
            "1st time.",
            "2nd time.",
            "3rd time.",
            "11th time.",
            "12th time.",
            "13th time.",
            "21st time.",
            "22nd time.",
            "101st time."
        ]
    );
}

#[test]
fn the_top_moments_of_a_time_are_the_best_scores_in_it() {
    let character = Character::new();
    let moments = vec![
        dance(10, "Westfall", 12),
        dance(20, "Goldshire", 3),
        dance(30, "Westfall", 12),
        dance(99, "Goldshire", 3),
    ];

    let top =
        timeways_story::flavor::top_moments(&moments, &[], &character, (Tick(15), Tick(40)), 5);

    let scores: Vec<(u64, i64, usize)> = top
        .iter()
        .map(|moment| (moment.flavor.at.0, moment.score, moment.count))
        .collect();
    assert_eq!(scores, [(20, 4 + 3 + 2, 2), (30, 4, 3)]);
}
