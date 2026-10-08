//! Random model answers and random player text. Every text that passes a check keeps its
//! limits, names nothing that its prompt did not give, and the search never fails on
//! syntax.

#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use timeways_story::npc_voice::Asked;
use timeways_story::arrival::arrival_in;
use timeways_story::check::{
    check, in_voice, later_names, names_after_cutoff, names_after_cutoff_except, plain_text,
    same_words, slop_in,
};
use timeways_story::draft;
use timeways_story::grounding::{given_text, ungrounded_names};
use timeways_story::lore::{LoreCall, Next};
use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::prompt::Context;
use timeways_story::house::without_fence_marks;
use timeways_story::input::MessageId;
use timeways_story::line_check::{Checked, Grounds, MOST_SENTENCES, checked_line, grounded};
use timeways_story::moments::Moment;
use timeways_story::narrator::{Telling, Who};
use timeways_story::narrator_build::{Answered, Setup, answered, offer};
use timeways_story::narrator_slots::parse;
use timeways_story::places::InstanceKind;
use timeways_story::present_check::{
    PresentFault, PresentGrounds, has_present_sentence, unsourced_present_in,
};
use timeways_story::prose::prose_faults;
use timeways_story::quest::variety::{Recent, Shape, main_words};
use timeways_story::quest::{self, Genre, Known, Step};
use timeways_story::race_class::{Class, Race};
use timeways_story::seen::{SeenText, TextKind};
use timeways_story::sentences::sentences;
use timeways_story::story::Output;
use timeways_story::{chronicle, hero, narrator, prologue, summary, tale, talk, zone_history};

/// What the prompt of each kind of text gave the model, in these runs.
const GIVEN: &str = "Hogger leads the gnolls of Elwynn Forest. Edwin VanCleef holds the Deadmines \
    for the Defias Brotherhood in Westfall.";

/// An accepted text names only what its prompt gave (GAMEPLAY.md 3.2.1).
fn assert_grounded(text: &str, given: &str) {
    assert_eq!(ungrounded_names(text, given), Vec::<String>::new(), "{text:?}");
}

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

