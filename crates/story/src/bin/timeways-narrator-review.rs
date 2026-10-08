//! Prints the narrator prompt of each big moment of a world, for review (GAMEPLAY.md
//! 3.2.1). With `--model`, it also sends each prompt to a model and prints the line that
//! the player would see. It reads a copy of the world, and never writes the world.
//!
//! ```text
//! timeways-narrator-review <world .sqlite> [--pack <lore pack>] [--race <token>]
//!     [--class <token>] [--model <shell command> | --claude]
//! ```

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use timeways_rules::narrator_shapes::WINDOW;
use timeways_story::learned::Read;
use timeways_story::line_check::{Checked, checked_line};
use timeways_story::narrator::{Who, what_happened};
use timeways_story::narrator_build::{Answered, Built, Setup, answered};
use timeways_story::narrator_review::{Review, Sources, reviews, template_lines};
use timeways_story::pack::Pack;
use timeways_story::prompt;
use timeways_story::store::{CharacterKey, Store, name_of_safe_id};

const USAGE: &str = "usage: timeways-narrator-review <world .sqlite> [--pack <lore pack>] \
[--race <token>] [--class <token>] [--model <shell command> | --claude]
       timeways-narrator-review --templates [--race <token>] [--class <token>]
  The world lives in <data folder>/worlds/r_<realm>/c_<name>.sqlite.
  A token is the one of the game: --race Scourge --class WARLOCK.
  --model runs the shell command with each prompt on stdin, and prints its line.
  --claude is the model of the live voice test: Claude Code with no tools.";

/// Claude Code with no tools, no MCP servers, and no settings, as the bridge runs it.
const CLAUDE: &str = "claude -p --tools '' --strict-mcp-config --setting-sources ''";

#[derive(Default)]
struct Options {
    world: PathBuf,
    pack: Option<PathBuf>,
    who: Who,
    model: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--templates") {
        return print_templates(&args[1..]);
    }
    let Some(options) = options(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match review(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn options(args: &[String]) -> Option<Options> {
    let (world, mut rest) = args.split_first()?;
    let mut options = Options {
        world: PathBuf::from(world),
        ..Options::default()
    };
    while let Some((flag, after)) = rest.split_first() {
        if flag == "--claude" {
            options.model = Some(CLAUDE.to_string());
            rest = after;
            continue;
        }
        let (value, after) = after.split_first()?;
        match flag.as_str() {
            "--pack" => options.pack = Some(PathBuf::from(value)),
            "--race" => options.who.race = Some(token(value)?),
            "--class" => options.who.class = Some(token(value)?),
            "--model" => options.model = Some(value.clone()),
            _ => return None,
        }
        rest = after;
    }
    Some(options)
}

/// Every line of the templates for a pairing, with fixed values (the flag `--templates`).
fn print_templates(args: &[String]) -> ExitCode {
    let Some(options) = options(
        &[String::new()]
            .into_iter()
            .chain(args.iter().cloned())
            .collect::<Vec<_>>(),
    ) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    for line in template_lines(&options.who) {
        println!("{line}");
    }
    ExitCode::SUCCESS
}

fn token<T: serde::de::DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).ok()
}

fn review(options: &Options) -> Result<(), Box<dyn Error>> {
    let copy = std::env::temp_dir().join(format!("timeways-review-{}", std::process::id()));
    let result = review_copy(options, &copy);
    let _ = fs::remove_dir_all(&copy);
    result
}

/// The world opens from a copy, because opening a world can repair it.
fn review_copy(options: &Options, copy: &Path) -> Result<(), Box<dyn Error>> {
    let key = copy_world(&options.world, copy)?;
    let opened = Store::Folder(copy.to_path_buf()).open(&key)?;
    let pack = match &options.pack {
        Some(path) => Pack::open(path)?,
        None => Pack::empty()?,
    };
    let events: Vec<_> = opened.character.world().history().iter().cloned().collect();
    let reads: Vec<Read> = opened.learned.read().to_vec();
    let sources = Sources {
        pack: &pack,
        reads: &reads,
        fallback: &options.who,
    };
    let found = reviews(&events, &sources)?;
    let mut recent: Vec<String> = Vec::new();
    for (number, review) in found.iter().enumerate() {
        println!(
            "=== {} of {}, at {}: {}",
            number + 1,
            found.len(),
            review.at.0,
            what_happened(&review.moment)
        );
        if let Some(reason) = &review.silence {
            println!("Shown: (silence: {reason})\n");
            continue;
        }
        match &options.model {
            Some(model) => {
                if let Some(shape) = told(model, review, &recent)? {
                    recent.push(shape);
                    if recent.len() > WINDOW {
                        recent.remove(0);
                    }
                }
            }
            None => println!("{}\n", review.prompt),
        }
    }
    Ok(())
}

