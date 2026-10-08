//! The player's edits of a chapter, a tale, and the summary (docs/plans/chapters.md 11),
//! through the story program with a fake model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::entry_edits::{EditText, EntryKey, EntryKind};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::{Chapter, Journal, pages};
use timeways_story::pack::Pack;
use timeways_story::reply_size::{MAX_LINE, MAX_SLOT, Size};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const HOUR: u64 = 3600;

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("edits-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn started(name: &str) -> Story {
    let folder = fresh(name);
    let pack = folder.join("pack.sqlite");
    Pack::write(&pack, &[]).unwrap();
    let mut story = Story::new(Pack::open(&pack).unwrap(), Store::Folder(folder));
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(character).unwrap();
    story
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
        spot: None,
        hour: None,
        taxi: None,
    };
    story.handle(input).unwrap();
}

fn meet(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
    };
    story.handle(input).unwrap();
}

/// A meeting and 14 camps on foot in Westfall: a chapter's weight.
fn a_chapter_in_westfall(story: &mut Story) {
    enter(story, HOUR, "Westfall", None);
    meet(story, HOUR, "Gryan Stoutmantle");
    for n in 1..=14 {
        enter(story, HOUR + n * 60, "Westfall", Some(&format!("Camp {n}")));
    }
}

/// A meeting in Duskwood closes the chapter in Westfall.
fn close_it(story: &mut Story) {
    enter(story, 5 * HOUR, "Duskwood", None);
    meet(story, 5 * HOUR, "Madame Eva");
}

fn edit(story: &mut Story, entry: EntryKey, text: EditText, paragraphs: &[&str]) {
    let input = Input::EntryEdited {
        at: Tick(2 * HOUR),
        entry,
        title: None,
        text,
        paragraphs: paragraphs.iter().map(ToString::to_string).collect(),
    };
    story.handle(input).unwrap();
}

fn journal(story: &mut Story) -> Journal {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(70),
            page: 0,
        })
        .unwrap();
    match outputs.into_iter().next() {
        Some(Output::Journal { page, .. }) => page.journal,
        other => panic!("expected a journal, got {other:?}"),
    }
}

fn first_chapter(story: &mut Story) -> (Chapter, EntryKey) {
    let chapter = journal(story).chapters.remove(0);
    let key = EntryKey {
        kind: EntryKind::Chapter,
        first: Some(chapter.first),
    };
    (chapter, key)
}

fn calls(outputs: Vec<Output>) -> Vec<(CallId, String)> {
    outputs
        .into_iter()
        .filter_map(|output| match output {
            Output::ModelCall { call, prompt } => Some((call, prompt)),
            _ => None,
        })
        .collect()
}

/// Batches end, half an hour of play apart. A saga call gets `saga`, and every other call
/// an answer that the checks refuse. Returns the prompts of the sagas.
fn settle(story: &mut Story, saga: &str) -> Vec<String> {
    let mut sagas = Vec::new();
    for batch in 0..6 {
        let later = Input::HourChanged {
            at: Tick(20 * HOUR + batch * 1800),
            hour: 12,
        };
        story.handle(later).unwrap();
        let mut open = calls(
            story
                .handle(Input::BatchEnd {
                    id: MessageId(batch),
                })
                .unwrap(),
        );
        while let Some((call, prompt)) = open.pop() {
            let is_saga = prompt.contains(r#"{"saga":"#);
            let text = if is_saga {
                sagas.push(prompt);
                serde_json::json!({ "saga": saga }).to_string()
            } else {
                String::new()
            };
            open.extend(calls(
                story.handle(Input::ModelAnswered { call, text }).unwrap(),
            ));
        }
    }
    sagas
}

const SAGA: &str = "Westfall burned while Stormwind looked away, and the militia held the hill.";

#[test]
fn an_edit_keeps_the_chapter_list() {
    let mut story = started("keeps-list");
    a_chapter_in_westfall(&mut story);
    let (before, key) = first_chapter(&mut story);

    edit(
        &mut story,
        key,
        EditText::Replace,
        &["We dug in at the hill."],
    );
    let (after, _) = first_chapter(&mut story);

    assert_eq!(after, before);
    let journal = journal(&mut story);
    assert_eq!(journal.edits.len(), 1);
    assert_eq!(journal.edits[0].paragraphs, ["We dug in at the hill."]);
}

#[test]
fn a_saga_after_a_note_on_the_open_chapter_shows_above_it() {
    let mut story = started("note-then-saga");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(&mut story, key, EditText::Keep, &["We sang at the camp."]);

    close_it(&mut story);
    let sagas = settle(&mut story, SAGA);

    assert!(
        sagas[0].contains("The player's telling, not canon."),
        "{}",
        sagas[0]
    );
    assert!(sagas[0].contains("We sang at the camp."), "{}", sagas[0]);
    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose.as_deref(), Some(SAGA));
    assert_eq!(journal(&mut story).edits[0].text, EditText::Keep);
}

