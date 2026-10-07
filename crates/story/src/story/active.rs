//! The character of the last `character_entered`: its world, its logs, and what one line
//! from the bridge adds to its database (docs/plans/links.md).

use super::calls::NARRATOR;
use crate::chapters::Book;
use crate::character::Character;
use crate::entry_edits::EntryEdit;
use crate::input::Input;
use crate::journal::{Journal, TalkQuest, journal_of};
use crate::narrator;
use crate::past::PastRow;
use crate::ratings::{RatedLine, ShownLine};
use crate::seen::SeenIndex;
use crate::store::{
    AliasLog, CallEnd, CharacterKey, Database, FlavorLog, HeroLog, LearnedLog, Line, NewCall,
    NewInput, NewRow, Next, Node, Origin, Outcome, Prose, QuestLog, Root, RowLog, StoreError,
    StoryLog, SummaryLog, Table, TaleText, ZoneHistory,
};
use crate::walk::RuleRow;
use timeways_rules::narrator_shapes::WINDOW;

/// A line from the bridge as the database keeps it, after the clock check.
pub(super) struct Kept {
    kind: String,
    root: Root,
    body: String,
    /// Every batch starts with `character_entered`, so only the one that founds the world
    /// is kept.
    only_to_found: bool,
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
            only_to_found: matches!(input, Input::CharacterEntered { .. }),
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
    pub(super) stories: StoryLog,
    pub(super) aliases: AliasLog,
    pub(super) summaries: SummaryLog,
    /// The rule epochs of the chapters (docs/plans/chapters.md 13).
    pub(super) rules: RowLog<RuleRow>,
    /// The texts of the tales (docs/plans/chapters.md 6).
    pub(super) tales: RowLog<TaleText>,
    /// "Your history here" of each zone (docs/plans/chapters.md 10).
    pub(super) zone_histories: RowLog<ZoneHistory>,
    /// The player's edits of the chapters, the tales, and the summary.
    pub(super) entry_edits: RowLog<EntryEdit>,
    /// The player's ratings of narrator text (GAMEPLAY.md 3.2.2).
    pub(super) ratings: RowLog<RatedLine>,
    /// The past of the character before Timeways: only the first row is kept.
    pub(super) past: RowLog<PastRow>,
    /// Why the last edit of an entry did not stand, until a journal page shows it.
    pub(super) edit_refused: Option<String>,
    /// The chapters and the tales, folded one event at a time.
    pub(super) book: Book,
    pub(super) seen_index: SeenIndex,
    /// Why the last edit of the hero did not stand, until a journal page shows it.
    pub(super) hero_refused: Option<String>,
    /// The quest that the newest talk with work asked for (GAMEPLAY.md 3.5). It lives in
    /// memory only: the talk window waits a few minutes at most.
    pub(super) talk_quest: Option<Box<TalkQuest>>,
    /// The newest narrator line of this run, for a rating of it. It lives in memory only:
    /// a rating comes right after the line.
    pub(super) shown_line: Option<ShownLine>,
}

impl Active {
    /// Writes everything of the line in one transaction: the input, the calls, and the
    /// rows, each row with the line or the call that made it.
    pub(super) fn save(&mut self, kept: Option<Kept>) -> Result<(), StoreError> {
        let founds = self.saved_events == 0;
        let kept = kept.filter(|kept| !kept.only_to_found || founds);
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
            (Table::Stories, self.stories.take_unsaved()),
            (Table::Aliases, self.aliases.take_unsaved()),
            (Table::Summaries, self.summaries.take_unsaved()),
            (Table::ChapterRules, self.rules.take_unsaved()),
            (Table::Tales, self.tales.take_unsaved()),
            (Table::ZoneHistories, self.zone_histories.take_unsaved()),
            (Table::EntryEdits, self.entry_edits.take_unsaved()),
            (Table::Ratings, self.ratings.take_unsaved()),
            (Table::Past, self.past.take_unsaved()),
        ])
    }

    /// Folds the events that came since the last call into the book.
    pub(super) fn catch_up(&mut self) {
        self.book.catch_up(self.character.world().history());
    }

    /// The journal with the chapters and the tales of the book.
    pub(super) fn journal(&self) -> Journal {
        journal_of(&self.character, &self.book)
    }

    /// The call gets its row now, with what it read, so a row that changes while the model
    /// thinks still counts as read. Returns the position of the row.
    pub(super) fn open_call_row(
        &mut self,
        kind: &'static str,
        pack: String,
        prompt: String,
        reads: Vec<Node>,
    ) -> u64 {
        let position = self.next.call;
        self.next.call += 1;
        self.new_calls.push(NewCall {
            position,
            kind,
            pack,
            prompt,
            reads,
        });
        position
    }

    /// The calls of these kinds: the saved ones, and the ones that this line opened.
    pub(super) fn count_calls(&self, kinds: &[&str]) -> Result<u64, StoreError> {
        let saved = self.database.count_calls(kinds)?;
        let opened = self
            .new_calls
            .iter()
            .filter(|call| kinds.contains(&call.kind));
        Ok(saved + u64::try_from(opened.count()).unwrap_or_default())
    }

    pub(super) fn end_call_row(
        &mut self,
        position: u64,
        answer: Option<&str>,
        outcome: Outcome,
        shape: Option<String>,
    ) {
        self.ended.push(CallEnd {
            position,
            answer: answer.map(str::to_string),
            outcome,
            shape,
        });
    }

    /// The lore that the narrator calls told, fenced: accepted, or still open. The calls
    /// that this line opened count too, before the save.
    pub(super) fn told_lore(&self) -> Result<Vec<String>, StoreError> {
        let mut told = self.database.told_lore()?;
        let opened = self
            .new_calls
            .iter()
            .filter(|call| call.kind == NARRATOR)
            .filter_map(|call| narrator::told_lore(&call.prompt))
            .map(str::to_string);
        told.extend(opened);
        Ok(told)
    }

    /// The main parts of the last accepted narrator lines, oldest first: the window of the
    /// rotation (docs/plans/narrator-templates.md 3.5). A line that is not saved yet counts.
    pub(super) fn recent_shapes(&self) -> Result<Vec<String>, StoreError> {
        let pending = self
            .ended
            .iter()
            .rev()
            .filter(|end| end.outcome == Outcome::Accepted)
            .filter_map(|end| end.shape.clone());
        let mut newest: Vec<String> = pending.collect();
        newest.extend(self.database.newest_shapes(WINDOW)?);
        newest.truncate(WINDOW);
        newest.reverse();
        Ok(newest)
    }
}
