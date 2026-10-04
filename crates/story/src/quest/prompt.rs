//! The words of the quest prompt. The lists hold only the targets that the check allows,
//! so the model has nothing else to pick (GAMEPLAY.md 3.4).

use super::{Known, MAX_KILLS, MAX_STEPS, MAX_TITLE_CHARS, MAX_WAIT_DAYS};
use crate::hero_hook::{Hook, QUEST_RULE, hook_block};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::talk::persona;

/// The names of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

const VISIT: &str = r#"{"goal": "visit", "place": "<a place above>"}: go there."#;
const MEET: &str = r#"{"goal": "meet", "npc": "<a person above>"}: speak with them."#;
const TALK: &str = r#"{"goal": "talk", "npc": "<a person above>", "about": "<a topic in a few words>"}: ask them about something with /talk. The topic is optional."#;

#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>, hook: Option<Hook<'_>>) -> String {
    let places = known.places();
    let people = known.people();
    let prey = known.prey();
    let hook = hook.map_or_else(String::new, |hook| {
        format!("{}\n\n", hook_block(&hook, QUEST_RULE))
    });
    format!(
        "{}\n{HOUSE_RULES}\n\nGive the player a small task of your own: a rumor, a favor, or \
         an errand.\n\nPlaces that the player can visit:\n{}\n\n\
         People that the player can meet:\n{}\n\n\
         Creatures that the player can hunt:\n{}\n\n\
         {hook}Rules:\n\
         - 1 to {MAX_STEPS} steps. Each step is one of these goals:\n{}\n\
         - Copy each name exactly as the list writes it. Use no other place, person, or \
         creature. An empty list has nothing to use.\n\
         - Each step names a different place, person, or creature.\n\
         - Only a step after a wait can send the player back to you.\n\
         - To let the player do 2 or 3 steps in any order, put them in one entry of the \
         steps: {{\"goal\": \"any_order\", \"steps\": [...]}}. At most one such entry, and \
         no wait in it.\n\
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
    let wait = format!(
        r#"{{"goal": "wait", "days": <1 to {MAX_WAIT_DAYS}>}}: the player comes back later. Never the first or the last step, and at most one."#
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
    goals.push(&wait);
    let lines: Vec<String> = goals.iter().map(|goal| format!("  - {goal}")).collect();
    lines.join("\n")
}
