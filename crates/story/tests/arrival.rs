//! The check of a clause that only tells that the hero came (GAMEPLAY.md 3.2.1).

use timeways_story::arrival::arrival_in;

fn kinds(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

#[test]
fn the_arrivals_that_the_player_called_cringe_are_found() {
    let found = |text: &str, hero: &[&str]| arrival_in(text, &kinds(hero));

    assert_eq!(
        found("$N came in by the Great Forge.", &[]).as_deref(),
        Some("n came")
    );
    assert_eq!(
        found("The troll climbed up to meet them.", &["troll"]).as_deref(),
        Some("troll climbed up")
    );
    assert_eq!(
        found("The orc stood at the gates of the Undercity.", &["orc"]).as_deref(),
        Some("orc stood at")
    );
    assert_eq!(
        found(
            "Darnassus sits in its crown, and the gnome walked in.",
            &["gnome"]
        )
        .as_deref(),
        Some("gnome walked")
    );
    assert_eq!(
        found("One more stranger came into those fields.", &[]).as_deref(),
        Some("stranger came")
    );
}

#[test]
fn each_verb_of_an_arrival_is_found_after_the_hero() {
    for clause in [
        "$N entered the Deadmines.",
        "$N arrived in Westfall.",
        "$N set foot in Ironforge.",
        "$N walked among the tauren.",
        "$N went down into the mine.",
        "$N made it to Booty Bay.",
        "$N found their way to the Crossroads.",
    ] {
        assert!(arrival_in(clause, &[]).is_some(), "{clause}");
    }
}

#[test]
fn small_words_between_the_hero_and_the_verb_still_count() {
    assert_eq!(
        arrival_in("Now $N has come.", &[]).as_deref(),
        Some("n has come")
    );
    assert_eq!(
        arrival_in("$N had finally arrived.", &[]).as_deref(),
        Some("n had finally arrived")
    );
}

#[test]
fn a_title_counts_as_the_hero() {
    assert_eq!(
        arrival_in("The Bookworm entered Stormwind.", &kinds(&["Bookworm"])).as_deref(),
        Some("bookworm entered")
    );
}

#[test]
fn a_people_that_came_to_a_place_is_history() {
    for text in [
        "The Scourge came to Lordaeron.",
        "The trolls came to the Echo Isles.",
        "The troll Zul'jin came to Lordaeron with the Horde.",
        "Strangers came to the Crossroads for its water.",
        "The orcs walked out of the camps.",
    ] {
        assert_eq!(arrival_in(text, &kinds(&["troll", "orc"])), None, "{text}");
    }
}

#[test]
fn a_race_that_is_not_the_hero_is_history() {
    assert_eq!(arrival_in("The Forsaken came back to Tirisfal.", &[]), None);
    assert_eq!(
        arrival_in("The tauren came to Mulgore.", &kinds(&["orc"])),
        None
    );
}

#[test]
fn a_deed_of_the_hero_is_no_arrival() {
    for text in [
        "Hogger raided Elwynn for years. $N ended it.",
        "$N reached level 20 in Duskwood.",
        "The Blackrock orcs took Stonewatch Keep. Gath'Ilzogg fell there to $N.",
        "Their pillagers have won this fight twice.",
    ] {
        assert_eq!(arrival_in(text, &kinds(&["orc"])), None, "{text}");
    }
}

#[test]
fn more_than_two_small_words_between_end_the_clause() {
    assert_eq!(arrival_in("$N has now then just come.", &[]), None);
}
