//! The character of the last `character_entered`: its world, its logs, and what one line
//! from the bridge adds to its database (docs/plans/links.md).

use crate::character::Character;
use crate::input::{CallId, Input};
use crate::seen::SeenIndex;
use crate::store::{
    CallEnd, CharacterKey, Database, FlavorLog, HeroLog, LearnedLog, Line, NewCall, NewInput,
    NewRow, Next, Node, Origin, Outcome, Prose, QuestLog, Root, StoreError, Table,
};
use std::collections::BTreeMap;

/// A line from the bridge as the database keeps it, after the clock check.
pub(super) struct Kept {
    kind: String,
    root: Root,
    body: String,
}

impl Kept {
    /// None for a line that leaves no input (`Input::is_kept`).
    pub(super) fn of(input: &Input) -> Result<Option<Kept>, StoreError> {
        if !input.is_kept() {
            return Ok(None);
        }
        let value = serde_json::to_value(input)?;
        let kind = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(Some(Kept {
            kind,
            root: input.root(),
            body: value.to_string(),
        }))
    }
}

pub(super) struct Active {
    pub(super) key: CharacterKey,
    pub(super) character: Character,
    pub(super) database: Database,
    /// The events that the database holds. The next save starts after them.
    pub(super) saved_events: usize,
    pub(super) next: Next,
    /// The row in `calls` of each open call of this character. The `CallId` of the bridge
    /// starts again at 1 in each run, so it never names a row.
    pub(super) call_rows: BTreeMap<CallId, u64>,
    /// The calls that the line opened.
    pub(super) new_calls: Vec<NewCall>,
    /// The calls that the line ended.
    pub(super) ended: Vec<CallEnd>,
    /// The row of the call whose answer the line carries. It made the rows of the line.
    pub(super) answering: Option<u64>,
    pub(super) prose: Prose,
    pub(super) flavor: FlavorLog,
    pub(super) hero: HeroLog,
    pub(super) learned: LearnedLog,
    pub(super) quests: QuestLog,
    pub(super) seen_index: SeenIndex,
    /// Why the last edit of the hero did not stand, until a journal page shows it.
    pub(super) hero_refused: Option<String>,
}

impl Active {
    /// Writes everything of the line in one transaction: the input, the calls, and the
    /// rows, each row with the line or the call that made it.
    pub(super) fn save(&mut self, kept: Option<Kept>) -> Result<(), StoreError> {
        // Every batch starts with `character_entered`, so only the one that founds the
        // world is kept.
        let founds = self.saved_events == 0;
        let kept = kept.filter(|kept| kept.kind != "character_entered" || founds);
        let input = kept.map(|kept| NewInput {
            position: self.next.input,
            kind: kept.kind,
            root: kept.root,
            body: kept.body,
        });
        let origin = match (self.answering.take(), &input) {
            (Some(call), _) => Some(Origin::Call(call)),
            (None, Some(input)) => Some(Origin::Input(input.position)),
            (None, None) => None,
        };
        let line = Line {
            origin,
            calls: std::mem::take(&mut self.new_calls),
            ended: std::mem::take(&mut self.ended),
            rows: self.take_rows()?,
            input,
        };
        self.database.save(&line)?;
        if line.input.is_some() {
            self.next.input += 1;
        }
        self.saved_events = self.character.world().history().len();
        Ok(())
    }

    fn take_rows(&mut self) -> Result<Vec<(Table, Vec<NewRow>)>, StoreError> {
        let events = self
            .character
            .world()
            .history()
            .iter()
            .skip(self.saved_events)
            .map(|event| {
                let body = serde_json::to_string(event)?;
                Ok(NewRow {
                    position: event.id.0,
                    body,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(vec![
            (Table::Events, events),
            (Table::Chapters, self.prose.take_unsaved()),
            (Table::Flavor, self.flavor.take_unsaved()),
            (Table::Hero, self.hero.take_unsaved()),
            (Table::Learned, self.learned.take_unsaved()),
            (Table::Quests, self.quests.take_unsaved()),
        ])
    }

    /// The call gets its row now, with what it read, so a row that changes while the model
    /// thinks still counts as read.
    pub(super) fn open_call_row(
        &mut self,
        call: CallId,
        kind: &'static str,
        pack: String,
        prompt: String,
        reads: Vec<Node>,
    ) {
        let position = self.next.call;
        self.next.call += 1;
        self.call_rows.insert(call, position);
        self.new_calls.push(NewCall {
            position,
            kind,
            pack,
            prompt,
            reads,
        });
    }

    /// The row of an open call, for a call that reads an earlier one.
    pub(super) fn call_row(&self, call: CallId) -> Option<u64> {
        self.call_rows.get(&call).copied()
    }

    /// The rows of the line rest on this call. A call of another character has no row
    /// here, so the rows of its line rest on nothing.
    pub(super) fn answer_with(&mut self, call: CallId) {
        self.answering = self.call_row(call);
    }

    /// A call of another character has no row here, and ends nothing.
    pub(super) fn end_call_row(&mut self, call: CallId, answer: Option<&str>, outcome: Outcome) {
        let Some(position) = self.call_rows.remove(&call) else {
            return;
        };
        self.ended.push(CallEnd {
            position,
            answer: answer.map(str::to_string),
            outcome,
        });
    }
}
