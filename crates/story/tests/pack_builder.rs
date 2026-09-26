//! Runs `timeways-pack` as a user runs it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use timeways_story::pack::{Link, Pack};

fn fresh(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_file(&path);
    path
}

fn build(passages: &str, name: &str) -> (Output, PathBuf) {
    let input = fresh(&format!("{name}.jsonl"));
    std::fs::write(&input, passages).unwrap();
    let pack = fresh(&format!("{name}.sqlite"));
    let output = Command::new(env!("CARGO_BIN_EXE_timeways-pack"))
        .arg(&input)
        .arg(&pack)
        .output()
        .unwrap();
    (output, pack)
}

#[test]
fn a_pack_built_from_lines_finds_its_passages_with_their_links() {
    let lines = concat!(
        r#"{"text":"The tower of Testvale fell.","source":"https://example.test/1","places":["Testvale"],"npcs":["Keeper Stubbs"]}"#,
        "\n\n",
        r#"{"text":"Mockshire has an inn.","source":"https://example.test/2","places":["Mockshire"]}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-ok");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
        format!("wrote 2 passages to {}", pack.display())
    );
    let found = Pack::open(&pack).unwrap().search("tower", 5).unwrap();
    let expected = vec![
        Link::Place("Testvale".to_string()),
        Link::Npc("Keeper Stubbs".to_string()),
    ];
    assert_eq!(found[0].links, expected);
}

#[test]
fn a_bad_line_names_its_number_and_writes_no_pack() {
    let lines = concat!(
        r#"{"text":"a","source":"b","places":["P"]}"#,
        "\n",
        r#"{"text":"a","source":"b","place":["P"]}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-bad-line");

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("line 2:")
    );
    assert!(!pack.exists());
}

#[test]
fn a_passage_with_no_links_writes_no_pack() {
    let (output, pack) = build(
        r#"{"text":"Legend says much.","source":"b"}"#,
        "builder-unlinked",
    );

    assert!(!output.status.success());
    assert!(!pack.exists());
}

#[test]
fn an_existing_pack_is_never_written_over() {
    let (first, pack) = build(
        r#"{"text":"a","source":"b","places":["P"]}"#,
        "builder-exists",
    );
    assert!(first.status.success());
    let input = fresh("builder-exists-2.jsonl");
    std::fs::write(&input, r#"{"text":"c","source":"d","places":["Q"]}"#).unwrap();

    let second = Command::new(env!("CARGO_BIN_EXE_timeways-pack"))
        .arg(&input)
        .arg(&pack)
        .output()
        .unwrap();

    assert!(!second.status.success());
    assert!(
        String::from_utf8(second.stderr)
            .unwrap()
            .contains("exists already")
    );
}
