//! The parts of the benches: the numbers, the names of the faults, the verdict on each call,
//! the model runners, the counters of a process, and the phases of a frame rate run.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::time::Duration;
use timeways_dev::asks::{Outcome, asks_of, quiet_moments};
use timeways_dev::bench::{Call, Played, Row};
use timeways_dev::bench_fps::{Phase, covering_run, phase_reports, samples_in};
use timeways_dev::faults::{
    faults_of_answer, is_retry, is_silence, name_of_reason, reasons_of_retry,
};
use timeways_dev::gate::needs_dev_mode;
use timeways_dev::model_runner::{Asked, Runner, read_completion};
use timeways_dev::proc_stats::{DrmClient, cpu_ticks, drm_client, nvidia_memory_of, rss_kb};
use timeways_dev::scenario::{Scenario, ScenarioError};
use timeways_dev::stats::{drop_percent, percentile, spread};
use timeways_story::dev_fps::FpsRun;
use timeways_story::line_check::LineFault;
use timeways_story::prompt;
use timeways_story::prose::ProseFault;

// The numbers ------------------------------------------------------------------------------

#[test]
fn the_percentiles_of_no_sample_one_sample_and_two_samples() {
    assert_eq!(percentile(&[], 0.5), None);
    assert_eq!(spread(&[]), None);
    assert_eq!(percentile(&[42.0], 0.05), Some(42.0));
    assert_eq!(percentile(&[42.0], 0.95), Some(42.0));
    assert_eq!(percentile(&[40.0, 60.0], 0.5), Some(50.0));
    assert_eq!(percentile(&[40.0, 60.0], 0.05), Some(41.0));
    assert_eq!(percentile(&[40.0, 60.0], 0.0), Some(40.0));
    assert_eq!(percentile(&[40.0, 60.0], 1.0), Some(60.0));
}

#[test]
fn a_spread_sorts_its_values_and_leaves_out_values_that_are_no_number() {
    let spread = spread(&[60.0, f64::NAN, 30.0, f64::INFINITY, 45.0]).unwrap();

    assert_eq!(spread.count, 3);
    assert_eq!((spread.min, spread.median, spread.max), (30.0, 45.0, 60.0));
    assert!((spread.mean - 45.0).abs() < 1e-9);
}

#[test]
fn a_drop_from_a_baseline_of_zero_is_no_number() {
    assert_eq!(drop_percent(0.0, 30.0), None);
    assert_eq!(drop_percent(60.0, 45.0), Some(25.0));
    assert_eq!(drop_percent(60.0, 66.0), Some(-10.0));
}

// The names of the faults ------------------------------------------------------------------

#[test]
fn every_reason_of_the_narrator_check_has_a_short_name() {
    let faults = [
        LineFault::Unreadable,
        LineFault::LaterName("Arthas".into()),
        LineFault::Emoji,
        LineFault::Banned("tapestry".into()),
        LineFault::Copy("the fields of".into()),
        LineFault::NewNumber("12".into()),
        LineFault::Ungrounded,
        LineFault::Bracket,
        LineFault::NamedTwice,
        LineFault::Arrival("came to Westfall".into()),
        LineFault::HeroAtAPlace,
        LineFault::TooManySentences(5),
        LineFault::Ledger,
        LineFault::Callback("my uncle".into()),
        LineFault::BadAnswer,
        LineFault::UnknownChoice("verb".into()),
        LineFault::ChoiceNotInLore("foe".into()),
        LineFault::HeroInHistory,
        LineFault::OverBudget(200),
        LineFault::HistorySentences(3, 1),
        LineFault::HistorySentences(3, 2),
        LineFault::Prose(ProseFault::InsideHero("grows in $N".into())),
        LineFault::Prose(ProseFault::Recognition("knows $N".into())),
        LineFault::Prose(ProseFault::SourceShape("He spoke.".into())),
        LineFault::Prose(ProseFault::Fragment("Level 10.".into())),
        LineFault::Prose(ProseFault::LongSentence("The".into())),
        LineFault::Prose(ProseFault::Pivot("not just".into())),
        LineFault::Prose(ProseFault::LevelOpener),
    ];

    for fault in faults {
        let reason = fault.to_string();
        assert_ne!(name_of_reason(&reason), "other", "{reason}");
    }
}

#[test]
fn the_names_of_the_faults_that_the_bench_counts_most() {
    let named = |fault: LineFault| name_of_reason(&fault.to_string());

    assert_eq!(named(LineFault::Banned("tapestry".into())), "slop");
    assert_eq!(named(LineFault::Copy("x".into())), "copy");
    assert_eq!(named(LineFault::Arrival("x".into())), "arrival");
    assert_eq!(named(LineFault::BadAnswer), "json-shape");
    assert_eq!(
        named(LineFault::Prose(ProseFault::InsideHero("x".into()))),
        "inside-hero"
    );
}

