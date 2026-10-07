//! `timeways-dev`: worlds from scenario files, snapshots of worlds, and the switch of dev
//! mode (TESTING.md, "Dev mode").

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use timeways_dev::bench_fps::{FpsBench, bench_fps, fps_text};
use timeways_dev::bench_model::{Bench, bench_model, write_results};
use timeways_dev::bench_report::{ModelReport, compare_text, text};
use timeways_dev::desktop::{Desktop, ModelSetting};
use timeways_dev::export_ratings::{ratings_of, write_export};
use timeways_dev::gate::{check_dev_mode, needs_dev_mode};
use timeways_dev::model_runner::Runner;
use timeways_dev::scenario::{BUILT_IN, Scenario, built_in};
use timeways_dev::seed::{Model, Report, play};
use timeways_dev::shell_model::shell_model;
use timeways_dev::worlds::{
    Existing, put_world, restore_snapshot, save_snapshot, snapshot_file, world_file,
};
use timeways_story::dev_mode::DevMode;
use timeways_story::pack::Pack;
use timeways_story::store::{CharacterKey, Store};
use timeways_story::story::Story;

const USAGE: &str = "usage:
  timeways-dev seed <character> --realm <realm> --scenario <name or file.jsonl>
      [--replace] [--no-model | --model <shell command>] [--pack <lore pack>] [--data <folder>]
  timeways-dev snapshot <character> <snapshot> --realm <realm> [--data <folder>]
  timeways-dev restore <character> <snapshot> --realm <realm> [--data <folder>]
  timeways-dev scenarios
  timeways-dev export-ratings <character> --realm <realm> [--out <file>] [--data <folder>]
  timeways-dev bench-model [--model <shell command> | --local | --claude | --compare local,claude]
      [--moments <v1 or file.jsonl>] [--runs <n>] [--pack <lore pack>] [--data <folder>]
  timeways-dev bench-fps --model <local | claude | shell command> [--seconds <n>] [--lead <n>]
      [--wait <n>] [--moments <v1 or file.jsonl>] [--pack <lore pack>] [--data <folder>]
  timeways-dev on | off | status

  seed, restore, and the benches run only while dev mode is on.
  export-ratings writes the ratings of a character into a file to send, with no player names.
  In --model and --compare, local is the local model of the config, or llama3.2:3b in
  Ollama, and claude is Claude Code with no tools.
  --data is the data folder of the story program. By default it is the one of the
  desktop app: <data>/gnomish-relay/timeways/story.";

