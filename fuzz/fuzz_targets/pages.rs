//! Random play through the fake bridge, with a model that answers each kind of call, then
//! every page of the journal. The real checks of the relay take each answer, each page fits
//! a slot of the game, the world stays open, and the quest limits hold.

#![no_main]

#[path = "common.rs"]
mod common;
#[path = "play.rs"]
#[allow(dead_code, reason = "this target reads the journal, not each reply")]
mod play;

use fake_bridge::FakeBridge;
use libfuzzer_sys::fuzz_target;
use play::{Run, journal_pages};
use serde_json::{Value, json};

fuzz_target!(|run: Run| {
    let played = play::play(run, play::story(common::pack()));
    let (mut bridge, character) = (played.bridge, played.character);

    // After any play, an event at the time of the clock still lands in the world.
    let zone = json!({"type": "zone_entered", "at": now(), "zone": "Newvale"});
    bridge.batch(&format!("{character}\n{zone}"));
    let journal = whole_journal(&mut bridge, character);
    let places = journal["places"].as_array().cloned().unwrap_or_default();
    assert!(
        places.iter().any(|place| place["name"] == "Newvale"),
        "an event after the play did not land"
    );
    assert_quest_limits(&journal["quests"]);
});

fn now() -> u64 {
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since_epoch.map_or(0, |elapsed| elapsed.as_secs())
}

/// Every page, with the lists of each page joined in order.
fn whole_journal(bridge: &mut FakeBridge, character: &str) -> Value {
    let mut lists: serde_json::Map<String, Value> = serde_json::Map::new();
    for page in journal_pages(bridge, character) {
        let page: Value = serde_json::from_str(&page).unwrap();
        for name in ["places", "quests"] {
            let items = page[name].as_array().cloned().unwrap_or_default();
            let joined = lists.entry(name).or_insert_with(|| json!([]));
            if let Some(joined) = joined.as_array_mut() {
                joined.extend(items);
            }
        }
    }
    Value::Object(lists)
}

/// At most 3 open quests, at most one open quest and one waiting offer for each giver
/// (GAMEPLAY.md 3.4).
fn assert_quest_limits(quests: &Value) {
    let quests = quests.as_array().cloned().unwrap_or_default();
    for status in ["accepted", "offered"] {
        let mut givers: Vec<&str> = quests
            .iter()
            .filter(|quest| quest["status"] == status)
            .filter_map(|quest| quest["giver"].as_str())
            .collect();
        if status == "accepted" {
            assert!(givers.len() <= 3, "{} open quests", givers.len());
        }
        givers.sort_unstable();
        assert!(
            givers.windows(2).all(|pair| pair[0] != pair[1]),
            "two {status} quests of one giver"
        );
    }
}
