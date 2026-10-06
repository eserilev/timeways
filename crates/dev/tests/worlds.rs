//! A seed, a snapshot, and a restore touch one world, and never lose the one before.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{NAME, REALM, folder, seed};
use std::path::{Path, PathBuf};
use timeways_dev::worlds::{
    Existing, WorldError, put_world, restore_snapshot, save_snapshot, snapshot_file, world_file,
};

/// A world of the fresh scenario, built in its own folder.
fn built_world(name: &str) -> PathBuf {
    let scratch = folder(name);
    seed("fresh", &scratch);
    world_file(&scratch, REALM, NAME).unwrap()
}

fn backups_of(world: &Path) -> Vec<PathBuf> {
    let prefix = format!("{}.bak-", world.file_name().unwrap().to_string_lossy());
    let mut found: Vec<_> = std::fs::read_dir(world.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            name.starts_with(&prefix) && name.ends_with(char::is_numeric)
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_world_goes_where_the_story_program_looks_for_it() {
    let data = Path::new("/data");

    let world = world_file(data, "Classic Beta PvP 2", "Ada").unwrap();

    assert_eq!(
        world,
        Path::new("/data/worlds/r_Classic_20Beta_20PvP_202/c_Ada.sqlite")
    );
}

#[test]
fn a_new_world_is_put_in_its_place() {
    let new = built_world("new-world");
    let data = folder("new-world-data");
    let world = world_file(&data, REALM, NAME).unwrap();

    let backup = put_world(&new, &world, Existing::Keep).unwrap();

    assert_eq!(backup, None);
    assert_eq!(std::fs::read(&world).unwrap(), std::fs::read(&new).unwrap());
}

#[test]
fn a_world_that_exists_stays_without_replace() {
    let new = built_world("keep");
    let data = folder("keep-data");
    let world = world_file(&data, REALM, NAME).unwrap();
    std::fs::create_dir_all(world.parent().unwrap()).unwrap();
    std::fs::write(&world, "the old world").unwrap();

    let result = put_world(&new, &world, Existing::Keep);

    assert!(matches!(result, Err(WorldError::Exists(_))));
    assert_eq!(std::fs::read_to_string(&world).unwrap(), "the old world");
}

#[test]
fn replace_moves_the_old_world_and_its_side_files_to_a_backup_first() {
    let new = built_world("replace");
    let data = folder("replace-data");
    let world = world_file(&data, REALM, NAME).unwrap();
    std::fs::create_dir_all(world.parent().unwrap()).unwrap();
    std::fs::write(&world, "the old world").unwrap();
    std::fs::write(format!("{}-wal", world.display()), "old writes").unwrap();

    let backup = put_world(&new, &world, Existing::Replace).unwrap().unwrap();

    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "the old world");
    let side = format!("{}-wal", backup.display());
    assert_eq!(std::fs::read_to_string(side).unwrap(), "old writes");
    assert!(!Path::new(&format!("{}-wal", world.display())).exists());
    assert_eq!(std::fs::read(&world).unwrap(), std::fs::read(&new).unwrap());
}

#[test]
fn two_backups_in_one_second_never_take_each_others_place() {
    let new = built_world("two-backups");
    let data = folder("two-backups-data");
    let world = world_file(&data, REALM, NAME).unwrap();
    std::fs::create_dir_all(world.parent().unwrap()).unwrap();
    std::fs::write(&world, "first").unwrap();

    put_world(&new, &world, Existing::Replace).unwrap();
    std::fs::write(&world, "second").unwrap();
    put_world(&new, &world, Existing::Replace).unwrap();

    let kept: Vec<_> = backups_of(&world)
        .iter()
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect();
    assert_eq!(kept.len(), 2);
    assert!(kept.contains(&"first".to_string()) && kept.contains(&"second".to_string()));
}

#[test]
fn a_seed_never_touches_another_character() {
    let new = built_world("other");
    let data = folder("other-data");
    let other = world_file(&data, REALM, "Kobee").unwrap();
    std::fs::create_dir_all(other.parent().unwrap()).unwrap();
    std::fs::write(&other, "Kobee's world").unwrap();

    put_world(
        &new,
        &world_file(&data, REALM, NAME).unwrap(),
        Existing::Replace,
    )
    .unwrap();

    assert_eq!(std::fs::read_to_string(&other).unwrap(), "Kobee's world");
    assert!(backups_of(&other).is_empty());
}

#[test]
fn a_snapshot_and_its_restore_bring_back_the_world_of_the_snapshot() {
    let data = folder("snapshot");
    let world = world_file(&data, REALM, NAME).unwrap();
    put_world(&built_world("snapshot-world"), &world, Existing::Keep).unwrap();
    let snapshot = snapshot_file(&data, REALM, NAME, "before-the-raid").unwrap();
    save_snapshot(&world, &snapshot).unwrap();
    std::fs::write(&world, "a world that changed").unwrap();

    let backup = restore_snapshot(&snapshot, &world).unwrap().unwrap();

    assert_eq!(
        std::fs::read_to_string(backup).unwrap(),
        "a world that changed"
    );
    let journal = common::journal(&data);
    assert_eq!(common::list(&journal, "chapters").len(), 1);
}

#[test]
fn a_snapshot_never_takes_the_place_of_another() {
    let data = folder("snapshot-twice");
    let world = world_file(&data, REALM, NAME).unwrap();
    put_world(&built_world("snapshot-twice-world"), &world, Existing::Keep).unwrap();
    let snapshot = snapshot_file(&data, REALM, NAME, "one").unwrap();
    save_snapshot(&world, &snapshot).unwrap();

    let again = save_snapshot(&world, &snapshot);

    assert!(matches!(again, Err(WorldError::Exists(_))));
}

#[test]
fn a_snapshot_name_is_a_plain_file_name() {
    let data = Path::new("/data");

    for name in ["", "../x", "a/b", "a b", "x.sqlite"] {
        let result = snapshot_file(data, REALM, NAME, name);
        assert!(matches!(result, Err(WorldError::BadSnapshotName)), "{name}");
    }
    assert!(snapshot_file(data, REALM, NAME, "level-30_v2").is_ok());
}

#[test]
fn a_restore_of_a_missing_snapshot_changes_nothing() {
    let data = folder("missing");
    let world = world_file(&data, REALM, NAME).unwrap();
    std::fs::create_dir_all(world.parent().unwrap()).unwrap();
    std::fs::write(&world, "the world").unwrap();
    let snapshot = snapshot_file(&data, REALM, NAME, "nothing").unwrap();

    let result = restore_snapshot(&snapshot, &world);

    assert!(matches!(result, Err(WorldError::Missing(_))));
    assert_eq!(std::fs::read_to_string(&world).unwrap(), "the world");
}
