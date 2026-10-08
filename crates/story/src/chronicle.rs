//! The narrator writes each finished chapter of the chronicle as a short saga (GAMEPLAY.md 3.3).
//! The words of its prompt live here, and the facts come from the chapter alone.

use crate::arrival::arrival_in;
use crate::check::{json_object, slop_in, voice_text};
use crate::grounding::ungrounded_names;
use crate::hero::OWN_WORDS;
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::journal::{Chapter, Deed, Place};
use crate::memory;
use crate::narrator::PERSONA;
use crate::places::PlaceKind;
use crate::prose::prose_faults;
use crate::samples::{self, Voice};
use serde::Deserialize;
use std::fmt::Write;

/// About 100 words. The prompt asks for 80.
pub const MAX_CHAPTER_CHARS: usize = 600;

/// The limit of the bridge for one string of the journal (Gnomish Relay SPEC.md 9.8).
pub const MAX_CHAPTER_BYTES: usize = 1600;

/// A footnote is one dry line of fact.
pub const MAX_FOOTNOTE_CHARS: usize = 200;

/// "It picks at most 3 footnotes for the chapter" (GAMEPLAY.md 5.4.1).
pub const MAX_FOOTNOTES: usize = 3;

/// The limit of the bridge for one string of the journal is 1600 bytes. A footnote keeps
/// far below it, so a chapter with 3 of them still fits on one page.
pub const MAX_FOOTNOTE_BYTES: usize = 600;

const FOOTNOTES: &str = "\
Pick at most 3 of the small moments for footnotes, or none. A footnote is one short, dry \
line of fact, for example: \"On the fourth day, $N danced in Goldshire, alone, at three in \
the morning.\"";

/// The author's note, with the format last.
const NOTE: &str = "\
Remember: one paragraph. Tie the deeds to the history of their places and peoples, and \
tell nothing of what comes next.
Write $N for the name of the hero, at most twice: the game puts the name there. Else say \
\"they\", or name no one.
Tell the facts as a story, not as a list. Few facts make a short chapter of two or three \
sentences.
Reply with JSON only: {\"saga\": \"<the chapter>\", \"footnotes\": [{\"moment\": <its number>, \
\"text\": \"<the footnote>\"}]}";

/// Which of the two drafts of a saga (GAMEPLAY.md 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draft {
    First,
    Second,
}

impl Draft {
    /// The second draft carries the next samples in turn, so the two drafts differ.
    fn sample_turn(self, number: usize) -> usize {
        match self {
            Draft::First => number,
            Draft::Second => number + Voice::Chapter.per_prompt(),
        }
    }
}

/// The draft that the judge picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    First,
    Second,
}

const JUDGE_TASK: &str = "Below are two drafts of one chapter of the chronicle. Pick the \
better one.";

/// The author's note of the judge, with the format last.
const JUDGE_NOTE: &str = "\
The better draft tells only the facts, names them plainly, and keeps your manner: serious, \
concrete, and sparing. A draft that adds a deed, a place, or a person is worse.
Reply with JSON only: {\"pick\": 1} or {\"pick\": 2}";

/// The saga of a chapter, and its footnotes with the number of the moment of each one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saga {
    pub text: String,
    pub footnotes: Vec<(usize, String)>,
}

#[derive(Deserialize)]
struct Reply {
    saga: String,
    #[serde(default)]
    footnotes: Vec<Footnote>,
}

#[derive(Deserialize)]
struct JudgeReply {
    pick: u8,
}

#[derive(Deserialize)]
struct Footnote {
    moment: usize,
    text: String,
}

/// `earlier` are the chapters just before this one, oldest first. `moments` are the small
/// moments of the chapter in plain words, best first. `told` is what the player wrote in
/// the chapter. `places` tell the kind of each zone: a dungeon, a raid, or a capital.
#[must_use]
pub fn prompt(
    places: &[Place],
    chapter: &Chapter,
    earlier: &[Chapter],
    moments: &[String],
    portrait: Option<&str>,
    told: &[&str],
) -> String {
    let own = OwnWords {
        portrait,
        told,
        telling: None,
    };
    draft_prompt(places, chapter, earlier, moments, &own, Draft::First)
}

/// What the player wrote: the portrait of the hero, the entries of the chapter, and the
/// player's own telling of the chapter (docs/plans/chapters.md 11).
#[derive(Clone, Copy, Debug, Default)]
pub struct OwnWords<'a> {
    pub portrait: Option<&'a str>,
    pub told: &'a [&'a str],
    pub telling: Option<&'a str>,
}