#[test]
fn a_saga_after_a_replace_does_not_show() {
    let mut story = started("replace-then-saga");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(&mut story, key, EditText::Replace, &["Our own words."]);

    close_it(&mut story);
    settle(&mut story, SAGA);

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose, None);
}

#[test]
fn restore_shows_the_newest_narrator_text() {
    let mut story = started("restore");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(&mut story, key, EditText::Replace, &["Our own words."]);
    close_it(&mut story);
    settle(&mut story, SAGA);

    edit(&mut story, key, EditText::Narrator, &[]);

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose.as_deref(), Some(SAGA));
    assert!(journal(&mut story).edits.is_empty());
}

#[test]
fn a_model_text_that_copies_an_edit_is_refused() {
    let mut story = started("copies-edit");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    let words = "the militia held the hill against the defias for three long nights";
    edit(&mut story, key, EditText::Keep, &[words]);

    close_it(&mut story);
    let saga = format!("In Westfall {words}.");
    settle(&mut story, &saga);
    let mut unedited = started("copies-edit-control");
    a_chapter_in_westfall(&mut unedited);
    close_it(&mut unedited);
    settle(&mut unedited, &saga);

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose, None);
    let (control, _) = first_chapter(&mut unedited);
    assert_eq!(control.prose, Some(saga));
}

#[test]
fn a_later_name_in_an_edit_is_not_allowed_in_a_saga() {
    let mut story = started("later-name");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(
        &mut story,
        key,
        EditText::Keep,
        &["One day we will see Shattrath."],
    );

    close_it(&mut story);
    settle(
        &mut story,
        "Westfall dreamed of Shattrath while the militia held the hill.",
    );

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose, None);
}

#[test]
fn the_same_edit_again_writes_no_row_and_a_bad_edit_leaves_its_reason() {
    let mut story = started("no-op");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(&mut story, key, EditText::Keep, &["We sang."]);
    edit(&mut story, key, EditText::Keep, &["We sang."]);

    edit(&mut story, key, EditText::Keep, &["a | b"]);
    let journal = journal(&mut story);

    assert_eq!(journal.edits.len(), 1);
    assert!(journal.edit_refused.is_some());
}

#[test]
fn a_player_name_in_an_edit_is_kept_as_an_id_and_shown_as_the_name() {
    let mut story = started("names");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);

    edit(
        &mut story,
        key,
        EditText::Keep,
        &["{Ada} held the hill with me."],
    );
    close_it(&mut story);
    let sagas = settle(&mut story, SAGA);

    assert!(!sagas[0].contains("Ada"), "{}", sagas[0]);
    assert!(
        sagas[0].contains("{P1} held the hill with me."),
        "{}",
        sagas[0]
    );
    assert_eq!(
        journal(&mut story).edits[0].paragraphs,
        ["Ada held the hill with me."]
    );
}

#[test]
fn the_largest_chapter_and_its_largest_edit_fit_the_journal() {
    let mut journal = Journal::default();
    let mut chapter = Chapter {
        prose: Some("\"".repeat(600)),
        footnotes: vec!["\"".repeat(200); 3],
        zones: vec!["\"".repeat(48); 20],
        people: vec!["\"".repeat(48); 20],
        again: vec!["\"".repeat(100); 20],
        ..Chapter::default()
    };
    chapter.title = Some("\"".repeat(48));
    journal.chapters = vec![chapter];
    journal.edits = vec![timeways_story::entry_edits::EditView {
        entry: EntryKey {
            kind: EntryKind::Chapter,
            first: Some(u64::MAX),
        },
        title: Some("\"".repeat(60)),
        text: EditText::Replace,
        paragraphs: vec!["\"".repeat(50); 20],
    }];

    for page in pages(journal) {
        let line = serde_json::to_string(&Output::Journal {
            id: MessageId(u64::MAX),
            page: Box::new(page),
            notice: None,
            dev: Some(timeways_story::dev_mode::DevOn),
            past: Some(timeways_story::story::PastWanted::Wanted),
        })
        .unwrap();
        assert!(line.len() <= MAX_LINE, "{} bytes", line.len());
    }
}

