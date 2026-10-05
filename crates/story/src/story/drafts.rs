//! "Help me write this" in the story program (GAMEPLAY.md 4.7): the idea of a player goes
//! to a model with what the world knows, and the checked draft goes back.

use super::{Active, Pending, Story, StoryError, aliases, reads};
use crate::aliases::{ids_in, knows_every_id, unmarked, with_names};
use crate::character::Character;
use crate::draft::{self, Draft, Known, MAX_TEXT_BYTES, MAX_TITLE_BYTES};
use crate::input::MessageId;
use crate::store::CharacterKey;
use crate::story::Output;

impl Story {
    /// The idea is the player's own words. Each player that it names reaches the model as
    /// an ID with a card (5.11).
    pub(super) fn ask_draft(
        &mut self,
        id: MessageId,
        idea: &str,
    ) -> Result<Vec<Output>, StoryError> {
        let marked = unmarked(idea);
        let idea = draft::checked_idea(&marked.text).ok_or(StoryError::BadWords)?;
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let idea = aliases::without_names(active, &marked.names, idea)?;
        let cards = active.aliases.cards(&idea);
        let known = known(active);
        let prompt = draft::prompt(&known, &idea, &cards);
        let reads = reads::events_about(active, known_names(&known));
        let pending = Pending::Draft {
            question: id,
            key: active.key.clone(),
        };
        Ok(self.open_call(pending, prompt, reads).into_iter().collect())
    }

    /// A draft for another character, or one that breaks a rule, comes back as no draft.
    pub(super) fn draft_answered(
        &mut self,
        question: MessageId,
        key: &CharacterKey,
        text: &str,
    ) -> Output {
        let draft = self
            .active
            .as_ref()
            .filter(|active| &active.key == key)
            .and_then(|active| {
                let draft = draft::checked_draft(text, &known(active)).ok()?;
                with_player_names(active, draft)
            });
        draft_answer(question, draft)
    }
}

/// The draft with the name of each player in place of its ID. A model cannot invent a
/// player, the names must fit the limits of the addon, and a step names no player.
fn with_player_names(active: &Active, draft: Draft) -> Option<Draft> {
    let table = active.aliases.table();
    let known_ids = knows_every_id(table, &draft.title) && knows_every_id(table, &draft.text);
    let plain_steps = draft
        .steps
        .iter()
        .all(|step| ids_in(&step.target).is_empty());
    if !known_ids || !plain_steps {
        return None;
    }
    let title = with_names(table, &draft.title);
    let text = with_names(table, &draft.text);
    let fits = title.len() <= MAX_TITLE_BYTES && text.len() <= MAX_TEXT_BYTES;
    fits.then_some(Draft {
        title,
        text,
        steps: draft.steps,
    })
}

pub(super) fn draft_answer(id: MessageId, draft: Option<Draft>) -> Output {
    Output::DraftAnswer {
        id,
        draft,
        notice: None,
    }
}

fn known(active: &Active) -> Known<'_> {
    let character = &active.character;
    Known {
        zones: character.visited_zones(),
        subzones: character.visited_subzones(),
        npcs: character.npcs_to_meet(),
        foes: foes(character),
    }
}

fn known_names<'a>(known: &Known<'a>) -> Vec<&'a str> {
    let mut names = known.zones.clone();
    names.extend(&known.subzones);
    names.extend(&known.npcs);
    names.extend(&known.foes);
    names
}

fn foes(character: &Character) -> Vec<&str> {
    let mut foes = character.foes_seen();
    for foe in character.foes_defeated() {
        if !foes.contains(&foe) {
            foes.push(foe);
        }
    }
    foes
}