/// The prompt of one draft. Only the samples differ between the two drafts.
#[must_use]
pub fn draft_prompt(
    places: &[Place],
    chapter: &Chapter,
    earlier: &[Chapter],
    moments: &[String],
    own: &OwnWords<'_>,
    draft: Draft,
) -> String {
    let number = chapter.number;
    let mut prompt = format!(
        "{PERSONA}\n{HOUSE_RULES}\n\nWrite chapter {number} of the chronicle, in at most 80 \
         words, from the facts below and from nothing else."
    );
    prompt.push_str(&what_came_before(earlier));
    let _ = write!(
        prompt,
        "\n\nThe facts of chapter {number}:\n{}",
        fenced(&facts(places, chapter))
    );
    prompt.push_str(&small_moments(moments));
    prompt.push_str(&own_words(own));
    let samples = samples::section(Voice::Chapter, draft.sample_turn(number));
    let _ = write!(prompt, "\n\n{samples}\n\n{NOTE}");
    prompt
}

/// Chapter memory, so the saga knows the road so far. Empty for the first chapter.
fn what_came_before(earlier: &[Chapter]) -> String {
    if earlier.is_empty() {
        return String::new();
    }
    let summaries: Vec<String> = earlier.iter().map(memory::summary).collect();
    let summaries: Vec<&str> = summaries.iter().map(String::as_str).collect();
    format!(
        "\n\nWhat came before, as the chronicle holds it. Do not tell it again:\n{}",
        fenced(&bulleted(&summaries))
    )
}

fn small_moments(moments: &[String]) -> String {
    if moments.is_empty() {
        return String::new();
    }
    format!(
        "\n\nSmall moments:\n{}\n{FOOTNOTES}",
        fenced(&numbered(moments))
    )
}

fn own_words(own: &OwnWords<'_>) -> String {
    let mut words = String::new();
    if let Some(portrait) = own.portrait {
        let _ = write!(words, "\n\n{OWN_WORDS}\n{}", fenced(portrait));
    }
    if !own.told.is_empty() {
        let _ = write!(
            words,
            "\n\nWhat the player wrote in this chapter:\n{}",
            fenced(&bulleted(own.told))
        );
    }
    if let Some(telling) = own.telling {
        let _ = write!(words, "\n\n{TELLING}\n{}", fenced(telling));
    }
    words
}

/// The heading of the player's own telling of an entry. Its rule: the deeds come from the
/// facts, and the telling is neither repeated nor contradicted.
pub const TELLING: &str = "The player's telling, not canon. Tell the deeds of the facts, do not \
repeat the telling, and do not contradict it:";

/// The prompt of the judge: the facts of chapter `number`, and the saga text of each draft.
#[must_use]
pub fn judge_prompt(number: usize, facts: &str, first: &str, second: &str) -> String {
    format!(
        "{PERSONA}\n{HOUSE_RULES}\n\n{JUDGE_TASK}\n\nThe facts of chapter {number}:\n{}\n\n\
         Draft 1:\n{}\n\nDraft 2:\n{}\n\n{JUDGE_NOTE}",
        fenced(facts),
        fenced(first),
        fenced(second)
    )
}

/// A bad answer picks the first draft: both drafts passed every check.
#[must_use]
pub fn checked_pick(text: &str) -> Pick {
    let reply = json_object(text).and_then(|json| serde_json::from_str::<JudgeReply>(json).ok());
    match reply {
        Some(JudgeReply { pick: 2 }) => Pick::Second,
        _ => Pick::First,
    }
}

/// The facts of a chapter, one on each line, as its prompts show them.
#[must_use]
pub fn facts(places: &[Place], chapter: &Chapter) -> String {
    let mut facts = Vec::new();
    if !chapter.zones.is_empty() {
        let zones: Vec<String> = chapter
            .zones
            .iter()
            .map(|zone| described(places, zone))
            .collect();
        facts.push(format!("- Traveled to: {}.", zones.join(", ")));
    }
    if !chapter.people.is_empty() {
        facts.push(format!("- Met: {}.", chapter.people.join(", ")));
    }
    for deed in &chapter.deeds {
        facts.push(format!("- {}.", deed_fact(deed)));
    }
    facts.join("\n")
}

