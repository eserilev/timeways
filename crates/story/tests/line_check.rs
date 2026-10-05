use timeways_story::line_check::{
    COPIED_LINE_WORDS, Checked, Grounds, LineFault, checked_line, grounded,
};
use timeways_story::moments::Moment;
use timeways_story::narrator::{MAX_LINE_CHARS, Telling, Who};
use timeways_story::race_class::Race;
use timeways_story::samples::Voice;

/// The grounds of the first turn, whose naming of a deed is `$N`.
fn grounds_of(moment: &Moment, lore: Option<&str>) -> Grounds {
    told_by(moment, lore, &Who::default())
}

fn told_by(moment: &Moment, lore: Option<&str>, who: &Who) -> Grounds {
    Grounds::of(&Telling { moment, lore, who }, 0)
}

fn westfall() -> Grounds {
    let moment = Moment::NewZone {
        zone: "Westfall".to_string(),
    };
    let lore = "Stormwind never paid the men who rebuilt it.";
    grounds_of(&moment, Some(lore))
}

fn hogger() -> Grounds {
    let moment = Moment::FirstKill {
        foe: "Hogger".to_string(),
    };
    let lore = "Hogger leads the Riverpaw gnolls of Elwynn Forest. The guards of Stormwind \
        have a price on his head.";
    grounds_of(&moment, Some(lore))
}

fn level_six() -> Grounds {
    let moment = Moment::LevelUp {
        level: 6,
        zone: None,
    };
    grounds_of(&moment, None)
}

