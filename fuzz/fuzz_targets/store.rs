//! Damaged files of a character. Opening them always works, because a crash can leave any
//! damage (GAMEPLAY.md 5.7). Only a history from another program is refused. The repair of
//! the first open leaves files that a second open reads the same way.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use timeways_story::store::{CharacterKey, Opened, Store, StoreError};

const FILES: [&str; 6] = [
    "c_Ada.jsonl",
    "c_Ada.flavor.jsonl",
    "c_Ada.hero.jsonl",
    "c_Ada.chronicle.jsonl",
    "c_Ada.learned.jsonl",
    "c_Ada.quests.jsonl",
];

fn folder() -> PathBuf {
    std::env::temp_dir().join(format!("timeways-fuzz-store-{}", std::process::id()))
}

/// The counts of what each file gave.
fn counts(opened: &Opened) -> [usize; 8] {
    [
        opened.history.as_ref().map_or(0, |file| file.len()),
        opened.prose.len(),
        opened.flavor.moments().len(),
        opened.flavor.told().len(),
        opened.hero.changes().len(),
        opened.learned.read().len(),
        opened.learned.rumors().len(),
        opened.quests.changes().len(),
    ]
}

fuzz_target!(|data: &[u8]| {
    let root = folder();
    let _ = std::fs::remove_dir_all(&root);
    let realm = root.join("worlds").join("r_Stormrage");
    std::fs::create_dir_all(&realm).unwrap();
    for (file, bytes) in FILES.iter().zip(data.split(|byte| *byte == 0xFF)) {
        std::fs::write(realm.join(file), bytes).unwrap();
    }
    let store = Store::Folder(root);
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    let first = match store.open(&key) {
        Ok(first) => first,
        Err(StoreError::Foreign { .. }) => return,
        Err(error) => panic!("a damaged file locked the character out: {error}"),
    };
    let second = store.open(&key).unwrap();
    assert_eq!(counts(&first), counts(&second));
});
