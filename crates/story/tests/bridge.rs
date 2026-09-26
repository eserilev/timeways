//! Drives the story program as the bridge does: a child process, with JSON lines on stdin.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use timeways_story::pack::{Link, Pack, Passage};

fn pack_file(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("bridge-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let tower = Passage {
        text: "The tower of Testvale fell.".to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![Link::Place("Testvale".to_string())],
    };
    Pack::write(&path, &[tower]).unwrap();
    path
}

fn run(pack: &Path, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_timeways-story"))
        .arg(pack)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn game_events_give_no_output_and_a_clean_exit() {
    let input = concat!(
        r#"{"type":"zone_entered","at":1,"zone":"Elwynn Forest","subzone":"Goldshire"}"#,
        "\n",
        r#"{"type":"npc_met","at":2,"name":"Innkeeper Farley"}"#,
        "\n",
        r#"{"type":"level_reached","at":3,"level":5}"#,
        "\n",
    );

    let output = run(&pack_file("events"), input);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn a_question_goes_to_the_model_and_its_answer_comes_back_with_the_sources() {
    let input = concat!(
        r#"{"type":"zone_entered","at":1,"zone":"Testvale"}"#,
        "\n",
        r#"{"type":"lore_asked","id":5,"at":2,"question":"why is this tower in ruins?"}"#,
        "\n",
        r#"{"type":"model_answered","call":1,"text":"Goblins burned it [1]."}"#,
        "\n",
    );

    let output = run(&pack_file("question"), input);

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(output.status.success());
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(
        lines[0].starts_with(r#"{"type":"model_call","call":1,"prompt":""#),
        "{stdout}"
    );
    assert_eq!(
        lines[1],
        r#"{"type":"lore_answer","id":5,"text":"Goblins burned it [1].","passages":[{"text":"The tower of Testvale fell.","source":"https://example.test/1"}]}"#
    );
}

#[test]
fn a_hello_gets_a_hello_with_the_protocol() {
    let output = run(
        &pack_file("hello"),
        "{\"type\":\"hello\",\"protocol\":1,\"app\":\"timeways\"}\n",
    );

    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "{\"type\":\"hello\",\"protocol\":1}\n"
    );
}

#[test]
fn a_bad_line_goes_to_stderr_and_the_next_line_still_counts() {
    let input = concat!(
        "not json\n",
        r#"{"type":"level_reached","at":1,"level":6}"#,
        "\n",
        r#"{"type":"level_reached","at":2,"level":5}"#,
        "\n",
    );

    let output = run(&pack_file("bad-line"), input);

    let log = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(log.contains("bad input"), "{log}");
    assert!(log.contains("refused"), "{log}");
}

#[test]
fn no_pack_is_a_failure_with_the_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_timeways-story"))
        .output()
        .unwrap();

    let log = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(log.contains("usage"), "{log}");
}

#[test]
fn a_missing_pack_is_a_failure() {
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bridge-missing.sqlite");
    let _ = std::fs::remove_file(&missing);

    let output = run(&missing, "");

    assert!(!output.status.success());
}
