//! Random model answers and random player text. Every text that passes a check keeps its
//! limits, and the search never fails on syntax.

#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use timeways_story::check::{
    check, in_voice, later_names, names_after_cutoff, names_after_cutoff_except, plain_text,
    same_words, slop_in,
};
use timeways_story::line_check::{Checked, Grounds, checked_line, grounded};
use timeways_story::moments::Moment;
use timeways_story::draft;
use timeways_story::house::without_fence_marks;
use timeways_story::input::MessageId;
use timeways_story::quest::variety::{Recent, Shape, main_words};
use timeways_story::quest::{self, Genre, Known, Step};
use timeways_story::seen::{SeenText, TextKind};
use timeways_story::story::Output;
use timeways_story::{chronicle, hero, narrator, summary, talk};

fn assert_plain(text: &str, max_chars: usize, max_bytes: usize) {
    assert!(
        text.chars().count() <= max_chars && text.len() <= max_bytes,
        "{text:?}"
    );
    assert!(!text.chars().any(char::is_control), "{text:?}");
    assert!(!text.is_empty() && text.trim() == text, "{text:?}");
    assert!(names_after_cutoff(text).is_empty(), "{text:?}");
}

fn assert_voice(text: &str, max_chars: usize, max_bytes: usize) {
    assert_plain(text, max_chars, max_bytes);
    assert!(in_voice(text), "{text:?}");
}

fn known(seen: &[SeenText]) -> Known<'_> {
    Known {
        giver: "Keeper Tessa",
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower", "Old Mill"],
        npcs: vec!["Keeper Tessa", "Farmer Bram"],
        foes: vec!["Duskbat"],
        last_targets: vec!["Old Mill"],
        seen,
        goods: vec!["Linen Cloth", "Light Leather"],
        recent: recent(),
        level: Some(12),
        dungeons: vec!["The Deadmines"],
        bosses: vec!["Edwin VanCleef"],
        game_quests: vec!["The Defias Brotherhood"],
        game_quests_done: vec!["Report to Goldtooth"],
    }
}

/// Three offers, newest first: a visit, a meeting, and a kill.
fn recent() -> Vec<Recent> {
    let offer = |title: &str, step: Step| Recent {
        number: 1,
        title: title.to_string(),
        shape: Shape::of(&[step], None),
        genre: Some(Genre::Errand),
    };
    vec![
        offer(
            "The Lost Lantern",
            Step::Visit {
                place: "Old Mill".to_string(),
            },
        ),
        offer(
            "Old Debts",
            Step::Meet {
                npc: "Farmer Bram".to_string(),
            },
        ),
        offer(
            "Bats in the Belfry",
            Step::Kill {
                creature: "Duskbat".to_string(),
                count: 2,
            },
        ),
    ]
}

/// The shape differs from the first 2 recent shapes, and no main word of the title is in
/// the first 3 recent titles.
fn assert_variety(offer: &quest::Quest) {
    let recent = recent();
    let shape = Shape::of(&offer.steps, offer.any_order);
    assert!(recent[..2].iter().all(|old| old.shape != shape), "{shape}");
    let words = main_words(&offer.title);
    for old in &recent {
        assert!(
            main_words(&old.title)
                .iter()
                .all(|word| !words.contains(word)),
            "{}",
            offer.title
        );
    }
}