fn faults(line: &str, grounds: &Grounds) -> Vec<LineFault> {
    match checked_line(line, grounds, "") {
        Checked::Refused(faults) => faults,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_line_that_ties_the_lore_to_the_deed_passes() {
    let line = "Stormwind put a price on Hogger's head, and Elwynn waited years. $N collected it.";

    assert_eq!(
        checked_line(line, &hogger(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn the_line_that_the_player_called_slop_is_refused() {
    let line = "Level six came on a grey morning. Our hero moved on without rest.";

    let found = faults(line, &level_six());

    for word in ["grey morning", "our hero", "moved on", "without rest"] {
        assert!(
            found.contains(&LineFault::Banned(word.to_string())),
            "{found:?}"
        );
    }
    assert!(found.contains(&LineFault::Ungrounded), "{found:?}");
}

#[test]
fn our_hero_is_refused() {
    let line = "Hogger raided Elwynn for years. Our hero ended it.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::Banned("our hero".to_string())]
    );
}

#[test]
fn a_banned_word_of_the_old_list_is_refused_too() {
    let line = "Hogger raided Elwynn, guys. $N ended it.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::Banned("guys".to_string())]
    );
}

#[test]
fn invented_weather_is_refused() {
    let line = "Hogger fell at dusk, in the rain.";

    assert_eq!(
        faults(line, &hogger()),
        [
            LineFault::Banned("rain".to_string()),
            LineFault::Banned("dusk".to_string())
        ]
    );
}

#[test]
fn a_slop_word_that_the_lore_holds_is_allowed() {
    let moment = Moment::FirstInstance {
        zone: "Shadowfang Keep".to_string(),
        kind: timeways_story::places::InstanceKind::Dungeon,
    };
    let lore = "Arugal hides in the shadow of his keep, among the worgen.";
    let grounds = grounds_of(&moment, Some(lore));
    let line = "Arugal hides in the shadow of Shadowfang Keep, among his worgen.";

    assert_eq!(
        checked_line(line, &grounds, ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_line_that_names_nothing_of_the_moment_or_its_lore_is_refused() {
    let line = "The road was long, and $N kept to it.";

    assert_eq!(faults(line, &hogger()), [LineFault::Ungrounded]);
}

#[test]
fn a_name_of_the_lore_grounds_a_line() {
    let line = "Stormwind put a price on a gnoll. $N collected it.";

    assert!(grounded(line, &hogger()));
}

#[test]
fn a_plural_or_a_possessive_of_a_name_grounds_a_line() {
    let moment = Moment::SlainAgain {
        killer: "Murloc Coastrunner".to_string(),
        times: 3,
    };
    let grounds = grounds_of(&moment, None);

    assert!(grounded("The murlocs of the coast won again.", &grounds));
    assert!(grounded("The Coastrunner's spear found $N.", &grounds));
}

#[test]
fn a_short_word_of_a_name_grounds_nothing() {
    let moment = Moment::FirstInstance {
        zone: "The Deadmines".to_string(),
        kind: timeways_story::places::InstanceKind::Dungeon,
    };
    let grounds = grounds_of(&moment, None);

    assert!(!grounded("The road went on.", &grounds));
    assert!(grounded("The Deadmines took $N in.", &grounds));
}

#[test]
fn the_number_of_a_milestone_grounds_its_line() {
    let moment = Moment::LevelUp {
        level: 20,
        zone: None,
    };

    assert!(grounded(
        "Level 20, and $N still stood.",
        &grounds_of(&moment, None)
    ));
}

#[test]
fn a_line_that_copies_four_words_of_a_sample_is_refused() {
    let sample = Voice::NarratorLine.samples()[1];
    let copied: Vec<&str> = sample.split(' ').take(COPIED_LINE_WORDS).collect();
    let line = format!("{} $N came.", copied.join(" "));

    let found = faults(&line, &hogger());

    assert!(
        found
            .iter()
            .any(|fault| matches!(fault, LineFault::Copy(_))),
        "{found:?}"
    );
}

#[test]
fn three_words_of_a_sample_are_no_copy() {
    let line = "Hogger raided Elwynn, and $N stopped him.";

    assert_eq!(
        checked_line(line, &hogger(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_run_that_the_moment_itself_holds_is_no_copy() {
    let line = "Stormwind never paid the men who rebuilt it. Westfall still pays.";

    assert_eq!(
        checked_line(line, &westfall(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_place_moment_with_no_hero_passes() {
    let line = "Stormwind never paid the men who rebuilt it. Westfall pays the bill now.";

    assert_eq!(
        checked_line(line, &westfall(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_place_moment_that_names_the_hero_is_refused() {
    let line = "Stormwind never paid the men who rebuilt it, and $N saw the cost.";

    assert_eq!(faults(line, &westfall()), [LineFault::HeroAtAPlace]);
}

#[test]
fn the_stranger_who_came_into_the_fields_is_refused() {
    let line = "Westfall fed Stormwind once. One more stranger came into those fields.";

    assert_eq!(
        faults(line, &westfall()),
        [
            LineFault::Banned("one more stranger".to_string()),
            LineFault::Arrival("stranger came".to_string())
        ]
    );
}

#[test]
fn a_deed_line_that_spends_a_clause_on_an_arrival_is_refused() {
    let line = "Hogger raided Elwynn for years. $N came to his camp and ended it.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::Arrival("n came".to_string())]
    );
}

#[test]
fn the_race_of_the_hero_counts_as_the_hero() {
    let moment = Moment::FirstKill {
        foe: "Hogger".to_string(),
    };
    let orc = Who {
        race: Some(Race::Orc),
        ..Who::default()
    };
    let grounds = told_by(&moment, None, &orc);

    assert_eq!(
        faults(
            "Hogger raided Elwynn for years. The orc walked into his camp.",
            &grounds
        ),
        [LineFault::Arrival("orc walked".to_string())]
    );
}

#[test]
fn a_people_that_came_to_a_place_is_history() {
    let moment = Moment::NewZone {
        zone: "Tirisfal Glades".to_string(),
    };
    let forsaken = Who {
        race: Some(Race::Forsaken),
        ..Who::default()
    };
    let grounds = told_by(&moment, None, &forsaken);

    for line in [
        "The Scourge came to Lordaeron, and Tirisfal Glades was its first field.",
        "The orcs came to Tirisfal Glades as prisoners, long before the plague.",
        "The troll Zul'jin came to Tirisfal Glades with the Horde.",
    ] {
        assert_eq!(
            checked_line(line, &grounds, ""),
            Checked::Line(line.to_string())
        );
    }
}

#[test]
fn a_number_that_the_moment_lacks_is_refused() {
    let line = "On the 23rd day, Hogger fell to $N.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::NewNumber("23".to_string())]
    );
}

#[test]
fn a_bracket_is_refused() {
    let line = "Hogger raided Elwynn for years. [name] ended it.";

    assert_eq!(faults(line, &hogger()), [LineFault::Bracket]);
}

#[test]
fn a_line_names_the_hero_at_most_once() {
    let line = "Hogger met $N, and $N ended him.";

    assert_eq!(faults(line, &hogger()), [LineFault::NamedTwice]);
}

#[test]
fn silence_is_a_valid_answer() {
    assert_eq!(checked_line(" SILENCE. ", &hogger(), ""), Checked::Silent);
    assert_eq!(checked_line("silence", &hogger(), ""), Checked::Silent);
}

#[test]
fn a_later_name_is_refused_unless_the_player_wrote_it() {
    let line = "Hogger never saw Shattrath. $N will.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::LaterName("Shattrath".to_string())]
    );
    assert_eq!(
        checked_line(line, &hogger(), "My mother waits in Shattrath."),
        Checked::Line(line.to_string())
    );
}

#[test]
fn an_emoji_is_refused() {
    let line = "Hogger fell to $N \u{1F389}";

    assert_eq!(faults(line, &hogger()), [LineFault::Emoji]);
}

#[test]
fn an_empty_or_long_line_is_unreadable() {
    assert_eq!(faults(" \n ", &hogger()), [LineFault::Unreadable]);
    assert_eq!(
        faults(&"a".repeat(MAX_LINE_CHARS + 1), &hogger()),
        [LineFault::Unreadable]
    );
    assert_eq!(
        faults("Hogger\u{7} fell.", &hogger()),
        [LineFault::Unreadable]
    );
}

#[test]
fn a_line_is_trimmed_and_joined_into_one_line() {
    assert_eq!(
        checked_line("  Hogger fell,\n and $N  stood.  ", &hogger(), ""),
        Checked::Line("Hogger fell, and $N stood.".to_string())
    );
}

#[test]
fn each_fault_tells_the_model_what_to_fix() {
    let reasons = [
        LineFault::Banned("our hero".to_string()).to_string(),
        LineFault::Ungrounded.to_string(),
        LineFault::NamedTwice.to_string(),
        LineFault::Bracket.to_string(),
        LineFault::Arrival("n came".to_string()).to_string(),
        LineFault::HeroAtAPlace.to_string(),
    ];

    assert!(reasons[0].contains("\"our hero\""));
    assert!(reasons[1].contains("Name its place, foe, person, or people"));
    assert!(reasons[2].contains("at most once"));
    assert!(reasons[3].contains("$N"));
    assert!(reasons[4].contains("only tells that the hero came"));
    assert!(reasons[5].contains("Leave the hero out"));
}
