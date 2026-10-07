use timeways_story::line_check::{
    COPIED_LINE_WORDS, Checked, Grounds, LineFault, MOST_SENTENCES, built_faults, callback_in,
    checked_line, grounded,
};
use timeways_story::moments::Moment;
use timeways_story::narrator::{MAX_LINE_CHARS, Telling, Who};
use timeways_story::prose::ProseFault;
use timeways_story::race_class::{Class, Race};
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
    let lore = "Stormwind never paid the men who rebuilt it. Westfall's rich fields have lain \
        fallow since the Second War.";
    grounds_of(&moment, Some(lore))
}

fn hogger() -> Grounds {
    let moment = Moment::FirstKill {
        foe: "Hogger".to_string(),
        zone: None,
        creature: None,
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
    let line =
        "The guards of Stormwind put a price on Hogger's head years ago. $N has collected it now.";

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
    let line = "Hogger raided Elwynn for years, until our hero stopped him.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::Banned("our hero".to_string())]
    );
}

#[test]
fn a_banned_word_of_the_old_list_is_refused_too() {
    let line = "Hogger raided Elwynn for years, guys, and $N stopped him.";

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
        zone: None,
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
    let sample = Voice::NarratorLine.samples()[0];
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
    let line = "Westfall's rich fields have lain fallow since the Second War, and its farmers pay \
        the Defias now.";

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
            LineFault::Banned("came into".to_string()),
            LineFault::Arrival("stranger came".to_string())
        ]
    );
}

#[test]
fn a_deed_line_that_spends_a_clause_on_an_arrival_is_refused() {
    let line = "Hogger raided Elwynn for years. $N came to his camp and stopped him.";

    assert_eq!(
        faults(line, &hogger()),
        [LineFault::Arrival("n came".to_string())]
    );
}

