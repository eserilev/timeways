//! "Help me write this" in the story program (GAMEPLAY.md 4.7): the idea of a player goes
//! to a model with what the world knows, and the checked draft goes back.

use super::{Active, Pending, Story, StoryError};
use crate::character::Character;
use crate::draft::{self, Draft, Known};
use crate::input::MessageId;
use crate::store::CharacterKey;
use crate::story::Output;

impl Story {
    /// The idea is the player's own words (5.11). The addon has taken out the names of the
    /// players who can get the task.
    pub(super) fn ask_draft(
        &mut self,
        id: MessageId,
        idea: &str,
    ) -> Result<Vec<Output>, StoryError> {
        let idea = draft::checked_idea(idea).ok_or(StoryError::BadWords)?;
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let prompt = draft::prompt(&known(active), idea);
        let pending = Pending::Draft {
            question: id,
            key: active.key.clone(),
        };
        Ok(self.open_call(pending, prompt).into_iter().collect())
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
            .and_then(|active| draft::checked_draft(text, &known(active)).ok());
        draft_answer(question, draft)
    }
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

fn foes(character: &Character) -> Vec<&str> {
    let mut foes = character.foes_seen();
    for foe in character.foes_defeated() {
        if !foes.contains(&foe) {
            foes.push(foe);
        }
    }
    foes
}
