//! `timeways-dev`: worlds from scenario files, snapshots of worlds, and the switch of dev
//! mode (TESTING.md, "Dev mode").

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use timeways_dev::desktop::{Desktop, ModelSetting};
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
  timeways-dev on | off | status

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
    match (command.as_str(), words.as_slice()) {
        ("seed", [character]) => seed(character, &flags),
        ("snapshot", [character, name]) => snapshot(character, name, &flags),
        ("restore", [character, name]) => restore(character, name, &flags),
        ("scenarios", []) => {
            for (name, about, _) in BUILT_IN {
                println!("{name:18} {about}");
            }
            Ok(())
        }
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
            "--realm" => flags.realm = Some(rest.next()?.clone()),
            "--scenario" => flags.scenario = Some(rest.next()?.clone()),
            "--model" => flags.model = Some(rest.next()?.clone()),
            "--pack" => flags.pack = Some(PathBuf::from(rest.next()?)),
            "--data" => flags.data = Some(PathBuf::from(rest.next()?)),
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

fn switch(mode: DevMode) -> Result<(), Box<dyn Error>> {
    let desktop = Desktop::here()?;
    desktop.set_dev_mode(mode)?;
    let word = if mode.is_on() { "on" } else { "off" };
    println!("Dev mode is {word}. Run `gnomish-relay restart` to apply it.");
    Ok(())
}
