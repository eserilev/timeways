#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::learned::Rumor;
use timeways_story::npc_memory::{
    MAX_MEMORIES, MAX_MEMORY_CHARS, Memory, Past, QuestEnding, RUMOR_CHARS, Recall, Source, line,
    memories, when,
};
use timeways_story::quest::{MAX_TITLE_CHARS, QuestChange, Step};
use timeways_story::story::MAX_NAME_BYTES;

const FARLEY: &str = "Innkeeper Farley";
const HOUR: u64 = 3_600;
const DAY: u64 = 24 * HOUR;

/// You stand in Goldshire, in Elwynn Forest, and met Farley there at tick 1.
fn in_goldshire() -> Character {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(1), FARLEY).unwrap();
    character
}

fn rumor(at: u64, npc: &str, text: &str) -> Rumor {
    Rumor {
        at: Tick(at),
        npc: npc.to_string(),
        text: text.to_string(),
    }
}

fn offered(number: u64, at: u64, giver: &str, title: &str) -> QuestChange {
    QuestChange::Offered {
        number,
        at: Tick(at),
        giver: giver.to_string(),
        title: title.to_string(),
        text: "Go.".to_string(),
        steps: vec![Step::Meet {
            npc: "Marshal Dughan".to_string(),
        }],
        any_order: None,
    }
}

fn remembered(
    character: &Character,
    rumors: &[Rumor],
    quests: &[QuestChange],
    now: u64,
) -> Vec<Memory> {
    let past = Past {
        character,
        rumors: (0..).zip(rumors).collect(),
        quests,
        own_name: "Ada",
    };
    memories(&past, FARLEY, Tick(now))
}

fn recalls(memories: &[Memory]) -> Vec<Recall> {
    memories
        .iter()
        .map(|memory| memory.recall.clone())
        .collect()
}

fn said(text: &str) -> Recall {
    Recall::Said {
        text: text.to_string(),
    }
}

fn gave(title: &str, ending: QuestEnding) -> Recall {
    Recall::GaveQuest {
        title: title.to_string(),
        ending,
    }
}

#[test]
fn the_npc_remembers_its_last_two_answers_to_you() {
    let character = in_goldshire();
    let rumors = [
        rumor(2, FARLEY, "First."),
        rumor(3, FARLEY, "Second."),
        rumor(4, FARLEY, "Third."),
    ];

    let memories = remembered(&character, &rumors, &[], 5);

    assert_eq!(recalls(&memories), [said("Third."), said("Second.")]);
}

#[test]
fn an_answer_of_another_npc_is_no_memory() {
    let character = in_goldshire();
    let rumors = [rumor(2, "Marshal Dughan", "Stay out of trouble.")];

    let memories = remembered(&character, &rumors, &[], 3);

    assert!(memories.is_empty(), "{memories:?}");
}

#[test]
fn a_rumor_that_names_your_character_is_no_memory() {
    let character = in_goldshire();
    let rumors = [
        rumor(2, FARLEY, "First."),
        rumor(3, FARLEY, "Second."),
        rumor(4, FARLEY, "Well met, ada."),
    ];

    let memories = remembered(&character, &rumors, &[], 5);

    assert_eq!(recalls(&memories), [said("Second.")]);
}

#[test]
fn a_long_rumor_is_cut_at_a_whole_word() {
    let character = in_goldshire();
    let rumors = [rumor(2, FARLEY, &"gnolls ".repeat(40))];

    let memories = remembered(&character, &rumors, &[], 3);

    let Recall::Said { text } = &memories[0].recall else {
        panic!("{memories:?}");
    };
    assert!(text.ends_with("gnolls..."), "{text}");
    assert!(text.chars().count() <= RUMOR_CHARS + 3, "{text}");
}

#[test]
fn a_quest_from_the_npc_shows_its_state() {
    let character = in_goldshire();
    let answers = [
        (None, QuestEnding::Waiting),
        (
            Some(QuestChange::Accepted {
                number: 1,
                at: Tick(3),
            }),
            QuestEnding::OnIt,
        ),
        (
            Some(QuestChange::Declined {
                number: 1,
                at: Tick(3),
            }),
            QuestEnding::TurnedDown,
        ),
        (
            Some(QuestChange::Abandoned {
                number: 1,
                at: Tick(3),
            }),
            QuestEnding::GaveUp,
        ),
    ];
    for (answer, ending) in answers {
        let mut quests = vec![offered(1, 2, FARLEY, "The Lost Lantern")];
        quests.extend(answer);

        let memories = remembered(&character, &[], &quests, 4);

        assert_eq!(recalls(&memories), [gave("The Lost Lantern", ending)]);
    }
}

#[test]
fn a_finished_quest_is_remembered_as_finished() {
    let character = in_goldshire();
    let quests = [
        offered(1, 2, FARLEY, "The Lost Lantern"),
        QuestChange::Accepted {
            number: 1,
            at: Tick(3),
        },
        QuestChange::StepDone {
            number: 1,
            step: 0,
            at: Tick(4),
        },
    ];

    let memories = remembered(&character, &[], &quests, 5);

    assert_eq!(
        recalls(&memories),
        [gave("The Lost Lantern", QuestEnding::Finished)]
    );
}