#[test]
fn the_race_of_the_hero_counts_as_the_hero() {
    let moment = Moment::FirstKill {
        foe: "Hogger".to_string(),
        zone: None,
        creature: None,
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
    let line = "Hogger raided Elwynn for years. [name] stopped him there.";

    assert_eq!(faults(line, &hogger()), [LineFault::Bracket]);
}

#[test]
fn a_line_names_the_hero_at_most_once() {
    let line = "Hogger raided Elwynn for years, until $N met him and $N stopped him.";

    assert_eq!(faults(line, &hogger()), [LineFault::NamedTwice]);
}

#[test]
fn silence_is_a_valid_answer() {
    assert_eq!(checked_line(" SILENCE. ", &hogger(), ""), Checked::Silent);
    assert_eq!(checked_line("silence", &hogger(), ""), Checked::Silent);
}

#[test]
fn a_later_name_is_refused_unless_the_player_wrote_it() {
    let line = "Hogger never saw Shattrath, and $N has not seen it yet.";

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

#[test]
fn an_item_name_allows_no_slop_word_that_the_lore_lacks() {
    let moment = Moment::FirstEpicItem {
        item: "Destiny".to_string(),
        zone: Some("Westfall".to_string()),
        slot: None,
    };
    let lore = "The Defias hold Westfall.";
    let line = "The Defias hold Westfall, and the paladin now carries Destiny.";

    let refused = faults(line, &grounds_of(&moment, Some(lore)));

    assert!(
        refused.contains(&LineFault::Banned("destiny".to_string())),
        "{refused:?}"
    );
}

fn assert_banned(line: &str, moment: &Moment, lore: &str, word: &str) {
    let refused = faults(line, &grounds_of(moment, Some(lore)));

    assert!(
        refused.contains(&LineFault::Banned(word.to_string())),
        "{refused:?}"
    );
}

#[test]
fn a_quest_title_allows_no_slop_word_that_the_lore_lacks() {
    let moment = Moment::ClassQuestDone {
        title: "The Legendary Hammer".to_string(),
    };
    let lore = "The Silver Hand trains its paladins in the Cathedral of Light.";
    let line = "The Silver Hand trains its paladins, and the legendary hammer is theirs.";

    assert_banned(line, &moment, lore, "legendary");
}

#[test]
fn a_buff_name_allows_no_slop_word_that_the_lore_lacks() {
    let moment = Moment::QuestMarked {
        mark: "Ancient Ward".to_string(),
        quest: "Shrine Duty".to_string(),
    };
    let lore = "The priests of the Shrine of Mockvale ward the pilgrims on the road.";
    let line = "The priests of the Shrine of Mockvale give an ancient ward to pilgrims.";

    assert_banned(line, &moment, lore, "ancient");
}

#[test]
fn a_book_title_allows_no_slop_word_that_the_lore_lacks() {
    let moment = Moment::Flavor {
        what: "The player read \"The Eternal Tide\" in Stormwind City.".to_string(),
        book: Some("The Eternal Tide".to_string()),
    };
    let lore = "The scribes of Stormwind City copy every book of the old kingdom.";
    let line = "The scribes of Stormwind City copy the eternal tide of old books.";

    assert_banned(line, &moment, lore, "eternal");
}

#[test]
fn an_item_name_that_the_lore_holds_is_allowed() {
    let moment = Moment::FirstEpicItem {
        item: "Destiny".to_string(),
        zone: None,
        slot: None,
    };
    let lore = "The sword Destiny was forged for the guards of Stormwind.";
    let line = "Stormwind forged Destiny for its guards. $N carries it now.";

    assert_eq!(
        checked_line(line, &grounds_of(&moment, Some(lore)), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn an_item_name_never_allows_a_name_after_the_cutoff() {
    let moment = Moment::BigUpgrade {
        item: "Blade of Shattrath".to_string(),
        zone: None,
        slot: None,
    };
    let line = "Shattrath made this blade. $N wears it now.";

    let refused = faults(line, &grounds_of(&moment, None));

    assert!(
        refused.contains(&LineFault::LaterName("Shattrath".to_string())),
        "{refused:?}"
    );
}

fn level_twenty() -> Grounds {
    let moment = Moment::LevelUp {
        level: 20,
        zone: Some("Duskwood".to_string()),
    };
    let lore = "The Night Watch of Darkshire guards the last town of Duskwood against the dead.";
    grounds_of(&moment, Some(lore))
}

#[test]
fn a_ledger_of_counts_is_refused() {
    let line = "The Night Watch of Darkshire holds Duskwood. $N has reached level 20 and killed 20 \
        foes there.";

    assert_eq!(faults(line, &level_twenty()), [LineFault::Ledger]);
}

#[test]
fn one_number_and_a_count_in_words_pass() {
    for line in [
        "The Night Watch of Darkshire holds Duskwood against the dead. $N has reached level 20.",
        "The Night Watch of Darkshire held Duskwood through the Second War, twice over. $N has \
         reached level 20.",
    ] {
        assert_eq!(
            checked_line(line, &level_twenty(), ""),
            Checked::Line(line.to_string())
        );
    }
}

#[test]
fn a_line_of_more_than_three_sentences_is_refused() {
    let line = "The Night Watch guards Darkshire. The dead walk in Raven Hill. The worgen roam \
        its woods. $N has reached level 20 in Duskwood.";

    assert_eq!(
        faults(line, &level_twenty()),
        [LineFault::TooManySentences(MOST_SENTENCES + 1)]
    );
}

#[test]
fn a_line_of_three_sentences_passes() {
    let line = "The Night Watch guards Darkshire from the dead of Raven Hill. The worgen roam \
        its woods. $N has reached level 20 in Duskwood.";

    assert_eq!(
        checked_line(line, &level_twenty(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_fault_of_the_style_guide_refuses_a_line() {
    let druid = Who {
        class: Some(Class::Druid),
        ..Who::default()
    };
    let moment = Moment::LevelUp {
        level: 20,
        zone: None,
    };
    let grounds = told_by(
        &moment,
        Some("The Cenarion Circle keeps the balance of nature."),
        &druid,
    );

    let refused = faults(
        "The Cenarion Circle keeps the balance of nature, and its power grows in the druid at \
         level 20.",
        &grounds,
    );

    assert_eq!(
        refused,
        [LineFault::Prose(ProseFault::InsideHero(
            "grows in the druid".to_string()
        ))]
    );
}

#[test]
fn a_callback_to_the_players_own_story_is_refused() {
    let story = "My sister Mara raised horses near Goldshire.";
    let line = "Hogger burned the farm where the sister Mara raised horses, and $N stopped him.";

    let refused = match checked_line(line, &hogger(), story) {
        Checked::Refused(faults) => faults,
        other => panic!("{other:?}"),
    };

    assert_eq!(
        refused,
        [LineFault::Callback("sister mara raised".to_string())]
    );
}

#[test]
fn a_run_that_the_lore_holds_or_two_shared_words_are_no_callback() {
    let story = "The Riverpaw gnolls of Elwynn Forest burned my farm.\nI grew up in Elwynn Forest.";
    let lore = "Hogger leads the Riverpaw gnolls of Elwynn Forest.";

    assert_eq!(
        callback_in(
            "For years Hogger led the Riverpaw gnolls of Elwynn Forest.",
            story,
            lore
        ),
        None
    );
    assert_eq!(
        callback_in("The farms in Elwynn Forest fed Stormwind.", story, ""),
        None
    );
}

#[test]
fn a_callback_never_spans_two_lines_of_the_players_story() {
    let story = "My sister Mara\nraised horses.";

    assert_eq!(
        callback_in("The sister Mara raised horses in Elwynn.", story, ""),
        None
    );
}

#[test]
fn each_new_fault_of_a_line_tells_the_model_what_to_fix() {
    let reasons = [
        LineFault::TooManySentences(4).to_string(),
        LineFault::Ledger.to_string(),
        LineFault::Callback("sister mara raised".to_string()).to_string(),
        LineFault::Prose(ProseFault::LevelOpener).to_string(),
    ];

    assert!(reasons[0].contains("Use 1 to 3"));
    assert!(reasons[1].contains("one number at most"));
    assert!(reasons[2].contains("the player's own story"));
    assert!(reasons[3].contains("opens with the level"));
}

#[test]
fn a_run_that_only_names_what_the_lore_names_is_no_copy() {
    let moment = Moment::NewZone {
        zone: "Searing Gorge".to_string(),
    };
    let lore = "Dark Iron dwarves dominate the Searing Gorge, as Blackrock orcs hold the Burning \
        Steppes.";
    let line = "As the Blackrock orcs hold the Burning Steppes, the Dark Iron dwarves hold the \
        Searing Gorge with their golems.";

    assert_eq!(
        checked_line(line, &grounds_of(&moment, Some(lore)), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn a_run_of_a_sample_beyond_the_names_of_the_lore_is_still_a_copy() {
    let moment = Moment::FirstKill {
        foe: "Edwin VanCleef".to_string(),
        zone: None,
        creature: None,
    };
    let lore = "Edwin VanCleef founded the Defias Brotherhood.";
    let line = "Edwin VanCleef founded the Defias Brotherhood, and the Brotherhood has lost its \
        founder.";

    assert_eq!(
        faults(line, &grounds_of(&moment, Some(lore))),
        [LineFault::Copy("and the brotherhood has".to_string())]
    );
}

#[test]
fn a_line_never_copies_the_sample_of_its_own_moment_because_the_prompt_left_it_out() {
    let westfall = Moment::NewZone {
        zone: "Westfall".to_string(),
    };
    let darkshore = Moment::NewZone {
        zone: "Darkshore".to_string(),
    };
    let line = "The farmers of Westfall left, and the Defias Brotherhood holds it now.";

    assert_eq!(
        checked_line(line, &grounds_of(&westfall, None), ""),
        Checked::Line(line.to_string())
    );
    assert_eq!(
        faults(
            line,
            &grounds_of(&darkshore, Some("Westfall lies south of Elwynn."))
        ),
        [LineFault::Copy("and the defias brotherhood".to_string())]
    );
}

/// The faults of a built line about Hogger, with a name from outside that holds almost
/// nothing.
fn built_with_outside(line: &str, outside: &str) -> Vec<LineFault> {
    let grounds = Grounds {
        outside: vec![outside.to_string()],
        ..hogger()
    };
    let lore = grounds.lore.clone().unwrap_or_default();
    built_faults(line, &lore, &grounds)
}

#[test]
fn an_empty_outside_name_never_blanks_the_banned_words() {
    let found = built_with_outside("Hogger led the Riverpaw at dusk. Hogger is dead.", "");

    assert!(
        found.contains(&LineFault::Banned("dusk".to_string())),
        "{found:?}"
    );
}

#[test]
fn a_one_letter_outside_name_never_blanks_the_banned_words() {
    let found = built_with_outside("Hogger led the Riverpaw at dusk. Hogger is dead.", "u");

    assert!(
        found.contains(&LineFault::Banned("dusk".to_string())),
        "{found:?}"
    );
}