#[test]
fn a_marked_name_before_a_curly_apostrophe_never_reaches_a_saga() {
    let mut story = started("curly-apostrophe");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);

    edit(
        &mut story,
        key,
        EditText::Keep,
        &["{Ada}’s shield held the hill."],
    );
    close_it(&mut story);
    let sagas = settle(&mut story, SAGA);

    assert!(!sagas[0].contains("Ada"), "{}", sagas[0]);
    assert!(sagas[0].contains("{P1}’s shield"), "{}", sagas[0]);
}

#[test]
fn a_saga_that_names_a_player_by_id_shows_the_name() {
    let mut story = started("id-in-saga");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(
        &mut story,
        key,
        EditText::Keep,
        &["{Ada} held the hill with me."],
    );

    close_it(&mut story);
    settle(
        &mut story,
        "Westfall burned while Stormwind looked away, and {P1} held the hill.",
    );

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(
        chapter.prose.as_deref(),
        Some("Westfall burned while Stormwind looked away, and Ada held the hill.")
    );
}

/// A name of 12 letters in place of each of 40 IDs: 900 letters, past the 600 of a saga.
#[test]
fn a_saga_whose_names_make_it_too_long_is_refused() {
    let mut story = started("names-too-long");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(
        &mut story,
        key,
        EditText::Keep,
        &["{Bartholomewa} held the hill."],
    );

    close_it(&mut story);
    let saga = "{P1} and {P1} held the hill. ".repeat(20);
    settle(&mut story, saga.trim_end());

    let (chapter, _) = first_chapter(&mut story);
    assert_eq!(chapter.prose, None);
}

/// The longest saga with names, a long edit, and a waiting notice still fit one reply.
#[test]
fn a_journal_page_with_many_names_in_place_of_ids_still_fits() {
    let mut story = started("names-fit");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    let long_name = "Ä".repeat(24);
    let paragraph = format!("{{{long_name}}} held the hill.");
    edit(&mut story, key, EditText::Keep, &[&paragraph]);

    close_it(&mut story);
    let saga = "{P1} and {P1} held the hill. ".repeat(8);
    settle(&mut story, saga.trim_end());
    story.set_program_notice("\"".repeat(1000));

    let (chapter, _) = first_chapter(&mut story);
    assert!(chapter.prose.unwrap().contains(&long_name));
    let output = story
        .handle(Input::JournalAsked {
            id: MessageId(u64::MAX),
            page: 0,
        })
        .unwrap();
    let line = serde_json::to_string(&output[0]).unwrap();
    assert!(line.len() <= MAX_LINE, "{} bytes", line.len());
    assert!(Size::of_json(line.as_bytes()).slot <= MAX_SLOT);
}

#[test]
fn a_name_learned_after_an_edit_never_reaches_a_saga() {
    let mut story = started("name-learned-later");
    a_chapter_in_westfall(&mut story);
    let (_, key) = first_chapter(&mut story);
    edit(
        &mut story,
        key,
        EditText::Keep,
        &["Ada held the hill with me."],
    );

    let in_sight = Input::PlayerDescribed {
        at: Tick(3 * HOUR),
        name: "Ada".to_string(),
        race: None,
        class: None,
    };
    story.handle(in_sight).unwrap();
    close_it(&mut story);
    let sagas = settle(&mut story, SAGA);

    assert!(!sagas[0].contains("Ada"), "{}", sagas[0]);
    assert!(sagas[0].contains("{P1} held the hill"), "{}", sagas[0]);
}

#[test]
fn an_edit_of_an_entry_that_does_not_exist_stays_out_of_the_journal() {
    let mut story = started("no-such-entry");
    a_chapter_in_westfall(&mut story);
    let (chapter, _) = first_chapter(&mut story);
    let no_chapter = EntryKey {
        kind: EntryKind::Chapter,
        first: Some(chapter.first + 1000),
    };
    let no_tale = EntryKey {
        kind: EntryKind::Tale,
        first: Some(chapter.first),
    };

    edit(&mut story, no_chapter, EditText::Keep, &["We sang."]);
    edit(&mut story, no_tale, EditText::Keep, &["We sang."]);

    assert!(journal(&mut story).edits.is_empty());
}
