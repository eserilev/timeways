//! The lore of a narrator moment: one passage about its place, foe, or person
//! (GAMEPLAY.md 3.2). It passes the spoiler limit as a `/lore` passage does, and the text
//! that the player read comes first.

use crate::character::Character;
use crate::check::mentions;
use crate::pack::{Link, Pack, PackError, Passage};
use crate::passage_limits;
use crate::seen::SeenIndex;
use thiserror::Error;

/// Enough candidates that the spoiler limit still leaves one about the subject.
const CANDIDATES: u32 = 20;

#[derive(Debug, Error)]
pub enum LoreError {
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("seen text: {0}")]
    Seen(#[from] rusqlite::Error),
}

/// A passage linked to `subject` comes first, then one that names it. A search for "The
/// Deadmines" also finds every passage with "the", so a passage about something else
/// never counts.
///
/// # Errors
///
/// Returns the error of the pack or of the index of seen text.
pub fn lore_about(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    subject: &str,
) -> Result<Option<Passage>, LoreError> {
    let mut found = seen.search(subject, CANDIDATES)?;
    found.extend(pack.search(subject, CANDIDATES)?);
    let known: Vec<Passage> = found
        .into_iter()
        .filter(|passage| character.knows_all(&passage.links))
        .filter_map(passage_limits::fitted)
        .collect();
    let linked = known.iter().find(|passage| is_linked(passage, subject));
    let named = || {
        known
            .iter()
            .find(|passage| mentions(&passage.text, subject))
    };
    Ok(linked.or_else(named).cloned())
}

fn is_linked(passage: &Passage, subject: &str) -> bool {
    passage.links.iter().any(|link| match link {
        Link::Place(name) | Link::Npc(name) => name == subject,
        Link::Common => false,
    })
}
