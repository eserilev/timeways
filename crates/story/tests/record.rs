use hourglass::Tick;
use timeways_story::record::{GameEvent, Record};

fn parse(line: &str) -> Result<Record, serde_json::Error> {
    serde_json::from_str(line)
}

#[test]
fn a_zone_record_reads_with_its_subzone() {
    let line = r#"{"at":100,"type":"zone_entered","zone":"Elwynn Forest","subzone":"Goldshire"}"#;

    let record = parse(line).unwrap();

    let event = GameEvent::ZoneEntered {
        zone: "Elwynn Forest".to_string(),
        subzone: Some("Goldshire".to_string()),
    };
    assert_eq!(
        record,
        Record {
            at: Tick(100),
            event
        }
    );
}

#[test]
fn a_zone_record_reads_with_no_subzone() {
    let line = r#"{"at":100,"type":"zone_entered","zone":"Elwynn Forest"}"#;

    let record = parse(line).unwrap();

    let event = GameEvent::ZoneEntered {
        zone: "Elwynn Forest".to_string(),
        subzone: None,
    };
    assert_eq!(record.event, event);
}

#[test]
fn an_npc_record_reads() {
    let line = r#"{"at":100,"type":"npc_met","name":"Innkeeper Farley"}"#;

    let record = parse(line).unwrap();

    assert_eq!(
        record.event,
        GameEvent::NpcMet {
            name: "Innkeeper Farley".to_string()
        }
    );
}

#[test]
fn a_level_record_reads() {
    let line = r#"{"at":100,"type":"level_reached","level":12}"#;

    let record = parse(line).unwrap();

    assert_eq!(record.event, GameEvent::LevelReached { level: 12 });
}

#[test]
fn an_unknown_type_is_refused() {
    let line = r#"{"at":100,"type":"dragon_slain"}"#;

    assert!(parse(line).is_err());
}

#[test]
fn a_negative_level_is_refused() {
    let line = r#"{"at":100,"type":"level_reached","level":-1}"#;

    assert!(parse(line).is_err());
}

#[test]
fn a_record_with_no_time_is_refused() {
    let line = r#"{"type":"npc_met","name":"Innkeeper Farley"}"#;

    assert!(parse(line).is_err());
}
