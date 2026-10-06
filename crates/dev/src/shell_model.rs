//! A model as a shell command that reads the prompt on stdin and prints its answer, as the
//! review tool runs it.

use crate::seed::Model;
use std::io::Write;
use std::process::{Command, Stdio};

/// A failed command, or one that prints nothing, is a failed call.
#[must_use]
pub fn shell_model(command: String) -> Model {
    Box::new(move |prompt| ask(&command, prompt))
}

fn ask(command: &str, prompt: &str) -> Option<String> {
    let mut child = Command::new("sh")
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    // A model that never reads the prompt, such as `echo`, can exit before the write ends.
    let _ = child.stdin.take()?.write_all(prompt.as_bytes());
    let output = child.wait_with_output().ok()?;
    let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !answer.is_empty()).then_some(answer)
}
