use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::flavor::{Flavor, Kind};
use timeways_story::titles::earned;

fn moment(kind: Kind, place: &str) -> Flavor {
    Flavor {
        at: Tick(1),
        hour: None,
        place: Some(place.to_string()),
        kind,
    }
}

fn dance_in(place: &str) -> Flavor {
    moment(
        Kind::Emote {
            emote: "dance".to_string(),
            target: None,
        },
        place,
    )
}

fn fall() -> Flavor {
    moment(
        Kind::FellTo {
            cause: "falling".to_string(),
        },
        "Westfall",
    )
}

#[test]
fn three_dances_in_goldshire_earn_the_dance_floor() {
    let character = Character::new();

    let two = earned(&[dance_in("Goldshire"), dance_in("Goldshire")], &character);
    let three = earned(
        &[
            dance_in("Goldshire"),
            dance_in("Goldshire"),
            dance_in("Goldshire"),
        ],
        &character,
    );

    assert!(two.is_empty());
    assert_eq!(three, ["Lord of the Goldshire Dance Floor"]);
}

#[test]
fn three_falls_earn_the_friend_of_gravity() {
    assert_eq!(
        earned(&[fall(), fall(), fall()], &Character::new()),
        ["Friend of Gravity"]
    );
}

#[test]
fn one_humbling_death_earns_the_humbled() {
    let humbled = moment(
        Kind::Humbled {
            killer: "Cow".to_string(),
            gap: 59,
        },
        "Goldshire",
    );

    assert_eq!(earned(&[humbled], &Character::new()), ["The Humbled"]);
}

#[test]
fn five_slaps_of_any_npcs_earn_slap_happy() {
    let mut character = Character::new();
    for (at, npc) in [(1, "A"), (2, "B"), (3, "A"), (4, "C"), (5, "A")] {
        character.slap(Tick(at), npc).unwrap();
    }

    assert_eq!(earned(&[], &character), ["Slap Happy"]);
}
