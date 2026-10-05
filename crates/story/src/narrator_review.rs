//! The narrator prompts of a world that exists, for a person to review (GAMEPLAY.md
//! 3.2.1). Each big moment of the history gets the prompt that the narrator gets at that
//! time: the world, the lore, and the text that the player had read so far.

use crate::character::Character;
use crate::learned::Read;
use crate::line_check::Grounds;
use crate::moments::{Moment, moments};
use crate::narrator::{self, Telling, Who};
use crate::narrator_lore::{LoreError, lore_of_moment};
use crate::pack::Pack;
use crate::seen::{SeenIndex, SeenText};
use hourglass::{Event, Tick};
use std::slice;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReviewError {
    #[error(transparent)]
    Lore(#[from] LoreError),
    #[error("seen text: {0}")]
    Seen(#[from] rusqlite::Error),
}

/// Where the lore of a moment comes from.
pub struct Sources<'a> {
    pub pack: &'a Pack,
    /// The texts that the player read, at any time.
    pub reads: &'a [Read],
    /// A race or a class for a world from before the addon sent them.
    pub fallback: &'a Who,
}

/// One big moment, its prompt, and what a line of it is checked against.
#[derive(Debug)]
pub struct Review {
    pub at: Tick,
    pub moment: Moment,
    pub grounds: Grounds,
    pub prompt: String,
}

/// Each event is a batch of its own, so a big moment never hides a smaller one.
///
/// # Errors
///
/// Returns the error of the pack or of the index of the read text.
pub fn reviews(events: &[Event], sources: &Sources<'_>) -> Result<Vec<Review>, ReviewError> {
    let mut reviews = Vec::new();
    for end in 1..=events.len() {
        let Some(character) = Character::from_history(&events[..end]) else {
            continue;
        };
        let event = &events[end - 1];
        for moment in moments(character.world(), character.you(), slice::from_ref(event)) {
            let review = review(&character, moment, event.tick, sources, reviews.len())?;
            reviews.push(review);
        }
    }
    Ok(reviews)
}

/// `turn` picks the samples and the naming, as the call number does in the game.
fn review(
    character: &Character,
    moment: Moment,
    at: Tick,
    sources: &Sources<'_>,
    turn: usize,
) -> Result<Review, ReviewError> {
    let read_then: Vec<SeenText> = sources
        .reads
        .iter()
        .filter(|read| read.at <= at)
        .map(|read| read.text.clone())
        .collect();
    let seen = SeenIndex::new(&read_then)?;
    let passage = lore_of_moment(sources.pack, &seen, character, &moment)?;
    let lore = passage.map(|passage| narrator::lore_excerpt(&passage.text));
    let who = with_fallback(Who::of(character), sources.fallback);
    let telling = Telling {
        moment: &moment,
        lore: lore.as_deref(),
        who: &who,
    };
    let prompt = narrator::prompt(&telling, turn);
    let grounds = Grounds::of(&telling, turn);
    Ok(Review {
        at,
        moment,
        grounds,
        prompt,
    })
}

fn with_fallback(who: Who, fallback: &Who) -> Who {
    Who {
        race: who.race.or(fallback.race),
        class: who.class.or(fallback.class),
        titles: who.titles,
    }
}
