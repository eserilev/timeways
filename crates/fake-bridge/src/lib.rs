//! A fake Gnomish Relay bridge for the tests and the fuzzers. It runs the real checks of
//! the relay (relay SPEC.md 9.8) around the real story program. So a line that the real
//! bridge refuses, or a request that gets no answer, fails the test.
//!
//! A broken protocol ends the test with a panic that names the line. Only tests and
//! fuzzers use this crate.

#![allow(
    clippy::panic,
    clippy::missing_panics_doc,
    reason = "a broken protocol ends the test"
)]

pub mod edges;

use app_protocol::addon_lines::{AddonLine, forwarded_line, read_batch};
use app_protocol::model_answer::clean_answer;
use app_protocol::story_lines::{
    Asked, CallId, FromStory, LineCheck, NO_SANDBOX, RequestId, batch_end_line,
    model_answered_line, model_failed_line, read_line, reply_text,
};
use std::collections::{BTreeMap, VecDeque};
use timeways_story::serve;
use timeways_story::story::Story;

/// At most 2 model calls of the story program run at once (relay SPEC.md 9.8).
const MAX_OPEN_CALLS: usize = 2;

/// What the addon gets for a batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The done reply: the JSON line that the bridge writes for the game.
    Done(String),
    /// The plain text of an error reply.
    Error(String),
}

/// The words of the model for a prompt, or None for a failed call.
pub type Model = Box<dyn FnMut(&str) -> Option<String>>;

/// The words of the model for the kind of a call ("narrator", "quest", ...) and its
/// prompt, or None for a failed call.
pub type ModelByKind = Box<dyn FnMut(&str, &str) -> Option<String>>;

pub struct FakeBridge {
    story: Story,
    model: Option<ModelByKind>,
    next_id: u64,
    /// Each batch that the story program has and did not answer yet, with whether it
    /// holds a request line.
    /// What each open batch asks for, or None for a batch of events only.
    waiting: BTreeMap<u64, Option<Asked>>,
    replies: BTreeMap<u64, Reply>,
    open_calls: VecDeque<(CallId, String)>,
    refused_calls: usize,
    dropped_lines: usize,
    /// What the story program wrote to stderr.
    errors: Vec<String>,
}

impl FakeBridge {
    /// A bridge with no model: every model call fails at once.
    #[must_use]
    pub fn new(story: Story) -> FakeBridge {
        FakeBridge {
            story,
            model: None,
            next_id: 1,
            waiting: BTreeMap::new(),
            replies: BTreeMap::new(),
            open_calls: VecDeque::new(),
            refused_calls: 0,
            dropped_lines: 0,
            errors: Vec::new(),
        }
    }

    /// With a model, a call stays open until `settle`.
    #[must_use]
    pub fn with_model(self, mut model: Model) -> FakeBridge {
        self.with_model_by_kind(Box::new(move |_, prompt| model(prompt)))
    }

    /// A model that also reads the kind of each call.
    #[must_use]
    pub fn with_model_by_kind(mut self, model: ModelByKind) -> FakeBridge {
        self.model = Some(model);
        self
    }

    /// One addon batch, and its reply once every model call ended.
    pub fn batch(&mut self, text: &str) -> Reply {
        let id = self.send(text);
        self.settle();
        self.reply(id)
    }

    /// Sends one addon batch as the bridge does, and gives its id. The reply can wait for
    /// a model call.
    pub fn send(&mut self, text: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let batch = match read_batch(text) {
            Ok(batch) => batch,
            Err(refused) => {
                self.replies
                    .insert(id, Reply::Error(format!("refused: {refused:?}")));
                return id;
            }
        };
        self.dropped_lines += batch.dropped.len();
        let asked = batch.lines.iter().find_map(AddonLine::asked);
        self.waiting.insert(id, asked);
        for line in &batch.lines {
            self.write_line(&forwarded_line(RequestId(id), line));
        }
        // A request line ends its batch itself, so each batch gets one answer line.
        if asked.is_none() {
            self.write_line(&batch_end_line(RequestId(id)));
        }
        id
    }

    /// Answers every open model call, oldest first, until none is open.
    pub fn settle(&mut self) {
        while self.settle_one() {}
    }

    /// Answers the oldest open model call, and gives false when none is open.
    pub fn settle_one(&mut self) -> bool {
        let Some((call, prompt)) = self.open_calls.pop_front() else {
            return false;
        };
        let kind = self
            .story
            .call_kind(timeways_story::input::CallId(call.0))
            .unwrap_or_default();
        let words = self.model.as_mut().and_then(|model| model(kind, &prompt));
        match words {
            // The bridge cleans every answer of a model before the story program sees it.
            Some(words) => self.write_line(&model_answered_line(call, &clean_answer(&words))),
            None => self.write_line(&model_failed_line(call)),
        }
        true
    }

