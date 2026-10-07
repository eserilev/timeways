//! `/twdev fps`: the story program keeps each run in the file of the dev folder, only while
//! dev mode is on, and never in a world (TESTING.md, "Testing the local model and the frame
//! rate").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Reply};
use serde_json::json;
use std::path::{Path, PathBuf};
use timeways_story::dev_fps::{FpsRun, MAX_SAMPLES, fps_file, read_runs};
use timeways_story::dev_mode::DevMode;
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-fps-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn story(folder: &Path, mode: DevMode) -> Story {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.to_path_buf()));
    story.set_dev_mode(mode);
    story
}

fn run_line(label: &str, samples: &[u32]) -> String {
    json!({
        "type": "dev_fps", "label": label, "started": 1_790_000_000, "ended": 1_790_000_002,
        "samples": samples, "hidden": 1, "min": 30, "p5": 31, "median": 58, "mean": 55,
        "memory_kb": 812, "dev": true
    })
    .to_string()
}

fn kept_runs(folder: &Path) -> Vec<FpsRun> {
    read_runs(&std::fs::read_to_string(fps_file(folder)).unwrap_or_default())
}

#[test]
fn a_run_of_twdev_fps_lands_in_the_file_of_the_dev_folder_while_dev_mode_is_on() {
    let folder = folder("on");
    let mut bridge = FakeBridge::new(story(&folder, DevMode::On));

    let reply = bridge.batch(&format!("{CHARACTER}\n{}", run_line("bench", &[60, 30])));

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    assert_eq!(bridge.dropped_lines(), 0);
    assert!(bridge.errors().is_empty(), "{:?}", bridge.errors());
    let runs = kept_runs(&folder);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].label, "bench");
    assert_eq!(runs[0].samples, [60, 30]);
    assert_eq!(runs[0].memory_kb, Some(812));
    assert_eq!(runs[0].cpu_ms, None);
}

#[test]
fn a_run_of_twdev_fps_is_refused_while_dev_mode_is_off_even_with_no_mark() {
    let folder = folder("off");
    let mut story = story(&folder, DevMode::Off);
    let _ = serve::line(&mut story, CHARACTER.as_bytes().to_vec());
    let unmarked = run_line("bench", &[60]).replace(r#","dev":true"#, "");

    let marked = serve::line(&mut story, run_line("bench", &[60]).into_bytes());
    let plain = serve::line(&mut story, unmarked.into_bytes());

    assert!(marked.error.is_some());
    assert!(
        plain
            .error
            .is_some_and(|error| error.contains("dev mode is off"))
    );
    assert!(!fps_file(&folder).exists());
}

#[test]
fn a_run_with_a_bad_label_or_too_many_samples_is_refused() {
    let folder = folder("bad");
    let mut story = story(&folder, DevMode::On);
    let _ = serve::line(&mut story, CHARACTER.as_bytes().to_vec());
    let too_many = vec![60; MAX_SAMPLES + 1];

    for line in [
        run_line("", &[60]),
        run_line("a label", &[60]),
        run_line(&"x".repeat(33), &[60]),
        run_line("bench", &too_many),
    ] {
        let served = serve::line(&mut story, line.into_bytes());
        assert!(served.error.is_some(), "{served:?}");
    }
    assert!(kept_runs(&folder).is_empty());
}

#[test]
fn a_run_of_the_most_samples_fits_one_line_of_the_bridge() {
    let folder = folder("most");
    let mut bridge = FakeBridge::new(story(&folder, DevMode::On));
    let most = vec![144; MAX_SAMPLES];

    let reply = bridge.batch(&format!("{CHARACTER}\n{}", run_line("bench", &most)));

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    assert_eq!(bridge.dropped_lines(), 0);
    assert_eq!(kept_runs(&folder).len(), 1);
}

#[test]
fn a_run_changes_no_world() {
    let folder = folder("no-world");
    let mut story = story(&folder, DevMode::On);
    let _ = serve::line(&mut story, CHARACTER.as_bytes().to_vec());
    let ask = r#"{"type":"journal_asked","id":2}"#;
    let before = serve::line(&mut story, ask.as_bytes().to_vec()).lines;

    let _ = serve::line(&mut story, run_line("bench", &[60]).into_bytes());

    let after = serve::line(&mut story, ask.as_bytes().to_vec()).lines;
    assert_eq!(before, after);
}

#[test]
fn a_run_that_stops_before_it_starts_is_refused() {
    let folder = folder("backwards");
    let mut story = story(&folder, DevMode::On);
    let _ = serve::line(&mut story, CHARACTER.as_bytes().to_vec());
    let line = run_line("bench", &[60]).replace("1790000002", "1789999999");

    let served = serve::line(&mut story, line.into_bytes());

    assert!(served.error.is_some(), "{served:?}");
}

#[test]
fn the_reader_skips_a_line_that_does_not_read() {
    let good = run_line("bench", &[60]);

    let runs = read_runs(&format!("not json\n{good}\n{{\"label\":\"x\"}}\n"));

    assert_eq!(runs.len(), 1);
}