/// A tale or a zone history keeps the saga check (docs/plans/chapters.md 6 and 10).
fn assert_entry_text(text: &str, max_chars: usize, max_bytes: usize) {
    assert_voice(text, max_chars, max_bytes);
    assert!(slop_in(text, "").is_empty(), "{text:?}");
    assert_eq!(arrival_in(text, &[]), None, "{text:?}");
    assert!(text.matches("$N").count() <= 2, "{text:?}");
    assert!(!text.to_lowercase().contains("our hero"), "{text:?}");
    assert_eq!(prose_faults(text, &[]), [], "{text:?}");
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

/// A narrator line that passes keeps its limits, holds no slop, no bracket, no arrival of
/// the hero, and no fault of the style guide, names the hero at most once, and names
/// something of its moment. A line of a place never names the hero.
fn assert_line(text: &str) {
    let orc = Who {
        race: Some(Race::Orc),
        ..Who::default()
    };
    let hogger = Moment::FirstKill {
        foe: "Hogger".to_string(),
        zone: None,
        creature: None,
    };
    let elwynn = Moment::NewZone {
        zone: "Elwynn Forest".to_string(),
    };
    for moment in [&hogger, &elwynn] {
        let telling = Telling {
            moment,
            lore: Some("Hogger leads the gnolls of Elwynn Forest."),
            who: &orc,
        };
        let grounds = Grounds::of(&telling, 0);
        let Checked::Line(line) = checked_line(text, &grounds, "") else {
            continue;
        };
        assert_voice(&line, narrator::MAX_LINE_CHARS, narrator::MAX_LINE_BYTES);
        assert!(slop_in(&line, "").is_empty(), "{line:?}");
        assert!(line.matches("$N").count() <= 1, "{line:?}");
        assert!(!line.contains(['[', ']', '{', '}', '<', '>']), "{line:?}");
        assert!(grounded(&line, &grounds), "{line:?}");
        assert_grounded(&line, &grounds.given());
        assert_eq!(arrival_in(&line, &grounds.hero_words), None, "{line:?}");
        assert!(!moment.is_arrival() || !line.contains("$N"), "{line:?}");
        assert_eq!(prose_faults(&line, &grounds.hero_words), [], "{line:?}");
        assert!(sentences(&line).len() <= MOST_SENTENCES, "{line:?}");
    }
}

/// A `/lore` answer that shows names only what its passages and its question gave.
fn assert_lore(text: &str) {
    let passage = Passage {
        text: GIVEN.to_string(),
        source: "the wiki page \"Elwynn Forest\"".to_string(),
        links: vec![Link::Place("Elwynn Forest".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: Vec::new(),
        setup_for: None,
    };
    let call = LoreCall::new("Who leads the gnolls?", &Context::default(), vec![passage]);
    let given = given_text(call.prompt());
    if let Next::Done(answer) = call.answered(text)
        && let Some(shown) = answer.text
    {
        assert_grounded(&shown, &given);
    }
}

/// The moments of the slot JSON, each with its lore and its hero.
fn slot_moments() -> Vec<(Moment, Who, &'static str)> {
    let paladin = Who {
        race: Some(Race::Human),
        class: Some(Class::Paladin),
        titles: vec!["Bookworm".to_string()],
    };
    let hogger = Moment::FirstKill {
        foe: "Hogger".to_string(),
        zone: Some("Elwynn Forest".to_string()),
        creature: None,
    };
    let level = Moment::LevelUp {
        level: 20,
        zone: None,
    };
    let elwynn = Moment::NewZone {
        zone: "Elwynn Forest".to_string(),
    };
    let murloc = Moment::SlainAgain {
        killer: "Murloc Coastrunner".to_string(),
        times: 3,
        zone: Some("Westfall".to_string()),
    };
    let ram = Moment::FirstMount {
        mount: "Gray Ram".to_string(),
        people: Some("Ironforge".to_string()),
    };
    vec![
        (
            hogger,
            paladin.clone(),
            "Hogger leads the Riverpaw gnolls of Elwynn Forest.",
        ),
        (
            level,
            paladin.clone(),
            "The Silver Hand guards Stormwind with the Light.",
        ),
        (
            elwynn,
            paladin.clone(),
            "Northshire Abbey stands in Elwynn Forest.",
        ),
        (
            murloc,
            paladin.clone(),
            "Murlocs raid the coast of Westfall.",
        ),
        (ram, paladin, "The Mountaineers of Ironforge ride rams."),
    ]
}

/// A slot answer never panics the parser. A line that the code builds from it keeps the
/// limits of a line, holds no slop and no fault of the style guide, names the hero at most
/// once, and never names the hero at a place (docs/plans/narrator-templates.md 6.2).
fn assert_slots(text: &str) {
    for (moment, who, lore) in slot_moments() {
        let setup = Setup {
            moment,
            who,
            turn: text.len(),
            recent: vec!["k.fell".to_string(), "v.stronger".to_string()],
            setup_foe: None,
        };
        let offer = offer(&setup).unwrap();
        let _ = parse(text, offer.kind, &offer.group_ids());
        let telling = Telling {
            moment: &setup.moment,
            lore: Some(lore),
            who: &setup.who,
        };
        let grounds = Grounds::of(&telling, setup.turn);
        let Answered::Line(built) = answered(text, &setup, &offer, &grounds, "") else {
            continue;
        };
        assert_voice(
            &built.line,
            narrator::MAX_LINE_CHARS,
            narrator::MAX_LINE_BYTES,
        );
        assert!(slop_in(&built.line, "").is_empty(), "{built:?}");
        assert!(built.line.matches("$N").count() <= 1, "{built:?}");
        assert!(
            !built.line.contains(['[', ']', '{', '}', '<', '>']),
            "{built:?}"
        );
        assert_eq!(
            arrival_in(&built.line, &grounds.hero_words),
            None,
            "{built:?}"
        );
        assert!(
            !setup.moment.is_arrival() || !built.line.contains("$N"),
            "{built:?}"
        );
        assert!(!built.shape.is_empty(), "{built:?}");
    }
}

/// The present check never panics, refuses only clauses of the history, and refuses every
/// present clause that names a defeated foe (docs/plans/lore-names-and-now.md 2.3 C).
fn assert_present(text: &str) {
    let _ = has_present_sentence(text);
    let names = vec!["The Deadmines".to_string()];
    let defeated = vec!["Edwin VanCleef".to_string()];
    for lore in ["Edwin VanCleef leads the Defias Brotherhood.", text, ""] {
        let grounds = PresentGrounds {
            lore,
            names: &names,
            defeated: &defeated,
        };
        for fault in unsourced_present_in(text, &grounds) {
            let (PresentFault::Unsourced(clause) | PresentFault::Defeated(clause, _)) = fault;
            assert!(text.contains(clause.as_str()), "{clause:?} in {text:?}");
        }
    }
}

/// A setup line that the code builds ends on its coda, whatever the history says
/// (docs/plans/lore-names-and-now.md 2.3 A).
fn assert_setup(text: &str) {
    let setup = Setup {
        moment: Moment::FirstInstance {
            zone: "The Deadmines".to_string(),
            kind: InstanceKind::Dungeon,
        },
        who: Who::default(),
        turn: text.len(),
        recent: Vec::new(),
        setup_foe: Some("Edwin VanCleef".to_string()),
    };
    let lore = "Gryan Stoutmantle sent adventurers into the Deadmines to kill Edwin VanCleef.";
    let offer = offer(&setup).unwrap();
    let telling = Telling {
        moment: &setup.moment,
        lore: Some(lore),
        who: &setup.who,
    };
    let grounds = Grounds::of(&telling, setup.turn);
    if let Answered::Line(built) = answered(text, &setup, &offer, &grounds, "") {
        assert!(
            built.line.ends_with("Edwin VanCleef is still alive."),
            "{built:?}"
        );
        assert!(!built.line.contains("$N"), "{built:?}");
    }
}

fuzz_target!(|data: &[u8]| {
    let moments = data.first().map_or(0, |byte| usize::from(byte % 9));
    let text = String::from_utf8_lossy(data);
    assert_present(&text);
    assert_setup(&text);

    if let Some(saga) = chronicle::checked_saga(&text, moments, "", "", GIVEN) {
        assert_voice(
            &saga.text,
            chronicle::MAX_CHAPTER_CHARS,
            chronicle::MAX_CHAPTER_BYTES,
        );
        assert!(slop_in(&saga.text, "").is_empty(), "{saga:?}");
        assert_grounded(&saga.text, GIVEN);
        assert_eq!(arrival_in(&saga.text, &[]), None, "{saga:?}");
        assert_eq!(prose_faults(&saga.text, &[]), [], "{saga:?}");
        assert!(saga.footnotes.len() <= chronicle::MAX_FOOTNOTES);
        for (moment, footnote) in &saga.footnotes {
            assert!((1..=moments).contains(moment));
            assert_grounded(footnote, GIVEN);
            assert_voice(footnote, chronicle::MAX_FOOTNOTE_CHARS, 1600);
        }
    }
    if let Some(summary) = summary::checked_summary(&text, "", "", GIVEN) {
        assert_voice(
            &summary,
            summary::MAX_SUMMARY_CHARS,
            summary::MAX_SUMMARY_BYTES,
        );
        assert!(slop_in(&summary, "").is_empty(), "{summary:?}");
        assert_grounded(&summary, GIVEN);
        assert_eq!(arrival_in(&summary, &[]), None, "{summary:?}");
        assert_eq!(prose_faults(&summary, &[]), [], "{summary:?}");
        assert!(
            summary.matches("$N").count() <= summary::MAX_NAMES,
            "{summary:?}"
        );
        assert!(!summary.to_lowercase().contains("our hero"), "{summary:?}");
    }
    if let Some(text) = prologue::checked_prologue(&text, "Westfall", "", GIVEN) {
        assert_grounded(&text, GIVEN);
        assert_entry_text(
            &text,
            prologue::MAX_PROLOGUE_CHARS,
            prologue::MAX_PROLOGUE_BYTES,
        );
    }
    if let Some(tale) = tale::checked_tale(&text, "", "", &[], GIVEN) {
        assert_entry_text(&tale, tale::MAX_TALE_CHARS, tale::MAX_TALE_BYTES);
        assert_grounded(&tale, GIVEN);
    }
    if let Ok(history) = zone_history::checked_history(&text, "", "", &[], GIVEN) {
        assert_entry_text(
            &history,
            zone_history::MAX_HISTORY_CHARS,
            zone_history::MAX_HISTORY_BYTES,
        );
        assert_grounded(&history, GIVEN);
    }
    if let Some(answer) = talk::checked_answer(&text, Asked::NoQuestion, "", GIVEN) {
        assert_voice(&answer.say, talk::MAX_SAY_CHARS, talk::MAX_SAY_BYTES);
        assert_grounded(&answer.say, GIVEN);
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
    assert_slots(&text);
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
    assert_lore(&text);
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
