//! Runs `timeways-pack` as a user runs it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use timeways_story::pack::{Deed, Dependency, Link, Pack, SetupFor};

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

/// The bridge refuses a lore answer with such a passage (GAMEPLAY.md 5.10).
#[test]
fn a_line_past_a_limit_of_the_bridge_names_its_number_and_writes_no_pack() {
    let long_text = format!(
        r#"{{"text":"{}","source":"b","common":true}}"#,
        "a".repeat(4097)
    );
    let long_source = format!(
        r#"{{"text":"a","source":"{}","common":true}}"#,
        "s".repeat(513)
    );
    let control = r#"{"text":"a","source":"b\u0007c","common":true}"#;
    let cases = [
        (long_text, "line 2: the text has 4097 bytes"),
        (long_source, "line 2: the source has 513 bytes"),
        (
            control.to_string(),
            "line 2: the source holds a control character",
        ),
    ];
    for (number, (line, error)) in cases.into_iter().enumerate() {
        let lines = format!("{{\"text\":\"a\",\"source\":\"b\",\"common\":true}}\n{line}\n");

        let (output, pack) = build(&lines, &format!("builder-limit-{number}"));

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with(error), "{stderr}");
        assert!(!pack.exists());
    }
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
fn a_long_paragraph_of_a_book_becomes_passages_that_fit_the_bridge() {
    let paragraph = "The tower of Testvale fell. ".repeat(300);
    let book = format!("{{{{Book|The Testvale Tower|content=\n{paragraph}\n}}}}");
    let index = "Intro.\n===Chapter I: Mythos===\n* [[The Testvale Tower]]\n";
    let dump = wiki_dump::write_dump(
        "builder-long-paragraph",
        &[
            wiki_dump::article("History of Warcraft", index),
            wiki_dump::article("The Testvale Tower", &book),
        ],
    );
    let pack = fresh("builder-long-paragraph.sqlite");

    let output = build_from_dump(&dump, &pack);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("      3  The Testvale Tower\n"), "{stdout}");
    let found = Pack::open(&pack).unwrap().search("tower", 5).unwrap();
    assert!(found.iter().all(|passage| passage.text.len() <= 4096));
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

#[test]
fn a_line_keeps_what_its_deed_depends_on() {
    let lines = concat!(
        r#"{"text":"The adventurers killed Edwin VanCleef.","source":"s","places":["Moonbrook"],"depends_on":{"foe":"Edwin VanCleef"}}"#,
        "\n",
        r#"{"text":"An adventurer returned the linen.","source":"s","places":["Moonbrook"],"depends_on":{"quest":"Red Linen Goods"}}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-depends-on");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pack = Pack::open(&pack).unwrap();
    let vancleef = pack.search("VanCleef", 5).unwrap();
    let returned = pack.search("linen", 5).unwrap();
    assert_eq!(
        vancleef[0].depends_on,
        Some(Dependency::Foe("Edwin VanCleef".to_string()))
    );
    assert_eq!(
        returned[0].depends_on,
        Some(Dependency::Quest("Red Linen Goods".to_string()))
    );
}

#[test]
fn a_line_that_tells_a_deed_with_no_tag_is_unresolved() {
    let lines = concat!(
        r#"{"text":"The adventurers killed the ooze.","source":"s","places":["Testvale"]}"#,
        "\n",
        r#"{"text":"Adventurers often visit the inn.","source":"s","places":["Testvale"]}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-untagged-deed");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pack = Pack::open(&pack).unwrap();
    assert_eq!(
        pack.search("ooze", 5).unwrap()[0].depends_on,
        Some(Dependency::Unresolved)
    );
    assert_eq!(pack.search("inn", 5).unwrap()[0].depends_on, None);
}

#[test]
fn the_report_lists_each_outcome_passage_with_its_dependency() {
    let index = "Intro.\n===Chapter I: Mythos===\n* [[The Testvale Tower]]\n";
    let book = format!(
        "{{{{Book|The Testvale Tower|content=\n{}\n}}}}",
        wiki_dump::long("The adventurers destroyed the tower of Testvale.")
    );
    let dump = wiki_dump::write_dump(
        "builder-outcome-report",
        &[
            wiki_dump::article("History of Warcraft", index),
            wiki_dump::article("The Testvale Tower", &book),
        ],
    );
    let pack = fresh("builder-outcome-report.sqlite");

    let output = build_from_dump(&dump, &pack);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(
            "outcome  unresolved    |  the book \"The Testvale Tower\"  |  The adventurers destroyed"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("tagged 1 outcome passages: 0 by foe, 0 by quest, 1 unresolved\n"),
        "{stdout}"
    );
}

#[test]
fn a_line_keeps_what_it_sets_up() {
    let lines = concat!(
        r#"{"text":"Gryan Stoutmantle sent adventurers to kill Edwin VanCleef.","source":"s","places":["Westfall"],"setup_for":{"foe":"Edwin VanCleef","instance":"The Deadmines"}}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-setup-for");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pack = Pack::open(&pack).unwrap();
    let found = pack.search("Stoutmantle", 5).unwrap();
    assert_eq!(
        found[0].setup_for,
        Some(SetupFor {
            deed: Deed::Foe("Edwin VanCleef".to_string()),
            instance: "The Deadmines".to_string(),
        })
    );
}

#[test]
fn a_setup_line_with_both_a_foe_and_a_quest_is_refused() {
    let lines = concat!(
        r#"{"text":"The marshal sent adventurers to kill the kingpin.","source":"s","places":["Westfall"],"setup_for":{"foe":"Test Kingpin","quest":"Into the Mines","instance":"The Deadmines"}}"#,
        "\n",
    );

    let (output, pack) = build(lines, "builder-setup-both");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("line 1: setup_for needs one of foe and quest"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!pack.exists());
}

#[test]
fn the_report_lists_each_setup_passage_and_counts_them_by_instance() {
    let index = "Intro.\n===Chapter I: Mythos===\n";
    let vancleef = format!(
        "{{{{Npcbox\n| name = Edwin VanCleef\n| faction = Combat\n}}}}\n{}\n",
        wiki_dump::long("Edwin VanCleef hatched a plan of revenge on the nobles of Stormwind.")
    );
    let dump = wiki_dump::write_dump(
        "builder-setup-report",
        &[
            wiki_dump::article("History of Warcraft", index),
            wiki_dump::article("Edwin VanCleef", &vancleef),
        ],
    );
    let pack = fresh("builder-setup-report.sqlite");

    let output = build_from_dump(&dump, &pack);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(
            "setup  The Deadmines  foe  Edwin VanCleef  |  the wiki page \"Edwin VanCleef\"  \
             |  Edwin VanCleef hatched a plan"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("tagged 1 setup passages: The Deadmines 1\n"),
        "{stdout}"
    );
}
