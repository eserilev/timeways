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

/// The own page of `subject` comes first, in page order, so the history of the Deadmines
/// wins over the page of a boss inside it. Then a passage linked to `subject`, then one
/// that names it. A search for "The Deadmines" also finds every passage with "the", so a
/// passage about something else never counts.
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
    let known = |passages: Vec<Passage>| -> Vec<Passage> {
        passages
            .into_iter()
            .filter(|passage| character.knows_all(&passage.links))
            .filter_map(passage_limits::fitted)
            .collect()
    };
    if let Some(own) = known(pack.about(subject, CANDIDATES)?).into_iter().next() {
        return Ok(Some(own));
    }
    let mut found = seen.search(subject, CANDIDATES)?;
    found.extend(pack.search(subject, CANDIDATES)?);
    let found = known(found);
    let linked = found.iter().find(|passage| is_linked(passage, subject));
    let named = || {
        found
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
