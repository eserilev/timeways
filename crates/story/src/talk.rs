//! Talk to an NPC (GAMEPLAY.md 3.5). The model plays the NPC and proposes a change of its
//! trust. The code checks both before anything shows or lands in the world (5.2).

use crate::check::{json_object, voice_text};
use crate::grounding::ungrounded_names;
use crate::hero_hook::{Hook, TALK_RULE, hook_block};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::npc_voice::{Asked, npc_faults};
use crate::pack::Passage;
use crate::samples::{self, Voice};
use crate::tokens::{Call, largest_fit};
use serde::Deserialize;
use std::fmt::Write;

/// About 70 words. The prompt asks for 60.
pub const MAX_SAY_CHARS: usize = 400;

/// The limit of the bridge for the words of a talk (Gnomish Relay SPEC.md 9.8).
pub const MAX_SAY_BYTES: usize = 1600;

/// The limit of the bridge for the NPC of a talk (Gnomish Relay SPEC.md 9.8).
pub const MAX_NPC_BYTES: usize = 64;

/// The largest change of trust that one talk proposes.
pub const MAX_TRUST_CHANGE: i64 = 5;

/// What the NPC knows about the player and its place.
#[derive(Debug, Default)]
pub struct Scene<'a> {
    pub npc: &'a str,
    pub place: Option<&'a str>,
    pub level: Option<i64>,
    pub trust: Option<i64>,
    pub slapped: Option<i64>,
    /// The entries of the player's own lore about this NPC or its place.
    pub own_lore: Vec<&'a str>,
    /// What the NPC remembers of the player, worded, newest first (`npc_memory`).
    pub memories: Vec<String>,
    /// The quests of other NPCs that send the player to talk to this one.
    pub quests: Vec<QuestTalk<'a>>,
    /// One answer of the hero sheet, one talk in three (`hero_hook`).
    pub hook: Option<Hook<'a>>,
}

/// A quest whose talk step sends the player to this NPC (docs/plans/quest-variety.md 4.2).
#[derive(Debug)]
pub struct QuestTalk<'a> {
    pub giver: &'a str,
    pub title: &'a str,
    pub about: Option<&'a str>,
}

/// The short persona of an NPC, from the facts alone. An NPC never gets the persona of the
/// narrator (GAMEPLAY.md 3.2.1). The name and the place come from the addon, so they are
/// data.
#[must_use]
pub fn persona(npc: &str, place: Option<&str>) -> String {
    let mut who = format!("Name: {npc}");
    if let Some(place) = place {
        let _ = write!(who, "\nPlace: {place}");
    }
    format!(
        "You are a person of the world of Warcraft, with this name and place:\n{}\nSpeak \
         as this person would: plainly, in your own voice, and only of what a person of your \
         place knows.",
        fenced(&who)
    )
}

/// `turn` picks the golden samples of the prompt. The passages come most relevant first,
/// so the last ones go first when the prompt is over its budget.
#[must_use]
pub fn prompt(scene: &Scene<'_>, passages: &[Passage], words: &str, turn: usize) -> String {
    largest_fit(passages.len(), Call::Talk.prompt_budget(), |kept| {
        prompt_with(scene, &passages[..kept], words, turn)
    })
}

fn prompt_with(scene: &Scene<'_>, passages: &[Passage], words: &str, turn: usize) -> String {
    format!(
        "{}\n{HOUSE_RULES}\n\nAnswer the player, and say how this talk changes your trust. \
         Stay true to the lore below. When you do not know, say so as this person would.{}\n\n\
         {}\n\n{MANNER}\n\nThe player says:\n{}\n\n\
         {WORK_RULE}\n\n\
         Remember: you are the person of the name above. Answer first, in 1 to 4 short \
         sentences and at most 40 words.\n\
         Reply with JSON only: {{\"say\": \"<your answer>\", \"trust\": <a whole number from \
         -{MAX_TRUST_CHANGE} to {MAX_TRUST_CHANGE}: how this talk changes your trust in the \
         player>, \"work\": <true when you offer the player work, else false>}}",
        who_you_are(scene),
        what_you_know(scene, passages),
        samples::section(Voice::NpcReply, turn),
        fenced(words)
    )
}

/// The compact voice guide of an NPC (docs/plans/npc-voice.md 7). `npc_voice.rs` refuses
/// what it forbids.
const MANNER: &str = "Your manner:\n\
     - Talk like a person of your people and trade: everyday words, short sentences.\n\
     - Answer the player first. Then one thing you saw, want, or fear. One ask at most.\n\
     - Be concrete: who, where, what happened, with the names of the lore. No speeches.\n\
     - Show your people in what you care about, not in spelling.\n\
     - Never open with \"Ah\". Never call the player \"adventurer\" or \"brave soul\". \
     Never say how the player feels.\n\
     - No \"dark times\", \"trouble lingers\", \"I've taken to\", or old words like \"thee\".\n\
     - When the player asked something, end on your answer, not a question.";

