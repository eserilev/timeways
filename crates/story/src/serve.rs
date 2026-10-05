//! One line from the bridge in, the lines of the answer out (GAMEPLAY.md 5.12). The
//! program and the fuzzer share this path.

use crate::input::{Input, MessageId};
use crate::lore::Answer;
use crate::story::{Output, Story};
use crate::talk::MAX_NPC_BYTES;
use serde_json::Value;

/// The JSON lines for stdout, the line for stderr when the input failed, and the notes
/// of the story for stderr.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Served {
    pub lines: Vec<String>,
    pub error: Option<String>,
    pub notes: Vec<String>,
}

/// Nothing that the bridge sends ends the loop: text that is not UTF-8, or that does not
/// read, is one bad input. A request that fails still gets an empty answer of its type,
/// because the bridge stops a story program that leaves a request with no answer.
#[must_use]
pub fn line(story: &mut Story, bytes: Vec<u8>) -> Served {
    let Ok(line) = String::from_utf8(bytes) else {
        return failed(Vec::new(), "bad input: not UTF-8".to_string());
    };
    let outputs = serde_json::from_str::<Input>(&line)
        .map_err(|error| format!("bad input: {error}: {line}"))
        .and_then(|input| {
            story
                .handle(input)
                .map_err(|error| format!("{error}: {line}"))
        });
    let mut served = match outputs {
        Ok(outputs) => Served {
            lines: outputs.iter().map(to_line).collect(),
            ..Served::default()
        },
        Err(error) => failed(empty_answer(&line).iter().map(to_line).collect(), error),
    };
    served.notes = story.take_notes();
    served
}

fn failed(lines: Vec<String>, error: String) -> Served {
    Served {
        lines,
        error: Some(error),
        notes: Vec::new(),
    }
}

/// These plain types always serialize.
fn to_line(output: &Output) -> String {
    serde_json::to_string(output).unwrap_or_default()
}

/// The bridge refuses a talk answer with a longer name, so a name past its limit is left out.
fn echoed_npc(request: &Value) -> String {
    let npc = request
        .get("npc")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let fits = npc.len() <= MAX_NPC_BYTES && !npc.chars().any(char::is_control);
    if fits { npc.to_string() } else { String::new() }
}

/// The empty answer to a request line, or None for a line that gets no answer of its own.
/// An event line has none: the `batch_end` of its batch answers it.
fn empty_answer(line: &str) -> Option<Output> {
    let value: Value = serde_json::from_str(line).ok()?;
    let id = MessageId(value.get("id")?.as_u64()?);
    match value.get("type")?.as_str()? {
        "lore_asked" => Some(Output::LoreAnswer {
            id,
            answer: Answer::default(),
            notice: None,
        }),
        "talk_asked" => Some(Output::TalkAnswer {
            id,
            npc: echoed_npc(&value),
            text: None,
            notice: None,
        }),
        "draft_asked" => Some(Output::DraftAnswer {
            id,
            draft: None,
            notice: None,
        }),
        // No pages: the addon keeps the journal that it shows.
        "journal_asked" => Some(Output::Journal {
            id,
            page: Box::default(),
            notice: None,
        }),
        _ => None,
    }
}