#[test]
fn a_retry_prompt_gives_back_the_reasons_of_the_answer_before_it() {
    let reasons = vec![
        "\"tapestry\" is empty or invented. Leave it out.".to_string(),
        "The line has a bracket.".to_string(),
    ];
    let retry = prompt::retry("Tell the moment.", "A tapestry [1].", &reasons);

    assert!(is_retry(&retry));
    assert!(!is_retry("Tell the moment."));
    assert_eq!(reasons_of_retry(&retry), reasons);
}

#[test]
fn the_text_of_an_answer_is_its_prose_field_and_never_a_choice() {
    use timeways_dev::faults::main_text;

    assert_eq!(
        main_text(r#"{"lore": "Westfall burns.", "group": "g.class"}"#),
        "Westfall burns."
    );
    assert_eq!(
        main_text(r#"{"saga": "The Defias rose.", "footnotes": []}"#),
        "The Defias rose."
    );
    assert_eq!(main_text(r#"{"pick": 1}"#), r#"{"pick": 1}"#);
    assert_eq!(main_text("```\nWestfall burns.\n```"), "Westfall burns.");
}

#[test]
fn a_choice_field_never_makes_a_whole_line_look_cut_off() {
    let answer = r#"{"lore": "Kobolds hold the Jasperlode Mine and the Fargodeep Mine now.", "group": "g.class"}"#;

    assert!(!faults_of_answer(answer, true).contains(&"cutoff"));
}

#[test]
fn silence_is_the_word_alone_or_the_lore_of_a_json_answer() {
    assert!(is_silence("SILENCE"));
    assert!(is_silence("  silence. "));
    assert!(is_silence("```\nSILENCE\n```"));
    assert!(is_silence(r#"{"lore": "SILENCE"}"#));
    assert!(!is_silence("Silence fell over Westfall."));
    assert!(!is_silence(r#"{"lore": "Westfall burns."}"#));
}

#[test]
fn an_answer_alone_shows_a_cut_off_end_a_bad_shape_and_slop() {
    assert_eq!(
        faults_of_answer(r#"{"lore": "Westfall once"#, true),
        ["cutoff"]
    );
    assert_eq!(
        faults_of_answer(r#"{"lore": "Murlocs live in and"#, false),
        ["cutoff"]
    );
    assert_eq!(faults_of_answer("Westfall burns.", true), ["json-shape"]);
    assert_eq!(faults_of_answer("", false), ["unreadable"]);
    assert!(
        faults_of_answer("The Defias hold Westfall now, and the farms", false).contains(&"cutoff")
    );
    assert!(
        faults_of_answer(
            "Our hero walked the fields of Westfall at dawn today.",
            false
        )
        .contains(&"slop")
    );
    assert_eq!(
        faults_of_answer(
            "The Defias Brotherhood holds the fields of Westfall now.",
            false
        ),
        ["other"]
    );
}

// The verdict on each call -----------------------------------------------------------------

fn call(batch: usize, moment: &str, kind: &str, prompt: &str, answer: Option<&str>) -> Call {
    Call {
        batch,
        moment: moment.to_string(),
        kind: kind.to_string(),
        prompt: prompt.to_string(),
        asked: Asked {
            answer: answer.map(str::to_string),
            latency: Duration::from_millis(1500),
            first_byte: None,
            tokens: Some(30),
        },
    }
}

fn row(kind: &str, prompt: &str, accepted: bool) -> Row {
    Row {
        kind: kind.to_string(),
        prompt: Some(prompt.to_string()),
        accepted,
    }
}

#[test]
fn a_refused_line_and_its_accepted_retry_make_one_shown_ask_with_a_retry() {
    let first = "Tell the arrival.";
    let reasons = vec!["\"tapestry\" is empty or invented. Leave it out.".to_string()];
    let retry = prompt::retry(first, "A tapestry.", &reasons);
    let played = Played {
        calls: vec![
            call(1, "arrival", "narrator", first, Some("A tapestry.")),
            call(1, "arrival", "narrator", &retry, Some("Westfall burns.")),
        ],
        moments: BTreeMap::from([(1, "arrival".to_string())]),
        replies: BTreeMap::from([(1, "Westfall burns, and the Defias hold it.".to_string())]),
        rows: vec![row("narrator", first, false), row("narrator", &retry, true)],
        failed_batches: Vec::new(),
    };

    let asks = asks_of(&played);

    assert_eq!(asks.len(), 1);
    let ask = &asks[0];
    assert_eq!(ask.outcome, Outcome::Shown);
    assert_eq!(ask.attempts.len(), 2);
    assert_eq!(ask.attempts[0].faults, ["slop"]);
    assert_eq!(
        ask.shown.as_deref(),
        Some("Westfall burns, and the Defias hold it.")
    );
}

#[test]
fn an_ask_ends_silent_refused_or_failed_by_its_last_call() {
    let played = Played {
        calls: vec![
            call(1, "deed", "narrator", "p1", Some("SILENCE")),
            call(2, "boss", "narrator", "p2", Some("Westfall, at dawn,")),
            call(3, "saga", "saga", "p3", None),
        ],
        moments: BTreeMap::from([
            (1, "deed".to_string()),
            (2, "boss".to_string()),
            (3, "saga".to_string()),
            (4, "tale".to_string()),
        ]),
        replies: BTreeMap::new(),
        rows: vec![
            row("narrator", "p1", false),
            row("narrator", "p2", false),
            row("saga", "p3", false),
        ],
        failed_batches: Vec::new(),
    };

    let asks = asks_of(&played);

    let outcomes: Vec<Outcome> = asks.iter().map(|ask| ask.outcome).collect();
    assert_eq!(
        outcomes,
        [Outcome::Silence, Outcome::Refused, Outcome::Failed]
    );
    assert!(asks[1].attempts[0].faults.contains(&"cutoff".to_string()));
    assert_eq!(quiet_moments(&played), ["tale"]);
}

#[test]
fn only_the_last_lore_call_of_a_batch_with_shown_words_is_accepted() {
    let played = Played {
        calls: vec![
            call(1, "lore", "lore", "q", Some("Nobody knows")),
            call(
                1,
                "lore",
                "lore",
                "q retry",
                Some("VanCleef leads them [1]."),
            ),
        ],
        moments: BTreeMap::from([(1, "lore".to_string())]),
        replies: BTreeMap::from([(1, "VanCleef leads them.".to_string())]),
        rows: Vec::new(),
        failed_batches: Vec::new(),
    };

    let asks = asks_of(&played);

    let accepted: Vec<bool> = asks
        .iter()
        .flat_map(|ask| &ask.attempts)
        .map(|a| a.accepted)
        .collect();
    assert_eq!(accepted, [false, true]);
}

// The model runners --------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn a_shell_model_gives_its_words_and_a_failed_one_gives_none() {
    let echo = Runner::Shell("cat".to_string()).ask("Westfall burns.");
    let failed = Runner::Shell("exit 3".to_string()).ask("Westfall burns.");

    assert_eq!(echo.answer.as_deref(), Some("Westfall burns."));
    assert!(echo.first_byte.is_some());
    assert_eq!(failed.answer, None);
}

#[test]
fn a_local_answer_gives_its_text_and_its_tokens() {
    let body = br#"{"choices":[{"message":{"role":"assistant","content":" Westfall burns. "}}],"usage":{"prompt_tokens":900,"completion_tokens":31}}"#;

    assert_eq!(
        read_completion(body),
        (Some("Westfall burns.".to_string()), Some(31))
    );
    assert_eq!(read_completion(b"<html>502</html>"), (None, None));
    assert_eq!(read_completion(br#"{"choices":[]}"#), (None, None));
}

#[test]
#[cfg(unix)]
fn a_local_model_that_does_not_run_fails_its_call() {
    let local = timeways_dev::model_runner::LocalModel {
        url: "http://127.0.0.1:9".to_string(),
        model: "none".to_string(),
    };

    assert_eq!(Runner::Local(local).ask("Hello").answer, None);
}

// The counters of a process ------------------------------------------------------------------

#[test]
fn the_cpu_ticks_of_a_process_count_from_the_end_of_its_name() {
    let stat = "4242 (ollama (runner)) S 1 4242 4242 0 -1 4194560 100 0 0 0 750 250 0 0 20 0";

    assert_eq!(cpu_ticks(stat), Some(1000));
    assert_eq!(cpu_ticks("4242 (x) S 1"), None);
}

#[test]
fn the_memory_of_a_process_is_its_resident_set() {
    let status = "Name:\tollama\nVmPeak:\t 9999 kB\nVmRSS:\t 2048000 kB\n";

    assert_eq!(rss_kb(status), Some(2_048_000));
    assert_eq!(rss_kb("Name:\tx\n"), None);
}

#[test]
fn a_gpu_client_sums_its_engines_and_its_resident_memory() {
    let fdinfo = "pos:\t0\ndrm-driver:\ti915\ndrm-client-id:\t1634\n\
                  drm-resident-system0:\t64804 KiB\ndrm-resident-stolen-local0:\t2 MiB\n\
                  drm-engine-render:\t7785107460 ns\ndrm-engine-compute:\t47424 ns\n\
                  drm-engine-capacity-video:\t2\n";

    assert_eq!(
        drm_client(fdinfo),
        Some(DrmClient {
            id: 1634,
            engine_ns: 7_785_154_884,
            resident_kib: 64804 + 2048,
        })
    );
    assert_eq!(drm_client("pos:\t0\nflags:\t02\n"), None);
}

#[test]
fn nvidia_memory_counts_only_the_processes_of_the_model() {
    let csv = "4242, 2100\n777, 900\nbad line\n4243, 100\n";

    assert_eq!(nvidia_memory_of(csv, &[4242, 4243]), 2200);
}

// The phases of a frame rate run --------------------------------------------------------------

fn run(started: u64, samples: Vec<u32>) -> FpsRun {
    let count = u64::try_from(samples.len()).unwrap();
    FpsRun {
        label: "bench".to_string(),
        started,
        ended: started + count,
        samples,
        hidden: 0,
        min: 0,
        p5: 0,
        median: 0,
        mean: 0,
        memory_kb: None,
        cpu_ms: None,
    }
}

fn phase(name: &str, from: u64, to: u64) -> Phase {
    Phase {
        name: name.to_string(),
        from,
        to,
    }
}

#[test]
fn each_phase_takes_the_samples_of_its_seconds() {
    let run = run(100, vec![60, 60, 30, 30, 50, 50]);

    assert_eq!(samples_in(&run, 100, 102), [60.0, 60.0]);
    assert_eq!(samples_in(&run, 102, 104), [30.0, 30.0]);
    assert_eq!(samples_in(&run, 104, 200), [50.0, 50.0]);
    assert!(samples_in(&run, 0, 100).is_empty());
    assert!(samples_in(&self::run(100, Vec::new()), 0, 200).is_empty());
}

#[test]
fn the_drop_of_each_phase_compares_with_the_baseline() {
    let run = run(100, vec![60, 60, 30, 30, 50, 50]);
    let phases = [
        phase("baseline", 100, 102),
        phase("load", 102, 104),
        phase("recovery", 104, 106),
    ];

    let reports = phase_reports(&phases, Some(&run), &[]);

    let drops: Vec<Option<f64>> = reports.iter().map(|r| r.median_drop_percent).collect();
    assert_eq!(drops[0], Some(0.0));
    assert_eq!(drops[1], Some(50.0));
    assert!((drops[2].unwrap() - 100.0 / 6.0).abs() < 1e-9);
    assert!(
        phase_reports(&phases, None, &[])
            .iter()
            .all(|r| r.fps.is_none())
    );
}

#[test]
fn only_a_run_that_covers_the_load_counts_and_the_newest_wins() {
    let load = phase("load", 200, 260);
    let runs = [
        run(150, vec![60; 200]),
        run(190, vec![50; 100]),
        run(210, vec![40; 100]),
    ];

    let found = covering_run(&runs, &load).unwrap();

    assert_eq!(found.started, 190);
    assert!(covering_run(&runs[2..], &load).is_none());
}

// The format and the gate ---------------------------------------------------------------------

#[test]
fn a_bench_name_marks_its_batch_and_a_second_name_is_refused() {
    let text = "{\"type\":\"level_reached\",\"at\":1,\"level\":2}\n\n\
                {\"bench\":\"arrival\"}\n{\"type\":\"zone_entered\",\"at\":2,\"zone\":\"Westfall\"}\n";

    let scenario = Scenario::parse(text).unwrap();
    let twice = Scenario::parse("{\"bench\":\"a\"}\n{\"bench\":\"b\"}\n");

    assert_eq!(scenario.batches[0].moment, None);
    assert_eq!(scenario.batches[1].moment.as_deref(), Some("arrival"));
    assert_eq!(twice, Err(ScenarioError::BadMoment { line: 2 }));
}

#[test]
fn the_benches_and_the_commands_that_write_a_world_need_dev_mode() {
    for command in ["seed", "restore", "bench-model", "bench-fps"] {
        assert!(needs_dev_mode(command), "{command}");
    }
    for command in ["snapshot", "scenarios", "on", "off", "status"] {
        assert!(!needs_dev_mode(command), "{command}");
    }
}

#[test]
fn the_first_set_of_moments_holds_every_moment_of_the_bench() {
    let (_, _, text) = timeways_dev::bench::SETS[0];

    let scenario = Scenario::parse(text).unwrap();

    let moments: Vec<&str> = scenario
        .batches
        .iter()
        .filter_map(|batch| batch.moment.as_deref())
        .collect();
    for moment in [
        "arrival",
        "deed",
        "tenth-level",
        "dungeon-setup",
        "revenge",
        "tale",
        "saga",
        "summary",
        "zone-history",
        "talk",
        "lore",
    ] {
        assert!(moments.contains(&moment), "{moment}");
    }
}
