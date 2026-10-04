//! The words of the quest prompt. The lists hold only the targets that the check allows,
//! so the model has nothing else to pick (GAMEPLAY.md 3.4).

use super::{Known, MAX_KILLS, MAX_STEPS, MAX_TITLE_CHARS};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::talk::persona;

/// The names of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>) -> String {
    format!(
        "{}\n{HOUSE_RULES}\n\nGive the player a small task of your own: a rumor, a favor, or \
         an errand.\n\nPlaces that the player can visit:\n{}\n\n\
         People that the player can meet:\n{}\n\n\
         Creatures that the player can hunt:\n{}\n\n\
         Rules:\n\
         - 1 to {MAX_STEPS} steps. A step is {{\"goal\": \"visit\", \"place\": \"<a place \
         above>\"}}, {{\"goal\": \"meet\", \"npc\": \"<a person above>\"}}, or \
         {{\"goal\": \"kill\", \"creature\": \"<a creature above>\", \"count\": <1 to \
         {MAX_KILLS}>}}.\n\
         - Copy each name exactly as the list writes it. Use no other place, person, or \
         creature. An empty list has nothing to use.\n\
         - Each step names a different place, person, or creature.\n\
         - The task is not a quest of the game, and it does not continue one.\n\
         - The title has at most {MAX_TITLE_CHARS} characters. The text has at most 60 \
         words, in your own voice.\n\n\
         Reply with JSON only: {{\"title\": \"...\", \"text\": \"...\", \"steps\": [...]}}",
        persona(known.giver, place),
        list(&known.places()),
        list(&known.people()),
        list(&known.prey())
    )
}

fn list(names: &[&str]) -> String {
    fenced(&bulleted(&names[..names.len().min(PROMPT_NAMES)]))
}
