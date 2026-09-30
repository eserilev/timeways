//! Reads inputs from the bridge on stdin, and writes outputs on stdout (GAMEPLAY.md 5.12).
//!
//! Stdout carries only the protocol, so every problem goes to stderr.

use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use timeways_story::pack::Pack;
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
    let store = args.next().map_or(Store::Memory, Store::Folder);
    let result = Pack::open(&pack)
        .map_err(Box::from)
        .and_then(|pack| serve(&mut Story::new(pack, store)));
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