#[test]
fn a_declined_quest_is_remembered_as_turned_down() {
    let character = in_goldshire();
    let quests = [
        offered(1, 2, FARLEY, "The Lost Lantern"),
        QuestChange::Declined {
            number: 1,
            at: Tick(3),
        },
    ];

    let memories = remembered(&character, &[], &quests, 4);

    assert_eq!(
        line(&memories[0], Tick(4)),
        "Less than an hour ago: you gave the player your quest \"The Lost Lantern\". They \
         turned it down."
    );
}

#[test]
fn an_offer_that_a_newer_offer_replaced_is_no_memory() {
    let character = in_goldshire();
    let quests = [
        offered(1, 2, FARLEY, "The Lost Lantern"),
        offered(2, 3, FARLEY, "The Wine Cellar"),
    ];

    let memories = remembered(&character, &[], &quests, 4);

    assert_eq!(
        recalls(&memories),
        [gave("The Wine Cellar", QuestEnding::Waiting)]
    );
}

#[test]
fn a_quest_of_another_giver_is_no_memory() {
    let character = in_goldshire();
    let quests = [offered(1, 2, "Marshal Dughan", "Report to the Marshal")];

    let memories = remembered(&character, &[], &quests, 3);

    assert!(memories.is_empty(), "{memories:?}");
}

#[test]
fn the_first_meeting_is_a_memory_only_after_an_hour() {
    let character = in_goldshire();

    let too_soon = remembered(&character, &[], &[], 1 + HOUR - 1);
    let an_hour_on = remembered(&character, &[], &[], 1 + HOUR);

    assert!(too_soon.is_empty(), "{too_soon:?}");
    assert_eq!(recalls(&an_hour_on), [Recall::FirstMet]);
}

#[test]
fn an_npc_that_you_only_saw_has_no_first_meeting() {
    let mut character = Character::new();
    character
        .see_npc(
            Tick(1),
            FARLEY,
            timeways_story::input::Reaction::Friendly,
            None,
        )
        .unwrap();

    let memories = remembered(&character, &[], &[], 10 * DAY);

    assert!(memories.is_empty(), "{memories:?}");
}

#[test]
fn a_death_that_the_npc_caused_is_a_fight_and_not_a_death_nearby() {
    let mut character = in_goldshire();
    character.die(Tick(2), Some(FARLEY)).unwrap();

    let memories = remembered(&character, &[], &[], 3);

    assert_eq!(recalls(&memories), [Recall::KilledYou]);
}

#[test]
fn a_rare_that_you_defeated_in_its_zone_is_a_memory() {
    let mut character = in_goldshire();
    character.defeat_npc(Tick(2), "Hogger").unwrap();
    character.defeat_npc(Tick(3), "Hogger").unwrap();

    let memories = remembered(&character, &[], &[], 4);

    let near = Recall::DefeatedNear {
        foe: "Hogger".to_string(),
        place: "Goldshire".to_string(),
    };
    assert_eq!(recalls(&memories), [near]);
    assert_eq!(memories[0].at, Tick(3));
}

#[test]
fn a_rare_that_you_defeated_is_remembered_by_the_rare_as_a_fight() {
    let mut character = in_goldshire();
    character.defeat_npc(Tick(2), FARLEY).unwrap();

    let memories = remembered(&character, &[], &[], 3);

    assert_eq!(recalls(&memories), [Recall::YouDefeatedIt]);
}

#[test]
fn a_deed_in_another_zone_or_with_no_place_is_no_memory() {
    let mut character = Character::new();
    character.defeat_npc(Tick(1), "Murloc Scout").unwrap();
    character.enter_zone(Tick(2), "Westfall", None).unwrap();
    character.defeat_npc(Tick(3), "Edwin VanCleef").unwrap();
    character.die(Tick(4), None).unwrap();
    character
        .enter_zone(Tick(5), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(5), FARLEY).unwrap();

    let memories = remembered(&character, &[], &[], 6);

    assert!(memories.is_empty(), "{memories:?}");
}

#[test]
fn only_the_newest_death_nearby_is_a_memory() {
    let mut character = in_goldshire();
    character.die(Tick(2), None).unwrap();
    character.die(Tick(3), Some("Hogger")).unwrap();

    let memories = remembered(&character, &[], &[], 4);

    let died = Recall::DiedNear {
        place: "Goldshire".to_string(),
        killer: Some("Hogger".to_string()),
    };
    assert_eq!(recalls(&memories), [died]);
}

