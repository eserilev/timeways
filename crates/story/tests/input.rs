use hourglass::Tick;
use timeways_story::input::{CallId, GameQuestKind, Input, MessageId};
use timeways_story::places::InstanceKind;

fn parse(line: &str) -> Result<Input, serde_json::Error> {
    serde_json::from_str(line)
}

#[test]
fn a_zone_input_reads_with_its_subzone() {
    let line = r#"{"type":"zone_entered","at":100,"zone":"Elwynn Forest","subzone":"Goldshire"}"#;

    let input = parse(line).unwrap();

    let expected = Input::ZoneEntered {
        at: Tick(100),
        zone: "Elwynn Forest".to_string(),
        subzone: Some("Goldshire".to_string()),
    };
    assert_eq!(input, expected);
}

#[test]
fn a_zone_input_reads_with_no_subzone() {
    let line = r#"{"type":"zone_entered","at":100,"zone":"Elwynn Forest"}"#;

    let input = parse(line).unwrap();

    let expected = Input::ZoneEntered {
        at: Tick(100),
        zone: "Elwynn Forest".to_string(),
        subzone: None,
    };
    assert_eq!(input, expected);
}

#[test]
fn an_npc_input_reads() {
    let line = r#"{"type":"npc_met","at":100,"name":"Innkeeper Farley"}"#;

    let input = parse(line).unwrap();

    assert_eq!(
        input,
        Input::NpcMet {
            at: Tick(100),
            name: "Innkeeper Farley".to_string()
        }
    );
}

#[test]
fn a_level_input_reads() {
    let line = r#"{"type":"level_reached","at":100,"level":12}"#;

    let input = parse(line).unwrap();

    assert_eq!(
        input,
        Input::LevelReached {
            at: Tick(100),
            level: 12
        }
    );
}

#[test]
fn a_lore_input_reads_with_its_target() {
    let line = r#"{"type":"lore_asked","id":3,"question":"who is this?","target":"Hogger"}"#;

    let input = parse(line).unwrap();

    let expected = Input::LoreAsked {
        id: MessageId(3),
        question: "who is this?".to_string(),
        target: Some("Hogger".to_string()),
    };
    assert_eq!(input, expected);
}

#[test]
fn a_model_answer_reads() {
    let line = r#"{"type":"model_answered","call":7,"text":"Nobody knows."}"#;

    let input = parse(line).unwrap();

    assert_eq!(
        input,
        Input::ModelAnswered {
            call: CallId(7),
            text: "Nobody knows.".to_string()
        }
    );
}

#[test]
fn a_model_failure_reads() {
    let line = r#"{"type":"model_failed","call":7}"#;

    let input = parse(line).unwrap();

    assert_eq!(input, Input::ModelFailed { call: CallId(7) });
}

