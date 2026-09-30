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
fn a_common_line_needs_no_place_or_npc() {
    let line = r#"{"text":"The testers came from the sea.","source":"https://example.test/1","common":true}"#;

    let (output, pack) = build(line, "builder-common");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let found = Pack::open(&pack).unwrap().search("testers", 5).unwrap();
    assert_eq!(found[0].links, [Link::Common]);
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

mod wiki_dump;

fn build_from_dump(dump: &Path, pack: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_timeways-pack"))
        .arg("from-dump")
        .arg(dump)
        .arg(pack)
        .output()
        .unwrap()
}

fn history_dump(name: &str) -> PathBuf {
    let index = "Intro.\n===Chapter I: Mythos===\n* [[The Testvale Tower (History of Warcraft)]]\n";
    let book = format!(
        "{{{{Book|The Testvale Tower|content=\n{}\n}}}}",
        wiki_dump::long("The tower of Testvale fell.")
    );
    wiki_dump::write_dump(
        name,
        &[
            wiki_dump::article("History of Warcraft", index),
            wiki_dump::article("The Testvale Tower (History of Warcraft)", &book),
        ],
    )
}

#[test]
fn a_pack_built_from_a_dump_holds_the_books_and_reports_each_page() {
    let dump = history_dump("builder-dump");
    let pack = fresh("builder-dump.sqlite");

    let output = build_from_dump(&dump, &pack);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("      1  The Testvale Tower (History of Warcraft)\n"));
    assert!(stdout.contains("missing chapter: Chapter II: The New World\n"));
    assert!(stdout.contains("missing  Forsaken\n"));
    assert!(stdout.ends_with(&format!("wrote 1 passages to {}\n", pack.display())));
    let found = Pack::open(&pack).unwrap().search("tower", 5).unwrap();
    assert_eq!(found[0].source, "the book \"The Testvale Tower\"");
    assert_eq!(found[0].links, [Link::Common]);
}

#[test]
fn a_pack_from_a_dump_is_never_written_over() {
    let dump = history_dump("builder-dump-exists");
    let pack = fresh("builder-dump-exists.sqlite");
    assert!(build_from_dump(&dump, &pack).status.success());

    let second = build_from_dump(&dump, &pack);

    assert!(!second.status.success());
    assert!(
        String::from_utf8(second.stderr)
            .unwrap()
            .contains("exists already")
    );
}

#[test]
fn a_broken_dump_writes_no_pack() {
    let dump = fresh("builder-broken.xml");
    std::fs::write(
        &dump,
        "<mediawiki><page><title>History of Warcraft</title></ns>",
    )
    .unwrap();
    let pack = fresh("builder-broken.sqlite");

    let output = build_from_dump(&dump, &pack);

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("dump: broken XML")
    );
    assert!(!pack.exists());
}
