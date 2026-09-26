//! One line from the bridge in, the lines of the answer out (GAMEPLAY.md 5.12). The
//! program and the fuzzer share this path.

use crate::input::Input;
use crate::story::Story;

/// The JSON lines for stdout, or the one line for stderr. Nothing that the bridge sends
/// ends the loop: text that is not UTF-8, or that does not read, is one bad input.
///
/// # Errors
///
/// Returns the log line of a bad input, of a refusal, or of an output that did not write.
pub fn line(story: &mut Story, bytes: Vec<u8>) -> Result<Vec<String>, String> {
    let Ok(line) = String::from_utf8(bytes) else {
        return Err("bad input: not UTF-8".to_string());
    };
    let input = serde_json::from_str::<Input>(&line)
        .map_err(|error| format!("bad input: {error}: {line}"))?;
    let outputs = story
        .handle(input)
        .map_err(|error| format!("{error}: {line}"))?;
    outputs
        .iter()
        .map(|output| serde_json::to_string(output).map_err(|error| format!("{error}: {line}")))
        .collect()
}
