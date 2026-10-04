//! The words of the quest prompt. The lists hold only the targets that the check allows,
//! so the model has nothing else to pick (GAMEPLAY.md 3.4).

use super::emotes::quest_emotes;
use super::variety::{Recent, recent_line};
use super::{Known, MAX_CARRY, MAX_KILLS, MAX_STEPS, MAX_TITLE_CHARS, MAX_WAIT_DAYS};
use crate::hero_hook::{Hook, QUEST_RULE, hook_block};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::talk::persona;

/// The names of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

const VISIT: &str = r#"{"goal": "visit", "place": "<a place above>"}: go there."#;
const VISIT_AT: &str = r#"{"goal": "visit_at", "place": "<a place above>", "time": "<dawn, noon, dusk, or night>"}: be there at that time of day, in the player's own time."#;
const MEET: &str = r#"{"goal": "meet", "npc": "<a person above>"}: speak with them."#;
const SLAP: &str = r#"{"goal": "slap", "npc": "<a person above>"}: slap them. Only in a comic quest, and never you."#;
const TALK: &str = r#"{"goal": "talk", "npc": "<a person above>", "about": "<a topic in a few words>"}: ask them about something with /talk. The topic is optional."#;

#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>, hook: Option<Hook<'_>>) -> String {
    let places = known.places();
    let people = known.people();
    let prey = known.prey();
    let goods = &known.goods;
    let hook = hook.map_or_else(String::new, |hook| {
        format!("{}\n\n", hook_block(&hook, QUEST_RULE))
    });
    format!(
        "{}\n{HOUSE_RULES}\n\nGive the player a small task of your own: a rumor, a favor, or \
         an errand.\n\nPlaces that the player can visit:\n{}\n\n\
         People that the player can meet:\n{}\n\n\
         Creatures that the player can hunt:\n{}\n\n\
         Goods that the player can bring:\n{}\n\n\
         {hook}{}Rules:\n\
         - 1 to {MAX_STEPS} steps. Each step is one of these goals:\n{}\n\
         - Copy each name exactly as the list writes it. Use no other place, person, or \
         creature. An empty list has nothing to use.\n\
         - Each step names a different place, person, or creature.\n\
         - Only a step after a wait can send the player back to you.\n\
         - To let the player do 2 or 3 steps in any order, put them in one entry of the \
         steps: {{\"goal\": \"any_order\", \"steps\": [...]}}. At most one such entry, and \
         no wait in it.\n\
         - Give the quest a genre: \"errand\" (a favor or a delivery), \"hunt\" (a creature \
         or a boss), \"mystery\" (a question with clues), \"rescue\" (a person who is lost or \
         in trouble), \"rivalry\" (a feud between two people), or \"comic\" (a joke).\n\
         - Start the text with why you need this: who you are, and what it means for you{}.\n\
         - The task is not a quest of the game, and it does not continue one.\n\
         - The title has at most {MAX_TITLE_CHARS} characters. The text has at most 60 \
         words, in your own voice.\n\n\
         Reply with JSON only: {{\"title\": \"...\", \"genre\": \"...\", \"text\": \"...\", \
         \"steps\": [...]}}",
        persona(known.giver, place),
        list(&places),
        list(&people),
        list(&prey),
        list(goods),
        recent_block(&known.recent),
        goals(&places, &people, &prey, goods),
        place.map_or_else(String::new, |place| format!(" in {place}"))
    )
}

fn list(names: &[&str]) -> String {
    fenced(&bulleted(&names[..names.len().min(PROMPT_NAMES)]))
}

/// A goal shows only when its list has a name, so the model never picks a goal that the
/// check refuses.
fn goals(places: &[&str], people: &[&str], prey: &[&str], goods: &[&str]) -> String {
    let goals = [
        (!places.is_empty(), VISIT.to_string()),
        (!places.is_empty(), VISIT_AT.to_string()),
        (!people.is_empty(), MEET.to_string()),
        (!people.is_empty(), TALK.to_string()),
        (!places.is_empty() || !people.is_empty(), emote_goal()),
        (!people.is_empty(), SLAP.to_string()),
        (!prey.is_empty(), kill_goal()),
        (!goods.is_empty() && !people.is_empty(), carry_goal()),
        (true, wait_goal()),
    ];
    let lines: Vec<String> = goals
        .iter()
        .filter(|(shown, _)| *shown)
        .map(|(_, goal)| format!("  - {goal}"))
        .collect();
    lines.join("\n")
}

fn kill_goal() -> String {
    format!(
        r#"{{"goal": "kill", "creature": "<a creature above>", "count": <1 to {MAX_KILLS}>}}: hunt them."#
    )
}

fn carry_goal() -> String {
    format!(
        r#"{{"goal": "carry", "item": "<a good above>", "count": <1 to {MAX_CARRY}>, "npc": "<a person above>"}}: have the goods in the bags when meeting the person. The player keeps them."#
    )
}

fn emote_goal() -> String {
    format!(
        r#"{{"goal": "emote", "emote": "<one of: {}>", "npc": "<a person above>"}}: use the emote on the person. Or give "place": "<a place above>" in place of "npc": use it in the place."#,
        quest_emotes().join(", ")
    )
}

fn wait_goal() -> String {
    format!(
        r#"{{"goal": "wait", "days": <1 to {MAX_WAIT_DAYS}>}}: the player comes back later. Never the first or the last step, and at most one."#
    )
}

/// The newest offers, so the model makes a different one. The titles are model text, so
/// they go in a fence. With no earlier quest, the block is left out.
fn recent_block(recent: &[Recent]) -> String {
    if recent.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = recent.iter().map(recent_line).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    format!(
        "The player's last quests, newest first:\n{}\nMake this quest different from these: \
         other kinds of steps or another order, another genre, and no word of their titles.\n\n",
        fenced(&bulleted(&lines))
    )
}