    /// The reply of a batch. A batch with no answer fails the test, because the real
    /// bridge then waits 120 s and stops the story program as hung.
    pub fn reply(&mut self, id: u64) -> Reply {
        self.replies.remove(&id).unwrap_or_else(|| {
            panic!(
                "the story program never answered batch #{id}. Its errors: {:?}",
                self.errors
            )
        })
    }

    /// What the story program wrote to stderr: each refused line, with its reason.
    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Model calls that failed because 2 others were open.
    #[must_use]
    pub fn refused_calls(&self) -> usize {
        self.refused_calls
    }

    /// Lines of the addon that the bridge dropped from their batch.
    #[must_use]
    pub fn dropped_lines(&self) -> usize {
        self.dropped_lines
    }

    #[must_use]
    pub fn story(&mut self) -> &mut Story {
        &mut self.story
    }

    fn write_line(&mut self, line: &str) {
        let line = line.trim_end_matches('\n');
        let served = serve::line(&mut self.story, line.as_bytes().to_vec());
        self.errors.extend(served.error);
        for output in served.lines {
            self.take_output(&output);
        }
    }

    fn take_output(&mut self, line: &str) {
        match checked_line(line) {
            Checked::Hello => {}
            Checked::ModelCall(call, prompt) => self.start_call(call, prompt),
            Checked::Answer { id, asked, reply } => {
                let waits_for = self
                    .waiting
                    .remove(&id)
                    .unwrap_or_else(|| panic!("an answer that no batch waits for: {line}"));
                // The bridge takes only an answer of the kind that its batch asked for.
                assert_eq!(asked, waits_for, "an answer of the wrong type: {line}");
                self.replies.insert(id, Reply::Done(reply));
            }
        }
    }

    /// With no model, or with 2 calls open, a call fails at once, as in the bridge.
    fn start_call(&mut self, call: CallId, prompt: String) {
        if self.model.is_some() && self.open_calls.len() < MAX_OPEN_CALLS {
            self.open_calls.push_back((call, prompt));
            return;
        }
        if self.model.is_some() {
            self.refused_calls += 1;
        }
        self.write_line(&model_failed_line(call));
    }
}

/// A line of the story program that the bridge takes.
pub enum Checked {
    Hello,
    ModelCall(CallId, String),
    /// `reply` is the line that the bridge writes for the game.
    Answer {
        id: u64,
        /// None for `events_seen`.
        asked: Option<Asked>,
        reply: String,
    },
}

/// Checks a line of the story program as the bridge does. A line that the bridge refuses,
/// drops a part of, or cannot fit in a slot fails the test.
#[must_use]
pub fn checked_line(line: &str) -> Checked {
    let checked = read_line(line.as_bytes())
        .unwrap_or_else(|bad| panic!("the bridge refuses {bad:?}: {line}"));
    let (id, answer, narrator) = match checked {
        FromStory::Hello { .. } => return Checked::Hello,
        FromStory::ModelCall { call, prompt } => return Checked::ModelCall(call, prompt),
        FromStory::Answer {
            id,
            answer,
            narrator,
            notice,
        } => (id, answer, (narrator, notice)),
    };
    assert_eq!(narrator.0, LineCheck::Kept, "a dropped narrator: {line}");
    assert_eq!(narrator.1, LineCheck::Kept, "a dropped notice: {line}");
    let answer = answer.unwrap_or_else(|| panic!("an answer over 24576 bytes"));
    let reply = reply_text(&answer, Some(NO_SANDBOX))
        .unwrap_or_else(|| panic!("a reply that does not fit a slot: {line}"));
    Checked::Answer {
        id: id.0,
        asked: answer.body.asked(),
        reply,
    }
}

/// The line that the game gets for a line of the story program, or None when the bridge
/// refuses the line or cannot fit its reply in a slot.
#[must_use]
pub fn game_reply(line: &str) -> Option<String> {
    let FromStory::Answer {
        answer: Some(answer),
        ..
    } = read_line(line.as_bytes()).ok()?
    else {
        return None;
    };
    reply_text(&answer, Some(NO_SANDBOX))
}

/// The lines of an addon batch that the bridge drops, or None when it refuses the batch.
#[must_use]
pub fn dropped_lines_of(batch: &str) -> Option<usize> {
    read_batch(batch).ok().map(|batch| batch.dropped.len())
}
