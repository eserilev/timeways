//! The narrator budget during `/twdev smoke`: a run in dev mode asks the model for every
//! narrator step, and the budget holds everywhere else (TESTING.md, "One command to test
//! everything").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::dev_mode::DevMode;
use timeways_story::dev_smoke::{SmokeDone, SmokeStep, Stopped, Verdict};
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::race_class::{Class, Race};
use timeways_story::smoke_budget::{NarratorBudget, SmokeRunState, narrator_budget};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const RUN: u64 = 1_790_000_000;

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("smoke-budget-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// The lore of the people of a human hero: a tenth level speaks only with it.
fn stormwind() -> Passage {
    Passage {
        text:
            "King Barathen Wrynn scattered the gnolls of Elwynn, and his line rules Stormwind City."
                .to_string(),
        source: "the wiki page \"Stormwind City\"".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: Some("Stormwind City".to_string()),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

/// A human paladin in a fresh world, with dev mode as `mode`.
fn paladin(name: &str, mode: DevMode) -> Story {
    let folder = folder(name);
    let pack_path = folder.join("pack.sqlite");
    Pack::write(&pack_path, &[stormwind()]).unwrap();
    let mut story = Story::new(Pack::open(&pack_path).unwrap(), Store::Folder(folder));
    story.set_dev_mode(mode);
    let entered = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: "Ada".to_string(),
    };
    story.handle(entered).unwrap();
    let described = Input::CharacterDescribed {
        at: Tick(1),
        race: Race::Human,
        class: Class::Paladin,
    };
    story.handle(described).unwrap();
    story
}

fn smoke_start(story: &mut Story) -> bool {
    let step = SmokeStep {
        run: RUN,
        step: 0,
        name: "start".to_string(),
        result: Verdict::Pass,
        reason: None,
        text: None,
        expect: Vec::new(),
        gate: None,
    };
    story.handle(Input::DevSmoke(step)).is_ok()
}

fn smoke_done(story: &mut Story) {
    let done = SmokeDone {
        run: RUN,
        steps: 1,
        passed: 1,
        failed: 0,
        waited: 0,
        seconds: 42,
        stopped: Stopped::Done,
        errors: Vec::new(),
        fps: None,
    };
    story.handle(Input::DevSmokeDone(done)).unwrap();
}

/// Four tenth levels in one hour, each in its own batch, after a first level that tells
/// nothing. Returns how many of them asked the model for a line. Each call fails, so no
/// slot stays open.
fn narrator_calls_for_four_tenth_levels(story: &mut Story) -> usize {
    let first = Input::LevelReached {
        at: Tick(10),
        level: 10,
    };
    story.handle(first).unwrap();
    let mut calls = 0;
    for (batch, level) in [(1, 20), (2, 30), (3, 40), (4, 50)] {
        let at = Tick(10 + u64::from(level));
        story.handle(Input::LevelReached { at, level }).unwrap();
        let outputs = story
            .handle(Input::BatchEnd {
                id: MessageId(batch),
            })
            .unwrap();
        for output in outputs {
            if let Output::ModelCall { call, .. } = output {
                calls += 1;
                story.handle(Input::ModelFailed { call }).unwrap();
            }
        }
    }
    calls
}

#[test]
fn in_a_smoke_run_the_budget_never_blocks_a_narrator_line() {
    let mut story = paladin("run", DevMode::On);
    assert!(smoke_start(&mut story));

    let calls = narrator_calls_for_four_tenth_levels(&mut story);

    assert_eq!(calls, 4);
}

#[test]
fn outside_a_smoke_run_the_fourth_line_in_an_hour_is_quiet() {
    let mut story = paladin("no-run", DevMode::On);

    let calls = narrator_calls_for_four_tenth_levels(&mut story);

    assert_eq!(calls, 3);
}

#[test]
fn after_a_smoke_run_ends_the_budget_holds_again() {
    let mut story = paladin("after", DevMode::On);
    assert!(smoke_start(&mut story));
    smoke_done(&mut story);

    let calls = narrator_calls_for_four_tenth_levels(&mut story);

    assert_eq!(calls, 3);
}

#[test]
fn with_dev_mode_off_a_smoke_line_never_lifts_the_budget() {
    let mut story = paladin("off", DevMode::Off);

    let started = smoke_start(&mut story);
    let calls = narrator_calls_for_four_tenth_levels(&mut story);

    assert!(!started);
    assert_eq!(calls, 3);
}

#[test]
fn the_budget_is_lifted_only_in_dev_mode_during_an_open_smoke_run() {
    for mode in [DevMode::Off, DevMode::On] {
        for run in [
            SmokeRunState::None,
            SmokeRunState::Open,
            SmokeRunState::Ended,
        ] {
            let lifted = narrator_budget(mode, run) == NarratorBudget::LiftedForSmoke;

            let expected = mode == DevMode::On && run == SmokeRunState::Open;
            assert_eq!(lifted, expected, "{mode:?} {run:?}");
        }
    }
}
