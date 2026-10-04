//! The words of the quest prompt. The lists hold only the targets that the check allows,
//! so the model has nothing else to pick (GAMEPLAY.md 3.4).

use super::{Known, MAX_KILLS, MAX_STEPS, MAX_TITLE_CHARS};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::talk::persona;

/// The names of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

const VISIT: &str = r#"{"goal": "visit", "place": "<a place above>"}: go there."#;
const MEET: &str = r#"{"goal": "meet", "npc": "<a person above>"}: speak with them."#;
const TALK: &str = r#"{"goal": "talk", "npc": "<a person above>", "about": "<a topic in a few words>"}: ask them about something with /talk. The topic is optional."#;

#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>) -> String {
    let places = known.places();
    let people = known.people();
    let prey = known.prey();
    format!(
        "{}\n{HOUSE_RULES}\n\nGive the player a small task of your own: a rumor, a favor, or \
         an errand.\n\nPlaces that the player can visit:\n{}\n\n\
         People that the player can meet:\n{}\n\n\
         Creatures that the player can hunt:\n{}\n\n\
         Rules:\n\
         - 1 to {MAX_STEPS} steps. Each step is one of these goals:\n{}\n\
         - Copy each name exactly as the list writes it. Use no other place, person, or \
         creature. An empty list has nothing to use.\n\
         - Each step names a different place, person, or creature.\n\
         - The task is not a quest of the game, and it does not continue one.\n\
         - The title has at most {MAX_TITLE_CHARS} characters. The text has at most 60 \
         words, in your own voice.\n\n\
         Reply with JSON only: {{\"title\": \"...\", \"text\": \"...\", \"steps\": [...]}}",
        persona(known.giver, place),
        list(&places),
        list(&people),
        list(&prey),
        goals(&places, &people, &prey)
    )
}

fn list(names: &[&str]) -> String {
    fenced(&bulleted(&names[..names.len().min(PROMPT_NAMES)]))
}

/// A goal shows only when its list has a name, so the model never picks a goal that the
/// check refuses.
fn goals(places: &[&str], people: &[&str], prey: &[&str]) -> String {
    let kill = format!(
        r#"{{"goal": "kill", "creature": "<a creature above>", "count": <1 to {MAX_KILLS}>}}: hunt them."#
    );
    let mut goals = Vec::new();
    if !places.is_empty() {
        goals.push(VISIT);
    }
    if !people.is_empty() {
        goals.extend([MEET, TALK]);
    }
    if !prey.is_empty() {
        goals.push(&kill);
    }
    let lines: Vec<String> = goals.iter().map(|goal| format!("  - {goal}")).collect();
    lines.join("\n")
}