/// The flags of a command, after its words.
#[derive(Default)]
struct Flags {
    realm: Option<String>,
    scenario: Option<String>,
    replace: bool,
    no_model: bool,
    model: Option<String>,
    pack: Option<PathBuf>,
    data: Option<PathBuf>,
    out: Option<PathBuf>,
    /// `local` or `claude`, from `--local` or `--claude`.
    named_model: Option<&'static str>,
    compare: Option<String>,
    moments: Option<String>,
    runs: Option<String>,
    seconds: Option<String>,
    lead: Option<String>,
    wait: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some((command, rest)) = args.split_first() else {
        return Err(USAGE.into());
    };
    let (words, flags) = split_flags(rest).ok_or(USAGE)?;
    if needs_dev_mode(command) {
        check_dev_mode(command, &data_folder(&flags)?)?;
    }
    match (command.as_str(), words.as_slice()) {
        ("seed", [character]) => seed(character, &flags),
        ("snapshot", [character, name]) => snapshot(character, name, &flags),
        ("restore", [character, name]) => restore(character, name, &flags),
        ("export-ratings", [character]) => export_ratings(character, &flags),
        ("scenarios", []) => {
            for (name, about, _) in BUILT_IN {
                println!("{name:18} {about}");
            }
            Ok(())
        }
        ("bench-model", []) => bench_models(&flags),
        ("bench-fps", []) => bench_frame_rate(&flags),
        ("on", []) => switch(DevMode::On),
        ("off", []) => switch(DevMode::Off),
        ("status", []) => {
            let mode = Desktop::here()?.dev_mode();
            println!("Dev mode is {}.", if mode.is_on() { "on" } else { "off" });
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn split_flags(args: &[String]) -> Option<(Vec<String>, Flags)> {
    let mut words = Vec::new();
    let mut flags = Flags::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--replace" => flags.replace = true,
            "--no-model" => flags.no_model = true,
            "--local" => flags.named_model = Some("local"),
            "--claude" => flags.named_model = Some("claude"),
            "--compare" => flags.compare = Some(rest.next()?.clone()),
            "--moments" => flags.moments = Some(rest.next()?.clone()),
            "--runs" => flags.runs = Some(rest.next()?.clone()),
            "--seconds" => flags.seconds = Some(rest.next()?.clone()),
            "--lead" => flags.lead = Some(rest.next()?.clone()),
            "--wait" => flags.wait = Some(rest.next()?.clone()),
            "--realm" => flags.realm = Some(rest.next()?.clone()),
            "--scenario" => flags.scenario = Some(rest.next()?.clone()),
            "--model" => flags.model = Some(rest.next()?.clone()),
            "--pack" => flags.pack = Some(PathBuf::from(rest.next()?)),
            "--data" => flags.data = Some(PathBuf::from(rest.next()?)),
            "--out" => flags.out = Some(PathBuf::from(rest.next()?)),
            flag if flag.starts_with("--") => return None,
            word => words.push(word.to_string()),
        }
    }
    Some((words, flags))
}

fn data_folder(flags: &Flags) -> Result<PathBuf, Box<dyn Error>> {
    match &flags.data {
        Some(data) => Ok(data.clone()),
        None => Ok(Desktop::here()?.story),
    }
}

fn realm(flags: &Flags) -> Result<&str, Box<dyn Error>> {
    flags
        .realm
        .as_deref()
        .ok_or_else(|| "--realm is missing: the realm as the game names it".into())
}

fn seed(character: &str, flags: &Flags) -> Result<(), Box<dyn Error>> {
    let realm = realm(flags)?;
    let name = flags.scenario.as_deref().ok_or("--scenario is missing")?;
    let scenario = Scenario::parse(&scenario_text(name)?)?;
    let world = world_file(&data_folder(flags)?, realm, character)?;
    let existing = if flags.replace {
        Existing::Replace
    } else {
        Existing::Keep
    };
    if world.exists() && existing == Existing::Keep {
        return Err(format!(
            "{} exists. Add --replace to move it to a backup and write a new one.",
            world.display()
        )
        .into());
    }
    let scratch = std::env::temp_dir().join(format!("timeways-dev-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let result = seed_in(&scratch, &scenario, realm, character, flags).and_then(|built| {
        let backup = put_world(&built, &world, existing)?;
        if let Some(backup) = backup {
            println!("The old world is now {}", backup.display());
        }
        println!("Wrote {}", world.display());
        println!("Run `gnomish-relay restart`, then log in as {character}.");
        Ok(())
    });
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

/// Plays the scenario into a world in `scratch`, and gives its file. A scenario with a
/// refused line writes nothing.
fn seed_in(
    scratch: &Path,
    scenario: &Scenario,
    realm: &str,
    character: &str,
    flags: &Flags,
) -> Result<PathBuf, Box<dyn Error>> {
    let story = Story::new(lore_pack(flags)?, Store::Folder(scratch.to_path_buf()));
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    // The last line lands a minute ago, so the world looks like play of today.
    let start = now.saturating_sub(scenario.span() + 60);
    let report = play(
        &scenario.starting_at(start),
        realm,
        character,
        story,
        model(flags)?,
    );
    print_report(&report);
    if !report.is_clean() {
        return Err("the scenario broke a rule, so no world was written".into());
    }
    Ok(scratch.join(CharacterKey::new(realm, character)?.relative_path()))
}

fn scenario_text(name: &str) -> Result<String, Box<dyn Error>> {
    if let Some(text) = built_in(name) {
        return Ok(text.to_string());
    }
    if Path::new(name)
        .extension()
        .is_some_and(|ext| ext == "jsonl")
    {
        return Ok(std::fs::read_to_string(name)?);
    }
    Err(format!("no scenario named {name}: see `timeways-dev scenarios`").into())
}

fn lore_pack(flags: &Flags) -> Result<Pack, Box<dyn Error>> {
    let path = match &flags.pack {
        Some(path) => Some(path.clone()),
        None => Desktop::here()?.lore_pack(),
    };
    if let Some(path) = path.filter(|path| path.exists()) {
        return Ok(Pack::open(&path)?);
    }
    println!("No lore pack: the narrator gets no lore.");
    Ok(Pack::empty()?)
}

fn model(flags: &Flags) -> Result<Option<Model>, Box<dyn Error>> {
    if flags.no_model {
        return Ok(None);
    }
    if let Some(command) = &flags.model {
        return Ok(Some(shell_model(command.clone())));
    }
    match Desktop::here()?.model() {
        ModelSetting::Command(command) => {
            println!("Model calls go to: {command}");
            Ok(Some(shell_model(command)))
        }
        ModelSetting::None => {
            println!("No model in the config: every model call fails, as in the game.");
            Ok(None)
        }
        ModelSetting::Local(local) => {
            println!("Model calls go to: {} at {}", local.model, local.url);
            let runner = Runner::Local(local);
            Ok(Some(Box::new(move |prompt| runner.ask(prompt).answer)))
        }
        ModelSetting::Other(kind) => Err(format!(
            "the model \"{kind}\" of the config needs --model <shell command>, or --no-model"
        )
        .into()),
    }
}

fn print_report(report: &Report) {
    println!("Played {} batches.", report.batches);
    for line in &report.narrator {
        println!("Narrator: {line}");
    }
    for line in &report.notices {
        println!("Timeways: {line}");
    }
    for line in &report.refused {
        eprintln!("Refused: {line}");
    }
    if report.dropped > 0 {
        eprintln!("The bridge dropped {} lines.", report.dropped);
    }
    for answer in &report.unused_answers {
        eprintln!("No call took the answer {answer}");
    }
    for error in &report.failed_batches {
        eprintln!("A batch failed: {error}");
    }
}

fn snapshot(character: &str, snapshot: &str, flags: &Flags) -> Result<(), Box<dyn Error>> {
    let data = data_folder(flags)?;
    let realm = realm(flags)?;
    let file = snapshot_file(&data, realm, character, snapshot)?;
    save_snapshot(&world_file(&data, realm, character)?, &file)?;
    println!("Saved {}", file.display());
    Ok(())
}

fn restore(character: &str, snapshot: &str, flags: &Flags) -> Result<(), Box<dyn Error>> {
    let data = data_folder(flags)?;
    let realm = realm(flags)?;
    let file = snapshot_file(&data, realm, character, snapshot)?;
    let world = world_file(&data, realm, character)?;
    if let Some(backup) = restore_snapshot(&file, &world)? {
        println!("The old world is now {}", backup.display());
    }
    println!("Restored {}", world.display());
    println!("Run `gnomish-relay restart`, then log in as {character}.");
    Ok(())
}

fn export_ratings(character: &str, flags: &Flags) -> Result<(), Box<dyn Error>> {
    let data = data_folder(flags)?;
    let world = world_file(&data, realm(flags)?, character)?;
    let export = ratings_of(&world, character, &Desktop::here()?.model_label())?;
    let file = write_export(&export, &data, flags.out.as_deref(), stamp())?;
    println!(
        "Wrote {} ratings to {}",
        export.ratings.len(),
        file.display()
    );
    println!("It holds no player names. Read it before you send it.");
    Ok(())
}

fn switch(mode: DevMode) -> Result<(), Box<dyn Error>> {
    let desktop = Desktop::here()?;
    desktop.set_dev_mode(mode)?;
    let word = if mode.is_on() { "on" } else { "off" };
    println!("Dev mode is {word}. Run `gnomish-relay restart` to apply it.");
    Ok(())
}

/// A number flag, or its default.
fn number(flag: Option<&String>, name: &str, default: u64) -> Result<u64, Box<dyn Error>> {
    match flag {
        None => Ok(default),
        Some(text) => text
            .parse()
            .map_err(|_| format!("{name} needs a whole number, not {text}").into()),
    }
}

/// `local`, `claude`, or a shell command that reads the prompt on stdin.
fn runner_named(name: &str) -> Result<Runner, Box<dyn Error>> {
    let desktop = Desktop::here()?;
    Ok(match name {
        "local" => Runner::Local(desktop.local_model()),
        "claude" => Runner::Shell(desktop.claude_command()),
        command => Runner::Shell(command.to_string()),
    })
}

/// The models of `bench-model`: the flags, or the model of the config.
fn bench_runners(flags: &Flags) -> Result<Vec<Runner>, Box<dyn Error>> {
    if let Some(list) = &flags.compare {
        return list
            .split(',')
            .map(|name| match (name.trim(), &flags.model) {
                ("model", Some(command)) => Ok(Runner::Shell(command.clone())),
                ("model", None) => Err("--compare model needs --model <shell command>".into()),
                (name, _) => runner_named(name),
            })
            .collect();
    }
    if let Some(command) = &flags.model {
        return Ok(vec![runner_named(command)?]);
    }
    if let Some(name) = flags.named_model {
        return Ok(vec![runner_named(name)?]);
    }
    match Desktop::here()?.model() {
        ModelSetting::Command(command) => Ok(vec![Runner::Shell(command)]),
        ModelSetting::Local(local) => Ok(vec![Runner::Local(local)]),
        ModelSetting::None | ModelSetting::Other(_) => {
            Err("no model to bench: give --model, --local, --claude, or --compare".into())
        }
    }
}

fn bench_of(flags: &Flags) -> Result<Bench, Box<dyn Error>> {
    let set = flags.moments.as_deref().unwrap_or("v1");
    let runs = usize::try_from(number(flags.runs.as_ref(), "--runs", 1)?)?.max(1);
    let pack = match &flags.pack {
        Some(path) => Some(path.clone()),
        None => Desktop::here()?.lore_pack(),
    };
    let pack = pack.filter(|path| path.exists());
    if pack.is_none() {
        println!("No lore pack: most moments get no lore, and no model call.");
    }
    Ok(Bench::of_set(set, pack, runs)?)
}

fn stamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn bench_models(flags: &Flags) -> Result<(), Box<dyn Error>> {
    let bench = bench_of(flags)?;
    let mut reports: Vec<ModelReport> = Vec::new();
    for runner in bench_runners(flags)? {
        println!("Benching {} ...", runner.name());
        reports.push(bench_model(&bench, &runner)?);
    }
    let readable = match reports.as_slice() {
        [one] => text(one),
        many => compare_text(many),
    };
    println!("{readable}");
    let (json, txt) = write_results(
        &data_folder(flags)?,
        &format!("model-{}", stamp()),
        &reports,
        &readable,
    )?;
    println!("Wrote {}\nWrote {}", json.display(), txt.display());
    Ok(())
}

fn bench_frame_rate(flags: &Flags) -> Result<(), Box<dyn Error>> {
    let name = flags
        .model
        .as_deref()
        .ok_or("--model is missing: local, claude, or a shell command")?;
    let fps = FpsBench {
        bench: bench_of(flags)?,
        runner: runner_named(name)?,
        seconds: number(flags.seconds.as_ref(), "--seconds", 60)?,
        lead: number(flags.lead.as_ref(), "--lead", 15)?,
        wait: number(flags.wait.as_ref(), "--wait", 300)?,
    };
    let story = data_folder(flags)?;
    let report = bench_fps(&fps, &story, &mut |line| println!("{line}"))?;
    let readable = fps_text(&report);
    println!("{readable}");
    let (json, txt) = write_results(&story, &format!("fps-{}", stamp()), &report, &readable)?;
    println!("Wrote {}\nWrote {}", json.display(), txt.display());
    Ok(())
}