fn assert_quest(text: &str) {
    let seen = [SeenText {
        kind: TextKind::Quest,
        title: Some("Rats".to_string()),
        npc: None,
        zone: None,
        text: "Miller Oda needs help.".to_string(),
    }];
    let known = known(&seen);
    let Ok(offer) = quest::checked_quest(text, &known) else {
        return;
    };
    assert_plain(&offer.title, quest::MAX_TITLE_CHARS, quest::MAX_OFFER_BYTES);
    assert_plain(&offer.text, quest::MAX_TEXT_CHARS, quest::MAX_OFFER_BYTES);
    assert!((1..=quest::MAX_STEPS).contains(&offer.steps.len()));
    assert!(
        !same_words(&offer.title, "Rats"),
        "the title of a game quest: {}",
        offer.title
    );
    assert!(quest::offer_line(known.giver, &offer).len() <= quest::MAX_OFFER_BYTES);
    for (n, step) in offer.steps.iter().enumerate() {
        assert_step(step, &offer.steps[..n]);
    }
    let waits: Vec<usize> = (0..offer.steps.len())
        .filter(|n| matches!(offer.steps[*n], Step::Wait { .. }))
        .collect();
    assert!(waits.len() <= 1, "two waits: {:?}", offer.steps);
    for wait in waits {
        assert!(
            wait > 0 && wait + 1 < offer.steps.len(),
            "{:?}",
            offer.steps
        );
    }
    if let Some(span) = offer.any_order {
        assert!((2..=3).contains(&(span.last + 1 - span.first)), "{span:?}");
        assert!(span.last < offer.steps.len(), "{span:?}");
        let set = &offer.steps[span.first..=span.last];
        assert!(!set.iter().any(|step| matches!(step, Step::Wait { .. })));
    }
    let slaps = offer
        .steps
        .iter()
        .any(|step| matches!(step, Step::Slap { .. }));
    assert!(!slaps || offer.genre == Genre::Comic, "{:?}", offer.genre);
    assert_variety(&offer);
    let targets: Vec<&str> = offer.steps.iter().filter_map(Step::target).collect();
    let mut distinct = targets.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), targets.len(), "a target twice: {targets:?}");
}

/// One step of a passed offer keeps its list and its range. `before` holds the steps
/// before it: only a step after a wait names the giver.
fn assert_step(step: &Step, before: &[Step]) {
    let after_wait = before.iter().any(|step| matches!(step, Step::Wait { .. }));
    let person = |npc: &str| npc == "Farmer Bram" || (after_wait && npc == "Keeper Tessa");
    match step {
        Step::Visit { place } | Step::VisitAt { place, .. } => {
            assert!(["Testvale", "Old Tower"].contains(&place.as_str()));
        }
        Step::Meet { npc } => assert!(person(npc), "{npc}"),
        Step::Talk { npc, about } => {
            assert!(person(npc), "{npc}");
            if let Some(about) = about {
                assert!(about.chars().count() <= quest::MAX_TOPIC_CHARS, "{about:?}");
                assert!(!about.contains('|') && !about.chars().any(char::is_control));
            }
        }
        Step::Kill { creature, count } => {
            assert_eq!(creature, "Duskbat");
            assert!((1..=quest::MAX_KILLS).contains(count));
        }
        Step::Wait { days } => assert!((1..=quest::MAX_WAIT_DAYS).contains(days)),
        Step::Level { level } => assert!((13..=15).contains(level), "{level}"),
        Step::Enter { dungeon } => assert_eq!(dungeon, "The Deadmines"),
        Step::Defeat { boss } => assert_eq!(boss, "Edwin VanCleef"),
        Step::GameQuest { title } => assert_eq!(title, "The Defias Brotherhood"),
        Step::Emote { emote, npc, place } => {
            assert!(quest::quest_emotes().contains(&emote.as_str()), "{emote}");
            assert!(npc.is_some() != place.is_some(), "{npc:?} {place:?}");
            assert!(npc.as_deref().is_none_or(person));
            assert!(
                place
                    .as_deref()
                    .is_none_or(|place| ["Testvale", "Old Tower"].contains(&place))
            );
        }
        Step::Slap { npc } => assert_eq!(npc, "Farmer Bram"),
        Step::Carry { item, count, npc } => {
            assert!(
                ["Linen Cloth", "Light Leather"].contains(&item.as_str()),
                "{item}"
            );
            assert!((1..=quest::MAX_CARRY).contains(count));
            assert!(person(npc), "{npc}");
        }
    }
}

/// A draft that passes fits the addon messages of a player task, and a reply of the bridge.
fn assert_draft(text: &str) {
    let known = draft::Known {
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower"],
        npcs: vec!["Farmer Bram"],
        foes: vec!["Old Gnasher"],
    };
    let Ok(checked) = draft::checked_draft(text, &known) else {
        return;
    };
    assert_voice(
        &checked.title,
        draft::MAX_TITLE_BYTES,
        draft::MAX_TITLE_BYTES,
    );
    assert_voice(&checked.text, draft::MAX_TEXT_BYTES, draft::MAX_TEXT_BYTES);
    assert!(!checked.title.contains('|') && !checked.text.contains('|'));
    assert!((1..=draft::MAX_STEPS).contains(&checked.steps.len()));
    for step in &checked.steps {
        assert!(
            ["place", "npc", "kill", "item"].contains(&step.goal.as_str()),
            "{step:?}"
        );
        assert!(step.target.len() <= draft::MAX_TARGET_BYTES, "{step:?}");
    }
    let line = serde_json::to_string(&Output::DraftAnswer {
        id: MessageId(1),
        draft: Some(checked),
        notice: None,
    })
    .unwrap();
    assert!(fake_bridge::game_reply(&line).is_some(), "{line}");
}

