//! The narrator prompts of a world that exists, for a person to review (GAMEPLAY.md
//! 3.2.1). Each big moment of the history gets the prompt that the narrator gets at that
//! time: the world, the lore, and the text that the player had read so far.

use crate::character::Character;
use crate::learned::Read;
use crate::line_check::Grounds;
use crate::moments::{Creature, SlotKind};
use crate::moments::{Moment, moments};
use crate::narrator::{self, Telling, Who};
use crate::narrator_build::{Offer, Setup, every_line, kind_of, offer, setup_foe};
use crate::narrator_lore::{LoreError, is_silent, lore_of_moment, lore_subjects};
use crate::narrator_slots::{ChoiceField, Choices, KillerKind, Tone, fields_of};
use crate::narrator_templates::Number;
use crate::pack::Pack;
use crate::places::InstanceKind;
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
    /// Why the moment gets no call, as the story program decides it.
    pub silence: Option<String>,
    /// What the templates build the line from. None for a flavor moment. The tool sets
    /// `recent` of the setup from the lines that it showed before.
    pub templated: Option<(Setup, Offer)>,
}

/// Each event is a batch of its own, so a big moment never hides a smaller one.
///
/// # Errors
///
/// Returns the error of the pack or of the index of the read text.
pub fn reviews(events: &[Event], sources: &Sources<'_>) -> Result<Vec<Review>, ReviewError> {
    let mut reviews = Vec::new();
    // The tool shows every prompt, so it counts the lore of each one as told.
    let mut told: Vec<String> = Vec::new();
    for end in 1..=events.len() {
        let Some(character) = Character::from_history(&events[..end]) else {
            continue;
        };
        let event = &events[end - 1];
        for moment in moments(character.world(), character.you(), slice::from_ref(event)) {
            let review = review(
                &character,
                moment,
                event.tick,
                sources,
                reviews.len(),
                &told,
            )?;
            if review.silence.is_none()
                && let Some(lore) = narrator::told_lore(&review.prompt)
            {
                told.push(lore.to_string());
            }
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
    told: &[String],
) -> Result<Review, ReviewError> {
    let read_then: Vec<SeenText> = sources
        .reads
        .iter()
        .filter(|read| read.at <= at)
        .map(|read| read.text.clone())
        .collect();
    let seen = SeenIndex::new(&read_then)?;
    let who = with_fallback(Who::of(character), sources.fallback);
    let passage = lore_of_moment(sources.pack, &seen, character, &moment, &who, told)?;
    let subjects = lore_subjects(&moment, &who);
    let mut silence =
        is_silent(&moment, &subjects, passage.as_ref()).then(|| thin_reason(&subjects));
    let foe = setup_foe(&moment, passage.as_ref());
    let lore = passage.map(|passage| narrator::lore_excerpt(&passage.text));
    let telling = Telling {
        moment: &moment,
        lore: lore.as_deref(),
        who: &who,
    };
    let grounds = Grounds::of(&telling, turn);
    let setup = Setup {
        moment: moment.clone(),
        who: who.clone(),
        turn,
        recent: Vec::new(),
        setup_foe: foe,
    };
    let templated = match (kind_of(&moment), offer(&setup)) {
        (None, _) => None,
        (Some(_), Some(offer)) => Some((setup, offer)),
        (Some(_), None) => {
            silence.get_or_insert_with(|| "no template fits the moment".to_string());
            None
        }
    };
    let prompt = match &templated {
        Some((_, offer)) => narrator::lore_prompt(&telling, turn, offer),
        None => narrator::line_prompt(&telling, turn),
    };
    Ok(Review {
        at,
        moment,
        grounds,
        prompt,
        silence,
        templated,
    })
}

fn thin_reason(subjects: &[String]) -> String {
    if subjects.is_empty() {
        return "the moment has no subject that lore can tell of".to_string();
    }
    format!("no passage about {}", subjects.join(" or "))
}

fn with_fallback(who: Who, fallback: &Who) -> Who {
    Who {
        race: who.race.or(fallback.race),
        class: who.class.or(fallback.class),
        titles: who.titles,
    }
}

/// The fixed moments of the review of the templates: one of each kind with templates.
fn fixed_moments() -> Vec<Moment> {
    let zone = Some("Westfall".to_string());
    vec![
        Moment::FirstKill {
            foe: "Edwin VanCleef".to_string(),
            zone: zone.clone(),
            creature: Some(Creature::Beast),
        },
        Moment::Revenge {
            foe: "Gath'Ilzogg".to_string(),
            deaths: 2,
            zone: zone.clone(),
        },
        Moment::SlainAgain {
            killer: "Defias Pillager".to_string(),
            times: 3,
            zone: zone.clone(),
        },
        Moment::QuestDone {
            title: "The Lost Plow".to_string(),
            giver: Some("Farmer Saldean".to_string()),
        },
        Moment::ClassQuestDone {
            title: "The Tome of Divinity".to_string(),
        },
        Moment::Titled {
            title: "Bookworm".to_string(),
        },
        Moment::Slapped {
            npc: "Innkeeper Heather".to_string(),
            times: 2,
            zone: zone.clone(),
        },
        Moment::QuestMarked {
            mark: "Rediscovered Light".to_string(),
            quest: "The Tome of Divinity".to_string(),
        },
        Moment::FirstMount {
            mount: "Gray Ram".to_string(),
            people: Some("Ironforge".to_string()),
        },
        Moment::FirstEpicMount {
            mount: "Swift Brown Steed".to_string(),
            people: None,
        },
        Moment::BigUpgrade {
            item: "Cruel Barb".to_string(),
            zone,
            slot: Some(SlotKind::Weapon),
        },
        Moment::LevelUp {
            level: 30,
            zone: None,
        },
    ]
}

/// A setup for each fixed moment, and the first entry of a dungeon whose lore sets up the
/// defeat of its boss.
fn fixed_setups(who: &Who) -> Vec<Setup> {
    let setup = |moment, setup_foe| Setup {
        moment,
        who: who.clone(),
        turn: 0,
        recent: Vec::new(),
        setup_foe,
    };
    let mut setups: Vec<Setup> = fixed_moments()
        .into_iter()
        .map(|moment| setup(moment, None))
        .collect();
    let entry = Moment::FirstInstance {
        zone: "The Deadmines".to_string(),
        kind: InstanceKind::Dungeon,
    };
    setups.push(setup(entry, Some("Edwin VanCleef".to_string())));
    setups
}

/// Every line that the templates can build for a pairing, with fixed values, so a person
/// reviews the templates in one list (docs/plans/narrator-templates.md 4). Each line names
/// its parts.
#[must_use]
pub fn template_lines(who: &Who) -> Vec<String> {
    let mut printed = Vec::new();
    for setup in fixed_setups(who) {
        printed.push(format!("=== {}", narrator::what_happened(&setup.moment)));
        let Some(offer) = offer(&setup) else {
            printed.push("(no shape fits)".to_string());
            continue;
        };
        let groups = if offer.groups.is_empty() {
            vec![None]
        } else {
            offer.groups.iter().map(Some).collect()
        };
        for group in groups {
            let anchor = group.and_then(|group| group.anchors.first().cloned());
            let lore = format!(
                "{} and the Defias hold Westfall and its rams now.",
                anchor.as_deref().unwrap_or("The Brotherhood")
            );
            let choices = Choices {
                group: group.map(|group| group.id.clone()),
                there: Some(true),
                leads: Some("the Defias".to_string()),
                leads_number: Some(Number::Many),
                tone: Some(Tone::Dry),
                killer: Some(KillerKind::Kind),
                breed: Some(true),
            };
            let offered = fields_of(offer.kind);
            let choices = only_offered(choices, offered);
            match every_line(&setup, &offer, &lore, &choices) {
                Ok(lines) => printed.extend(
                    lines
                        .into_iter()
                        .map(|built| format!("{}  [{}]", built.line, built.parts.join(" + "))),
                ),
                Err(faults) => printed.push(format!("(refused: {faults:?})")),
            }
        }
    }
    printed
}

fn only_offered(choices: Choices, offered: &[ChoiceField]) -> Choices {
    let has = |field: ChoiceField| offered.contains(&field);
    Choices {
        group: choices.group.filter(|_| has(ChoiceField::Group)),
        there: choices.there.filter(|_| has(ChoiceField::There)),
        leads: choices.leads.filter(|_| has(ChoiceField::Leads)),
        leads_number: choices
            .leads_number
            .filter(|_| has(ChoiceField::LeadsNumber)),
        tone: choices.tone.filter(|_| has(ChoiceField::Tone)),
        killer: choices.killer.filter(|_| has(ChoiceField::Killer)),
        breed: choices.breed.filter(|_| has(ChoiceField::Breed)),
    }
}
