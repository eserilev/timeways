//! A model for the benches, with the time of each call: a shell command that reads the
//! prompt on stdin, or a local model behind the OpenAI-compatible API, as the bridge calls
//! it (relay `model_local.rs`).

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const COMPLETIONS: &str = "/v1/chat/completions";
/// A slow local model on a CPU can take minutes for one long answer.
const LOCAL_TIMEOUT_SECONDS: &str = "600";

/// Where a local model listens, as `[story] local_url` and `local_model` name it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalModel {
    pub url: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Runner {
    Shell(String),
    Local(LocalModel),
}

/// One call: the answer, or None for a failed call, and its times.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    pub answer: Option<String>,
    pub latency: Duration,
    /// The time to the first byte of the answer. A runner that sends the answer at once
    /// gives none.
    pub first_byte: Option<Duration>,
    /// The tokens of the answer, when the runner counts them.
    pub tokens: Option<u64>,
}

impl Runner {
    #[must_use]
    pub fn ask(&self, prompt: &str) -> Asked {
        match self {
            Runner::Shell(command) => ask_shell(command, prompt),
            Runner::Local(local) => ask_local(local, prompt),
        }
    }

    /// The name of the model in a report.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Runner::Shell(command) => command.clone(),
            Runner::Local(local) => format!("{} at {}", local.model, local.url),
        }
    }

    /// The names of the processes that do the work of the model, for their CPU, memory,
    /// and GPU: the server of a local model, or the program of a shell command.
    #[must_use]
    pub fn process_names(&self) -> Vec<String> {
        match self {
            Runner::Local(_) => ["ollama", "llama-server", "lms", "LM Studio"]
                .map(String::from)
                .to_vec(),
            Runner::Shell(command) => command
                .split_whitespace()
                .next()
                .and_then(|program| program.rsplit('/').next())
                .map(String::from)
                .into_iter()
                .collect(),
        }
    }
}

/// What a process printed, and its times.
struct Output {
    stdout: Vec<u8>,
    success: bool,
    latency: Duration,
    first_byte: Option<Duration>,
}

/// Writes `input` from a thread, so a process that prints before it reads all of it never
/// blocks.
fn run(mut command: Command, input: Vec<u8>) -> Option<Output> {
    let start = Instant::now();
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    // A model that never reads the prompt, such as `echo`, can exit before the write ends.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let mut stdout = child.stdout.take()?;
    let mut bytes = Vec::new();
    let mut first_byte = None;
    let mut chunk = [0_u8; 8192];
    while let Ok(read) = stdout.read(&mut chunk) {
        if read == 0 {
            break;
        }
        first_byte.get_or_insert_with(|| start.elapsed());
        bytes.extend_from_slice(&chunk[..read]);
    }
    let _ = writer.join();
    let status = child.wait().ok()?;
    Some(Output {
        stdout: bytes,
        success: status.success(),
        latency: start.elapsed(),
        first_byte,
    })
}

fn failed(start: Instant) -> Asked {
    Asked {
        answer: None,
        latency: start.elapsed(),
        first_byte: None,
        tokens: None,
    }
}

/// A failed command, or one that prints nothing, is a failed call.
fn ask_shell(command: &str, prompt: &str) -> Asked {
    let start = Instant::now();
    let mut shell = Command::new("sh");
    shell.args(["-c", command]);
    let Some(output) = run(shell, prompt.as_bytes().to_vec()) else {
        return failed(start);
    };
    let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Asked {
        answer: (output.success && !answer.is_empty()).then_some(answer),
        latency: output.latency,
        first_byte: output.first_byte,
        tokens: None,
    }
}

/// `-q` comes first, so no `.curlrc` applies, and the body goes through stdin, as in the
/// bridge.
fn curl(url: &str) -> Command {
    let mut command = Command::new("curl");
    command.args([
        "-q",
        "--silent",
        "--fail",
        "--noproxy",
        "*",
        "--max-time",
        LOCAL_TIMEOUT_SECONDS,
        "--header",
        "Content-Type: application/json",
        "--data-binary",
        "@-",
        &format!("{url}{COMPLETIONS}"),
    ]);
    command
}

fn ask_local(local: &LocalModel, prompt: &str) -> Asked {
    let start = Instant::now();
    let body = json!({
        "model": local.model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false,
    });
    let Some(output) = run(curl(&local.url), body.to_string().into_bytes()) else {
        return failed(start);
    };
    let (answer, tokens) = read_completion(&output.stdout);
    Asked {
        answer: answer.filter(|_| output.success),
        latency: output.latency,
        // The whole answer comes at once, so its first byte tells nothing.
        first_byte: None,
        tokens,
    }
}

/// The text of the first choice and the tokens of the answer. An answer that does not read
/// is a failed call.
#[must_use]
pub fn read_completion(bytes: &[u8]) -> (Option<String>, Option<u64>) {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return (None, None);
    };
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    let tokens = value
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64);
    (text, tokens)
}