/// A narrator line that passes keeps its limits, holds no slop and no bracket, names the
/// hero at most once, and names something of its moment.
fn assert_line(text: &str) {
    let moment = Moment::FirstKill {
        foe: "Hogger".to_string(),
    };
    let grounds = Grounds::of(&moment, Some("Hogger leads the gnolls of Elwynn Forest."));
    let Checked::Line(line) = checked_line(text, &grounds, "") else {
        return;
    };
    assert_voice(&line, narrator::MAX_LINE_CHARS, narrator::MAX_LINE_BYTES);
    assert!(slop_in(&line, "").is_empty(), "{line:?}");
    assert!(line.matches("$N").count() <= 1, "{line:?}");
    assert!(!line.contains(['[', ']', '{', '}', '<', '>']), "{line:?}");
    assert!(grounded(&line, &grounds), "{line:?}");
}

fuzz_target!(|data: &[u8]| {
    let moments = data.first().map_or(0, |byte| usize::from(byte % 9));
    let text = String::from_utf8_lossy(data);

    if let Some(saga) = chronicle::checked_saga(&text, moments, "", "") {
        assert_voice(
            &saga.text,
            chronicle::MAX_CHAPTER_CHARS,
            chronicle::MAX_CHAPTER_BYTES,
        );
        assert!(slop_in(&saga.text, "").is_empty(), "{saga:?}");
        assert!(saga.footnotes.len() <= chronicle::MAX_FOOTNOTES);
        for (moment, footnote) in &saga.footnotes {
            assert!((1..=moments).contains(moment));
            assert_voice(footnote, chronicle::MAX_FOOTNOTE_CHARS, 1600);
        }
    }
    if let Some(summary) = summary::checked_summary(&text, "", "") {
        assert_voice(
            &summary,
            summary::MAX_SUMMARY_CHARS,
            summary::MAX_SUMMARY_BYTES,
        );
        assert!(slop_in(&summary, "").is_empty(), "{summary:?}");
        assert!(summary.matches("$N").count() <= summary::MAX_NAMES, "{summary:?}");
        assert!(!summary.to_lowercase().contains("our hero"), "{summary:?}");
    }
    if let Some(answer) = talk::checked_answer(&text, "") {
        assert_voice(&answer.say, talk::MAX_SAY_CHARS, talk::MAX_SAY_BYTES);
        assert!((-talk::MAX_TRUST_CHANGE..=talk::MAX_TRUST_CHANGE).contains(&answer.trust_change));
        // Only the JSON `true` offers work, so a text with no "true" never does.
        if answer.work == talk::Work::Offered {
            assert!(text.contains("true"), "work with no true: {text:?}");
        }
    }
    if chronicle::checked_pick(&text) == chronicle::Pick::Second {
        assert!(text.contains('2'), "a pick of draft 2 with no 2: {text:?}");
    }
    assert_quest(&text);
    for word in main_words(&text) {
        assert!(word.chars().count() > 1, "{word:?}");
        assert_eq!(word, word.to_lowercase(), "{word:?}");
    }
    assert_draft(&text);
    assert_line(&text);
    if let Some(line) = plain_text(&text, 50, 200) {
        assert_plain(&line, 50, 200);
    }
    if let Ok(own) = hero::checked_text(&text, hero::LONG) {
        assert!(own.chars().count() <= hero::LONG.chars, "{own:?}");
        assert!(own.len() <= hero::LONG.bytes, "{own:?}");
        assert!(!own.chars().any(char::is_control), "{own:?}");
    }
    assert!(
        names_after_cutoff_except(&text, &text).is_empty(),
        "the player's own names: {text:?}"
    );
    let _ = check(&text, moments);
    let known: Vec<&str> = later_names().collect();
    assert!(
        names_after_cutoff(&text)
            .iter()
            .all(|name| known.contains(name))
    );
    common::pack().search(&text, 5).unwrap();
    let inside = without_fence_marks(&text);
    assert!(
        !inside.contains("<<<") && !inside.contains(">>>"),
        "{text:?}"
    );
});