#[test]
fn an_unknown_type_is_refused() {
    assert!(parse(r#"{"type":"dragon_slain","at":100}"#).is_err());
}

#[test]
fn a_negative_level_is_refused() {
    assert!(parse(r#"{"type":"level_reached","at":100,"level":-1}"#).is_err());
}

#[test]
fn a_game_event_with_no_time_is_refused() {
    assert!(parse(r#"{"type":"npc_met","name":"Innkeeper Farley"}"#).is_err());
}

#[test]
fn a_hello_reads_with_the_fields_of_the_bridge() {
    let line = r#"{"type":"hello","protocol":1,"app":"timeways"}"#;

    assert_eq!(parse(line).unwrap(), Input::Hello);
}

#[test]
fn a_game_event_reads_with_the_id_that_the_bridge_adds() {
    let line = r#"{"type":"level_reached","id":4,"at":100,"level":12}"#;

    assert_eq!(
        parse(line).unwrap(),
        Input::LevelReached {
            at: Tick(100),
            level: 12
        }
    );
}

#[test]
fn a_question_with_no_id_is_refused() {
    assert!(parse(r#"{"type":"lore_asked","question":"who?"}"#).is_err());
}

#[test]
fn a_journal_request_reads() {
    let line = r#"{"type":"journal_asked","id":9}"#;

    assert_eq!(
        parse(line).unwrap(),
        Input::JournalAsked {
            id: MessageId(9),
            page: 0
        }
    );
}

#[test]
fn a_journal_request_reads_with_its_page() {
    let line = r#"{"type":"journal_asked","id":9,"page":3}"#;

    assert_eq!(
        parse(line).unwrap(),
        Input::JournalAsked {
            id: MessageId(9),
            page: 3
        }
    );
}

#[test]
fn a_kill_input_reads() {
    let line = r#"{"type":"npc_defeated","at":100,"name":"Hogger"}"#;

    assert_eq!(
        parse(line).unwrap(),
        Input::NpcDefeated {
            at: Tick(100),
            name: "Hogger".to_string()
        }
    );
}

#[test]
fn a_death_reads_with_and_without_a_killer() {
    let with = r#"{"type":"died","at":100,"killer":"Hogger"}"#;
    let without = r#"{"type":"died","at":100}"#;

    assert_eq!(
        parse(with).unwrap(),
        Input::Died {
            at: Tick(100),
            killer: Some("Hogger".to_string()),
            cause: None,
            killer_level: None,
            hour: None
        }
    );
    assert_eq!(
        parse(without).unwrap(),
        Input::Died {
            at: Tick(100),
            killer: None,
            cause: None,
            killer_level: None,
            hour: None
        }
    );
}

#[test]
fn a_slap_reads() {
    let line = r#"{"type":"npc_slapped","at":100,"name":"Innkeeper Farley"}"#;

    assert_eq!(
        parse(line).unwrap(),
        Input::NpcSlapped {
            at: Tick(100),
            name: "Innkeeper Farley".to_string()
        }
    );
}

#[test]
fn a_talk_reads() {
    let line =
        r#"{"type":"talk_asked","id":2,"at":100,"npc":"Innkeeper Farley","text":"any news?"}"#;

    let expected = Input::TalkAsked {
        id: MessageId(2),
        at: Tick(100),
        npc: "Innkeeper Farley".to_string(),
        text: "any news?".to_string(),
    };
    assert_eq!(parse(line).unwrap(), expected);
}

#[test]
fn the_edits_of_the_hero_read() {
    let set = r#"{"type":"hero_set","at":1,"field":"goal","text":"Find my brother."}"#;
    let added = r#"{"type":"hero_added","at":2,"text":"An oath.","npc":"Innkeeper Farley"}"#;
    let removed = r#"{"type":"hero_removed","at":3,"number":4}"#;

    assert_eq!(
        parse(set).unwrap(),
        Input::HeroSet {
            at: Tick(1),
            field: "goal".to_string(),
            text: "Find my brother.".to_string()
        }
    );
    assert_eq!(
        parse(added).unwrap(),
        Input::HeroAdded {
            at: Tick(2),
            text: "An oath.".to_string(),
            npc: Some("Innkeeper Farley".to_string())
        }
    );
    assert_eq!(
        parse(removed).unwrap(),
        Input::HeroRemoved {
            at: Tick(3),
            number: 4
        }
    );
}

#[test]
fn a_quest_of_the_game_reads_with_its_kind() {
    let taken = parse(
        r#"{"type":"game_quest_accepted","at":5,"title":"Rediscovering the Light","kind":"class"}"#,
    )
    .unwrap();
    let done = parse(
        r#"{"type":"game_quest_done","at":6,"title":"Rattling the Rattlecages","kind":"normal"}"#,
    )
    .unwrap();

    assert_eq!(
        taken,
        Input::GameQuestAccepted {
            at: Tick(5),
            title: "Rediscovering the Light".to_string(),
            kind: GameQuestKind::Class,
        }
    );
    assert_eq!(
        done,
        Input::GameQuestDone {
            at: Tick(6),
            title: "Rattling the Rattlecages".to_string(),
            kind: GameQuestKind::Normal,
        }
    );
}

#[test]
fn a_quest_of_the_game_with_an_unknown_kind_is_refused() {
    let line = r#"{"type":"game_quest_done","at":6,"title":"X","kind":"epic"}"#;

    assert!(parse(line).is_err());
}

#[test]
fn an_instance_reads_as_a_dungeon_or_a_raid() {
    let dungeon =
        parse(r#"{"type":"instance_entered","at":5,"zone":"The Deadmines","kind":"party"}"#)
            .unwrap();
    let raid =
        parse(r#"{"type":"instance_entered","at":5,"zone":"Molten Core","kind":"raid"}"#).unwrap();

    assert_eq!(
        dungeon,
        Input::InstanceEntered {
            at: Tick(5),
            zone: "The Deadmines".to_string(),
            kind: InstanceKind::Dungeon,
        }
    );
    assert!(matches!(
        raid,
        Input::InstanceEntered {
            kind: InstanceKind::Raid,
            ..
        }
    ));
}

#[test]
fn a_battleground_is_no_instance_of_the_story() {
    let line = r#"{"type":"instance_entered","at":5,"zone":"Warsong Gulch","kind":"pvp"}"#;

    assert!(parse(line).is_err());
}