/// Work that the NPC mentions becomes a real quest, written and checked after the talk
/// (GAMEPLAY.md 3.5). So the NPC names only the trouble, never a detail that the quest
/// can lack.
const WORK_RULE: &str = "When you offer the player work, or the player asks for work and you \
     have some, set \"work\" to true. The work becomes a real quest that the player reads \
     next. So say only what troubles you, and name no place, creature, count, or reward.";

fn who_you_are(scene: &Scene<'_>) -> String {
    let mut who = persona(scene.npc, scene.place);
    let _ = write!(who, " A player speaks to you. {}", trust_words(scene.trust));
    if let Some(slapped) = scene.slapped {
        let _ = write!(
            who,
            " The player slapped you {slapped} times, and you remember each one."
        );
    }
    who
}

/// The trust of the NPC in words, never as a number, with the bands of the People page.
fn trust_words(trust: Option<i64>) -> &'static str {
    match trust {
        None => "You do not know the player yet.",
        Some(50..) => "You trust the player.",
        Some(10..) => "You like the player.",
        Some(-9..) => "You have no strong feeling about the player.",
        Some(-49..) => "You are wary of the player.",
        Some(_) => "You distrust the player.",
    }
}

fn what_you_know(scene: &Scene<'_>, passages: &[Passage]) -> String {
    let mut known = String::new();
    if let Some(level) = scene.level {
        let _ = write!(known, "\n\nThe player is level {level}.");
    }
    if !passages.is_empty() {
        let texts: Vec<&str> = passages
            .iter()
            .map(|passage| passage.text.as_str())
            .collect();
        let _ = write!(
            known,
            "\n\nLore that you know:\n{}",
            fenced(&bulleted(&texts))
        );
    }
    if !scene.own_lore.is_empty() {
        let _ = write!(
            known,
            "\n\nWhat the player told of their own story, about you or this place. It is their \
             story, not canon:\n{}",
            fenced(&bulleted(&scene.own_lore))
        );
    }
    if let Some(hook) = &scene.hook {
        let _ = write!(known, "\n\n{}", hook_block(hook, TALK_RULE));
    }
    known.push_str(&what_you_remember(&scene.memories));
    for quest in &scene.quests {
        let _ = write!(known, "\n\n{}", quest_line(quest));
    }
    known
}

/// The rule against a made-up past stays with no memories, because the slaps and the
/// trust above are a past too.
fn what_you_remember(memories: &[String]) -> String {
    const NO_MADE_UP_PAST: &str = "Never speak of a past with the player that is not written here.";
    if memories.is_empty() {
        return format!("\n\n{NO_MADE_UP_PAST}");
    }
    let lines: Vec<&str> = memories.iter().map(String::as_str).collect();
    format!(
        "\n\nWhat you remember of the player, newest first. Each line is true:\n{}\n\
         Bring up at most one of these, and only when it fits what the player says. \
         {NO_MADE_UP_PAST}",
        fenced(&bulleted(&lines))
    )
}

/// The title and the topic are model text that passed a check, so they are data.
fn quest_line(quest: &QuestTalk<'_>) -> String {
    let mut data = format!("Giver: {}\nQuest: {}", quest.giver, quest.title);
    if let Some(about) = quest.about {
        let _ = write!(data, "\nTopic: {about}");
    }
    format!(
        "The player comes to you for a quest of another person:\n{}\nPlay along with it. Say \
         what you know, and make nothing up about the giver.",
        fenced(&data)
    )
}

#[derive(Deserialize)]
struct Reply {
    say: String,
    trust: i64,
    /// Only a JSON `true` offers work. A missing field, or any other value, offers none,
    /// and the words still show.
    #[serde(default)]
    work: serde_json::Value,
}

/// Whether the NPC offers the player work. Work becomes a real quest (GAMEPLAY.md 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    Offered,
    NotOffered,
}

/// The words of the NPC, the change of trust that passed the check, and the work.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub say: String,
    pub trust_change: i64,
    pub work: Work,
}

/// None when the words break a rule. A change of trust outside the band is dropped, and
/// the words still show. `asked` tells whether the player asked something, and
/// `player_text` is the hero in the player's own words.
#[must_use]
pub fn checked_answer(text: &str, asked: Asked, player_text: &str, given: &str) -> Option<Answer> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let say = voice_text(&reply.say, MAX_SAY_CHARS, MAX_SAY_BYTES, player_text)?;
    let told = format!("{given}\n{player_text}");
    if !ungrounded_names(&say, &told).is_empty() || !npc_faults(&say, asked, &told).is_empty() {
        return None;
    }
    let in_band = (-MAX_TRUST_CHANGE..=MAX_TRUST_CHANGE).contains(&reply.trust);
    let work = if reply.work == serde_json::Value::Bool(true) {
        Work::Offered
    } else {
        Work::NotOffered
    };
    Some(Answer {
        say,
        trust_change: if in_band { reply.trust } else { 0 },
        work,
    })
}
