//! Plays a scenario through the real story program, behind the real checks of the bridge,
//! as the game would send it (TESTING.md, "Dev mode").

use crate::scenario::{Batch, Scenario};
use fake_bridge::{FakeBridge, Reply};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::time::Duration;
use timeways_story::story::Story;

/// The words of a real model for a prompt, or None for a failed call.
pub type Model = Box<dyn FnMut(&str) -> Option<String>>;

/// A slow model never makes the story program drop a narrator line here: nobody waits.
pub const NO_DEADLINE: Duration = Duration::from_hours(24);

/// What the play showed. A clean scenario has no refused line, no dropped line, and no
/// fixed answer left over.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub batches: usize,
    /// Each line that the story program refused, with the reason.
    pub refused: Vec<String>,
    /// Lines that the checks of the bridge dropped.
    pub dropped: usize,
    /// Fixed answers that no call of their kind took.
    pub unused_answers: Vec<String>,
    /// The batches that got an error reply.
    pub failed_batches: Vec<String>,
    pub narrator: Vec<String>,
    pub notices: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.refused.is_empty()
            && self.dropped == 0
            && self.unused_answers.is_empty()
            && self.failed_batches.is_empty()
    }
}

type Answers = Rc<RefCell<BTreeMap<String, VecDeque<String>>>>;

/// Plays every batch of `scenario` for the character into `story`. A fixed answer of the
/// scenario goes first, then `model`. With no model, a call fails, as with no model in the
/// bridge. The story ends with the play, so its world is on the disk after.
#[must_use]
pub fn play(
    scenario: &Scenario,
    realm: &str,
    name: &str,
    mut story: Story,
    model: Option<Model>,
) -> Report {
    story.set_events_deadline(NO_DEADLINE);
    let answers: Answers = Rc::default();
    let mut bridge = FakeBridge::new(story).with_model_by_kind(model_with(&answers, model));
    let character = json!({"type": "character_entered", "realm": realm, "name": name});
    let mut report = Report::default();
    for batch in &scenario.batches {
        for answer in &batch.answers {
            let mut queues = answers.borrow_mut();
            let queue = queues.entry(answer.kind.clone()).or_default();
            queue.push_back(answer.text.clone());
        }
        let text = batch_text(&character, batch);
        let errors_before = bridge.errors().len();
        let reply = bridge.batch(&text);
        report.batches += 1;
        report
            .refused
            .extend(bridge.errors()[errors_before..].iter().cloned());
        read_reply(&reply, &mut report);
    }
    report.dropped = bridge.dropped_lines();
    for (kind, queue) in answers.borrow().iter() {
        report
            .unused_answers
            .extend(queue.iter().map(|text| format!("{kind}: {text}")));
    }
    report
}

/// The text of a batch as the addon sends it: the character line, then the lines.
#[must_use]
pub fn batch_text(character: &Value, batch: &Batch) -> String {
    let mut text = character.to_string();
    for line in &batch.lines {
        text.push('\n');
        text.push_str(&Value::Object(line.clone()).to_string());
    }
    text
}

fn model_with(answers: &Answers, mut model: Option<Model>) -> fake_bridge::ModelByKind {
    let answers = Rc::clone(answers);
    Box::new(move |kind, prompt| {
        let fixed = answers
            .borrow_mut()
            .get_mut(kind)
            .and_then(VecDeque::pop_front);
        fixed.or_else(|| model.as_mut().and_then(|model| model(prompt)))
    })
}

fn read_reply(reply: &Reply, report: &mut Report) {
    let text = match reply {
        Reply::Done(text) => text,
        Reply::Error(error) => {
            report.failed_batches.push(error.clone());
            return;
        }
    };
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    if let Some(line) = value.get("narrator").and_then(Value::as_str) {
        report.narrator.push(line.to_string());
    }
    if let Some(line) = value.get("notice").and_then(Value::as_str) {
        report.notices.push(line.to_string());
    }
}