/// The copy keeps the folders of the data folder, because the store finds a world by them.
fn copy_world(world: &Path, copy: &Path) -> Result<CharacterKey, Box<dyn Error>> {
    let bad_path = || {
        format!(
            "{} is not worlds/r_<realm>/c_<name>.sqlite",
            world.display()
        )
    };
    let file = world
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(bad_path)?;
    let realm_folder = world
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(bad_path)?;
    let name = file
        .strip_prefix("c_")
        .and_then(|rest| rest.strip_suffix(".sqlite"))
        .and_then(name_of_safe_id)
        .ok_or_else(bad_path)?;
    let realm = realm_folder
        .strip_prefix("r_")
        .and_then(name_of_safe_id)
        .ok_or_else(bad_path)?;
    let folder = copy.join("worlds").join(realm_folder);
    fs::create_dir_all(&folder)?;
    fs::copy(world, folder.join(file))?;
    for journal in ["-wal", "-shm"] {
        let side = PathBuf::from(format!("{}{journal}", world.display()));
        if side.exists() {
            fs::copy(&side, folder.join(format!("{file}{journal}")))?;
        }
    }
    Ok(CharacterKey::new(&realm, &name)?)
}

/// The line as the player would see it: one retry with the reasons, as the story program
/// does, and silence after a second refusal. Returns the main part of a shown line.
fn told(model: &str, review: &Review, recent: &[String]) -> Result<Option<String>, Box<dyn Error>> {
    let answer = ask(model, &review.prompt)?;
    println!("Answer: {answer}");
    let Answered::Refused(faults) = verdict(review, &answer, recent) else {
        return Ok(shown(&verdict(review, &answer, recent)));
    };
    let reasons: Vec<String> = faults.iter().map(ToString::to_string).collect();
    println!("Refused: {}", reasons.join(" "));
    let retry = ask(model, &prompt::retry(&review.prompt, &answer, &reasons))?;
    println!("Retry: {retry}");
    Ok(shown(&verdict(review, &retry, recent)))
}

fn verdict(review: &Review, answer: &str, recent: &[String]) -> Answered {
    if let Some((setup, offer)) = &review.templated {
        let setup = Setup {
            recent: recent.to_vec(),
            setup_foe: None,
            ..setup.clone()
        };
        return answered(answer, &setup, offer, &review.grounds, "");
    }
    match checked_line(answer, &review.grounds, "") {
        Checked::Line(line) => Answered::Line(Built {
            line,
            shape: String::new(),
            parts: Vec::new(),
            named: false,
        }),
        Checked::Silent => Answered::Silent,
        Checked::Refused(faults) => Answered::Refused(faults),
    }
}

fn shown(verdict: &Answered) -> Option<String> {
    match verdict {
        Answered::Line(built) => {
            println!("Shown: {}", built.line);
            println!("Shape: {}\n", built.parts.join(" + "));
            (!built.shape.is_empty()).then(|| built.shape.clone())
        }
        Answered::Silent => {
            println!("Shown: (silence: the model had nothing to tell)\n");
            None
        }
        Answered::Refused(faults) => {
            println!("Shown: (silence: {faults:?})\n");
            None
        }
    }
}

fn ask(model: &str, prompt: &str) -> Result<String, Box<dyn Error>> {
    let mut child = Command::new("sh")
        .args(["-c", model])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let written = child
        .stdin
        .take()
        .ok_or("the model has no stdin")?
        .write_all(prompt.as_bytes());
    // A model that never reads the prompt, such as `echo`, can exit before the write ends.
    if let Err(error) = written
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    let output = child.wait_with_output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
