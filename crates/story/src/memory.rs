//! Chapter memory: one short line about each closed chapter, built from its facts
//! (GAMEPLAY.md 3.3). The model never writes it, so it never drifts from the history.

use crate::journal::{Chapter, Deed};

/// The chapters before a chapter that its prompt recalls.
pub const MEMORY_CHAPTERS: usize = 3;

/// The zones and foes of one summary. More costs tokens and adds little.
const NAMES_PER_LIST: usize = 3;

#[must_use]
pub fn summary(chapter: &Chapter) -> String {
    let mut parts = Vec::new();
    if !chapter.zones.is_empty() {
        parts.push(format!("traveled to {}", first_names(&chapter.zones)));
    }
    if !chapter.people.is_empty() {
        parts.push(format!("met {}", first_names(&chapter.people)));
    }
    if let Some(level) = highest_level(&chapter.deeds) {
        parts.push(format!("reached level {level}"));
    }
    let foes = first_kills(&chapter.deeds);
    if !foes.is_empty() {
        parts.push(format!("defeated {}", foes.join(", ")));
    }
    for title in finished_tasks(&chapter.deeds) {
        parts.push(format!("finished the quest \"{title}\""));
    }
    parts.extend(deaths(&chapter.deeds));
    if parts.is_empty() {
        return format!("Chapter {}: nothing of note.", chapter.number);
    }
    format!("Chapter {}: {}.", chapter.number, parts.join("; "))
}

fn first_names(names: &[String]) -> String {
    names[..names.len().min(NAMES_PER_LIST)].join(", ")
}

fn highest_level(deeds: &[Deed]) -> Option<i64> {
    deeds
        .iter()
        .filter_map(|deed| match deed {
            Deed::Level { to, .. } => Some(*to),
            _ => None,
        })
        .max()
}

fn first_kills(deeds: &[Deed]) -> Vec<&str> {
    let foes = deeds.iter().filter_map(|deed| match deed {
        Deed::Defeated { foe, times: 1, .. } => Some(foe.as_str()),
        _ => None,
    });
    foes.take(NAMES_PER_LIST).collect()
}

fn finished_tasks(deeds: &[Deed]) -> Vec<&str> {
    deeds
        .iter()
        .filter_map(|deed| match deed {
            Deed::QuestDone { title, .. } => Some(title.as_str()),
            _ => None,
        })
        .collect()
}

fn deaths(deeds: &[Deed]) -> Option<String> {
    let count = deeds
        .iter()
        .filter(|deed| matches!(deed, Deed::Died { .. }))
        .count();
    match count {
        0 => None,
        1 => Some("died once".to_string()),
        _ => Some(format!("died {count} times")),
    }
}
