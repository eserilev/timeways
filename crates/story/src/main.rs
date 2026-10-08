//! Reads inputs from the bridge on stdin, and writes outputs on stdout (GAMEPLAY.md 5.12).
//!
//! Stdout carries only the protocol, so every problem goes to stderr.

use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use timeways_story::dev_mode::DevMode;
use timeways_story::lore_start::open_lore;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let Some(pack) = args.next() else {
        eprintln!("usage: timeways-story <lore pack> [<data folder>]");
        return ExitCode::FAILURE;
    };
    // With no data folder, nothing is saved. That is for a run by hand.
    let folder = args.next();
    let result = open_lore(&pack, folder.as_deref())
        .map_err(Box::from)
        .and_then(|start| {
            let dev_mode = folder.as_deref().map_or(DevMode::Off, DevMode::of_folder);
            let mut story = Story::new(start.pack, folder.map_or(Store::Memory, Store::Folder));
            story.set_dev_mode(dev_mode);
            if dev_mode.is_on() {
                eprintln!("dev mode is on: /twdev lines land in the worlds");
            }
            if let Some(log) = start.log {
                eprintln!("{log}");
            }
            if let Some(notice) = start.notice {
                story.set_program_notice(notice);
            }
            serve(&mut story)
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn serve(story: &mut Story) -> Result<(), Box<dyn Error>> {
    let mut out = io::stdout().lock();
    let mut log = io::stderr().lock();
    for bytes in io::stdin().lock().split(b'\n') {
        let served = serve::line(story, bytes?);
        for line in served.lines {
            writeln!(out, "{line}")?;
        }
        if let Some(error) = served.error {
            writeln!(log, "{error}")?;
        }
        for note in served.notes {
            writeln!(log, "{note}")?;
        }
    }
    Ok(())
}
