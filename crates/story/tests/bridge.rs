//! Drives the story program as the bridge does: a child process, with JSON lines on stdin.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_timeways-story"))
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
fn good_records_give_no_output_and_a_clean_exit() {
    let input = concat!(
        r#"{"at":1,"type":"zone_entered","zone":"Elwynn Forest","subzone":"Goldshire"}"#,
        "\n",
        r#"{"at":2,"type":"npc_met","name":"Innkeeper Farley"}"#,
        "\n",
        r#"{"at":3,"type":"level_reached","level":5}"#,
        "\n",
    );

    let output = run(input);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn a_bad_line_goes_to_stderr_and_the_next_line_still_counts() {
    let input = concat!(
        "not json\n",
        r#"{"at":1,"type":"level_reached","level":6}"#,
        "\n",
        r#"{"at":2,"type":"level_reached","level":5}"#,
        "\n",
    );

    let output = run(input);

    let log = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(log.contains("bad record"), "{log}");
    assert!(log.contains("refused"), "{log}");
}