#[test]
fn the_npc_keeps_five_memories_and_drops_the_deeds_nearby_first() {
    let mut character = in_goldshire();
    character.defeat_npc(Tick(10 * DAY), "Hogger").unwrap();
    character.die(Tick(10 * DAY + 1), None).unwrap();
    let rumors = [rumor(2, FARLEY, "First."), rumor(3, FARLEY, "Second.")];
    let quests = [
        offered(1, 4, FARLEY, "The Lost Lantern"),
        QuestChange::Accepted {
            number: 1,
            at: Tick(5),
        },
        offered(2, 6, FARLEY, "The Wine Cellar"),
    ];

    let memories = remembered(&character, &rumors, &quests, 11 * DAY);

    assert_eq!(memories.len(), MAX_MEMORIES);
    assert!(
        memories.iter().all(|memory| !matches!(
            memory.recall,
            Recall::DefeatedNear { .. } | Recall::DiedNear { .. }
        )),
        "{memories:?}"
    );
    assert!(
        memories
            .iter()
            .any(|memory| memory.recall == Recall::FirstMet)
    );
}

#[test]
fn the_memories_come_newest_first() {
    let mut character = in_goldshire();
    character.defeat_npc(Tick(2 * DAY), "Hogger").unwrap();
    let rumors = [rumor(3 * DAY, FARLEY, "Hogger is gone.")];
    let quests = [offered(1, DAY, FARLEY, "The Lost Lantern")];

    let memories = remembered(&character, &rumors, &quests, 4 * DAY);

    let ats: Vec<Tick> = memories.iter().map(|memory| memory.at).collect();
    assert_eq!(ats, [Tick(3 * DAY), Tick(2 * DAY), Tick(DAY), Tick(1)]);
}

#[test]
fn an_npc_with_no_past_has_no_memories() {
    let character = in_goldshire();

    let memories = remembered(&character, &[], &[], 2);

    assert!(memories.is_empty(), "{memories:?}");
}

#[test]
fn each_memory_names_the_rows_behind_it() {
    let mut character = in_goldshire();
    character.die(Tick(2), Some(FARLEY)).unwrap();
    let rumors = [
        rumor(3, "Marshal Dughan", "Hm."),
        rumor(4, FARLEY, "Never again."),
    ];
    let quests = [
        offered(1, 5, "Marshal Dughan", "Report to the Marshal"),
        offered(2, 6, FARLEY, "The Lost Lantern"),
        QuestChange::Accepted {
            number: 2,
            at: Tick(7),
        },
    ];

    let memories = remembered(&character, &rumors, &quests, 8);

    let sources = |wanted: fn(&Recall) -> bool| -> Vec<Source> {
        let memory = memories.iter().find(|memory| wanted(&memory.recall));
        memory.unwrap().sources.clone()
    };
    assert_eq!(
        sources(|recall| matches!(recall, Recall::Said { .. })),
        [Source::Learned(1)]
    );
    assert_eq!(
        sources(|recall| matches!(recall, Recall::GaveQuest { .. })),
        [Source::Quest(1), Source::Quest(2)]
    );
    let fight = sources(|recall| *recall == Recall::KilledYou);
    assert_eq!(fight.len(), 2, "the kill and the death: {fight:?}");
}

#[test]
fn a_memory_says_when_in_words_at_each_edge() {
    let cases = [
        (3_599, "Less than an hour ago"),
        (3_600, "Hours ago"),
        (86_399, "Hours ago"),
        (86_400, "Yesterday"),
        (172_799, "Yesterday"),
        (172_800, "Two days ago"),
        (604_799, "Six days ago"),
        (604_800, "A week ago"),
        (1_209_600, "Two weeks ago"),
        (2_591_999, "Four weeks ago"),
        (2_592_000, "A month ago"),
        (5_183_999, "A month ago"),
        (5_184_000, "Two months ago"),
        (31_535_999, "Twelve months ago"),
        (31_536_000, "Over a year ago"),
    ];
    for (age, words) in cases {
        assert_eq!(when(Tick(1_000), Tick(1_000 + age)), words, "age {age}");
    }
}

#[test]
fn a_memory_from_a_later_tick_reads_as_less_than_an_hour_ago() {
    assert_eq!(when(Tick(9_000), Tick(1_000)), "Less than an hour ago");
}

#[test]
fn the_longest_line_of_each_kind_fits_its_limit() {
    let name = "W".repeat(MAX_NAME_BYTES);
    let longest = [
        Recall::Said {
            text: format!("{}...", "w".repeat(RUMOR_CHARS)),
        },
        gave(&"T".repeat(MAX_TITLE_CHARS), QuestEnding::Waiting),
        Recall::FirstMet,
        Recall::KilledYou,
        Recall::YouDefeatedIt,
        Recall::DefeatedNear {
            foe: name.clone(),
            place: name.clone(),
        },
        Recall::DiedNear {
            place: name.clone(),
            killer: Some(name.clone()),
        },
    ];
    for recall in longest {
        let memory = Memory {
            at: Tick(0),
            recall,
            sources: Vec::new(),
        };

        let line = line(&memory, Tick(1));

        assert!(line.chars().count() <= MAX_MEMORY_CHARS, "{line}");
    }
}