fn numbered(moments: &[String]) -> String {
    let lines: Vec<String> = moments
        .iter()
        .enumerate()
        .map(|(index, moment)| format!("{}. {moment}", index + 1))
        .collect();
    lines.join("\n")
}

/// The saga as the player reads it, or None when it breaks a rule. A footnote that breaks
/// a rule, or names no moment of the list, is dropped alone. A chapter that fails keeps
/// its plain list; its second draft is its second chance (GAMEPLAY.md 3.3). `facts` are
/// the facts of the chapter: a slop word that they hold, such as a name, stays allowed.
/// `player_text` is the hero in the player's own words. A footnote is one short line of a
/// small moment, so the checks of the style guide (`prose_faults`) hold for the saga text
/// alone.
#[must_use]
pub fn checked_saga(
    text: &str,
    moment_count: usize,
    facts: &str,
    player_text: &str,
    given: &str,
) -> Option<Saga> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let saga = voice_text(
        &reply.saga,
        MAX_CHAPTER_CHARS,
        MAX_CHAPTER_BYTES,
        player_text,
    );
    let given = format!("{given}\n{player_text}");
    let saga =
        saga.filter(|saga| is_clean(saga, facts, &given) && prose_faults(saga, &[]).is_empty())?;
    let mut footnotes: Vec<(usize, String)> = Vec::new();
    for footnote in reply.footnotes {
        let known = (1..=moment_count).contains(&footnote.moment);
        let new = footnotes
            .iter()
            .all(|(moment, _)| *moment != footnote.moment);
        let text = voice_text(
            &footnote.text,
            MAX_FOOTNOTE_CHARS,
            MAX_FOOTNOTE_BYTES,
            player_text,
        )
        .filter(|text| is_clean(text, facts, &given));
        if let (true, true, Some(text)) = (known, new, text) {
            footnotes.push((footnote.moment, text));
        }
    }
    footnotes.truncate(MAX_FOOTNOTES);
    Some(Saga {
        text: saga,
        footnotes,
    })
}

/// No slop, no clause that only tells that the hero came, and no invented name.
fn is_clean(text: &str, facts: &str, given: &str) -> bool {
    slop_in(text, facts).is_empty()
        && arrival_in(text, &[]).is_none()
        && ungrounded_names(text, given).is_empty()
}

fn described(places: &[Place], zone: &str) -> String {
    let kind = places
        .iter()
        .find(|place| place.name == zone && place.within.is_none())
        .map_or(PlaceKind::Zone, |place| place.kind);
    kind.described(zone)
}

/// A deed as one plain fact, as a prompt tells it.
#[must_use]
pub fn deed_fact(deed: &Deed) -> String {
    match deed {
        Deed::Level { from: None, to, .. } => format!("Began the saga at level {to}"),
        Deed::Level { to, .. } => format!("Reached level {to}"),
        Deed::Defeated { foe, times: 1, .. } => format!("Defeated {foe}, a first kill"),
        Deed::Defeated { foe, times, .. } => format!("Defeated {foe} again, {times} times in all"),
        Deed::Titled { title, .. } => format!("Earned the title \"{title}\""),
        Deed::QuestDone { title, .. } | Deed::GameQuestDone { title, .. } => {
            format!("Finished the quest \"{title}\"")
        }
        Deed::ClassQuestDone { title, .. } => format!("Finished the class quest \"{title}\""),
        Deed::QuestMarked { mark, quest, .. } => {
            format!("Gained the lasting effect \"{mark}\" during the quest \"{quest}\"")
        }
        Deed::Mounted {
            mount, epic: false, ..
        } => format!("Rode a first mount, {mount}"),
        Deed::Mounted {
            mount, epic: true, ..
        } => format!("Rode a first swift mount, {mount}"),
        Deed::EpicItem { item, .. } => format!("Put on {item}, a first item of the finest kind"),
        Deed::Upgraded { item, .. } => {
            format!("Put on {item}, far better than the item it replaced")
        }
        Deed::WonBattle { battleground, .. } => {
            format!("Won a battle in {battleground}, a first win")
        }
        Deed::PvpRank { rank, .. } => format!("Reached PvP rank {rank}"),
        Deed::Died {
            killer: Some(killer),
            ..
        } => format!("Fell to {killer}"),
        Deed::Died { killer: None, .. } => "Died".to_string(),
    }
}
