//! Where the desktop app of the player keeps its files, and the `[story]` table of its
//! config: the lore pack and the model (relay SPEC.md 9.7). The paths follow the bridge.

use crate::model_runner::LocalModel;
use serde::Deserialize;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use thiserror::Error;
use timeways_story::ai::Ai;
use timeways_story::dev_mode::{DevMode, SETTINGS_FILE};

/// Claude Code with no tools, no MCP servers, and no settings, as the review tool runs it.
const CLAUDE: &str = "claude -p --tools '' --strict-mcp-config --setting-sources ''";
/// The free local model that the setup of the relay installs (relay `ollama_install.rs`).
const LOCAL_URL: &str = "http://127.0.0.1:11434";
const LOCAL_MODEL: &str = "llama3.2:3b";

#[derive(Debug, Error)]
pub enum DesktopError {
    #[error("no home folder: set HOME")]
    NoHome,
    #[error("{path} does not read as TOML. Fix it or delete it, then try again.")]
    BadSettings { path: PathBuf },
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// The folders of the desktop app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Desktop {
    /// The data folder of the story program: `<data>/gnomish-relay/timeways/story`.
    pub story: PathBuf,
    /// The config of the bridge: `<config>/gnomish-relay/config.toml`.
    pub config: PathBuf,
}

/// The model of the `[story]` table, as a shell command that reads the prompt on stdin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSetting {
    None,
    Command(String),
    /// A local model behind the OpenAI-compatible API, such as Ollama.
    Local(LocalModel),
    /// A model that this tool cannot run.
    Other(String),
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    story: StoryTable,
}

#[derive(Debug, Default, Deserialize)]
struct StoryTable {
    lore_pack: Option<String>,
    model: Option<String>,
    claude_model: Option<String>,
    local_url: Option<String>,
    local_model: Option<String>,
}

impl Desktop {
    /// The folders of this computer, from the environment, as the bridge finds them.
    ///
    /// # Errors
    ///
    /// Returns `NoHome` when the environment names no home folder.
    pub fn here() -> Result<Desktop, DesktopError> {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let home = var("HOME").or_else(|| var("USERPROFILE"));
        let (config, data) = if cfg!(windows) {
            (var("APPDATA"), var("LOCALAPPDATA"))
        } else if cfg!(target_os = "macos") {
            let support = home.map(|home| home.join("Library").join("Application Support"));
            (support.clone(), support)
        } else {
            (
                var("XDG_CONFIG_HOME").or_else(|| home.as_ref().map(|home| home.join(".config"))),
                var("XDG_DATA_HOME").or_else(|| home.map(|home| home.join(".local").join("share"))),
            )
        };
        let (Some(config), Some(data)) = (config, data) else {
            return Err(DesktopError::NoHome);
        };
        Ok(Desktop::under(&config, &data))
    }

    #[must_use]
    pub fn under(config: &Path, data: &Path) -> Desktop {
        Desktop {
            story: data.join("gnomish-relay").join("timeways").join("story"),
            config: config.join("gnomish-relay").join("config.toml"),
        }
    }

    fn story_table(&self) -> StoryTable {
        std::fs::read_to_string(&self.config)
            .ok()
            .and_then(|text| toml::from_str::<Config>(&text).ok())
            .unwrap_or_default()
            .story
    }

    /// The lore pack of the config, with `~` as the home folder.
    #[must_use]
    pub fn lore_pack(&self) -> Option<PathBuf> {
        self.story_table().lore_pack.map(|path| home_path(&path))
    }

    #[must_use]
    pub fn model(&self) -> ModelSetting {
        let table = self.story_table();
        match table.model.as_deref() {
            None | Some("none" | "") => ModelSetting::None,
            Some("claude") => ModelSetting::Command(self.claude_command()),
            Some("local") => ModelSetting::Local(self.local_model()),
            Some(other) => ModelSetting::Other(other.to_string()),
        }
    }

    /// The model of the config in a few words, for an export: "claude", "claude opus",
    /// "local llama3.2:3b", or "none". It holds no command and no path, so it names no
    /// folder of the player.
    #[must_use]
    pub fn model_label(&self) -> String {
        let table = self.story_table();
        match table.model.as_deref() {
            None | Some("none" | "") => "none".to_string(),
            Some("claude") => match table.claude_model {
                Some(model) => format!("claude {model}"),
                None => "claude".to_string(),
            },
            Some("local") => format!("local {}", self.local_model().model),
            Some(other) => other.to_string(),
        }
    }

    /// Claude Code with the `claude_model` of the config, as the bridge runs it.
    #[must_use]
    pub fn claude_command(&self) -> String {
        match self.story_table().claude_model {
            Some(model) => format!("{CLAUDE} --model '{model}'"),
            None => CLAUDE.to_string(),
        }
    }

    /// The local model of the config. With none, the one that the setup of the relay
    /// installs.
    #[must_use]
    pub fn local_model(&self) -> LocalModel {
        let table = self.story_table();
        LocalModel {
            url: table.local_url.unwrap_or_else(|| LOCAL_URL.to_string()),
            model: table.local_model.unwrap_or_else(|| LOCAL_MODEL.to_string()),
        }
    }

    #[must_use]
    pub fn dev_mode(&self) -> DevMode {
        DevMode::of_folder(&self.story)
    }

    /// The dev switch `ai = "off"` of the settings (TESTING.md, "Dev mode").
    #[must_use]
    pub fn dev_ai(&self) -> Ai {
        Ai::of_folder(&self.story)
    }

    /// Writes `dev = true` or `dev = false` into the settings of the story program, and
    /// keeps the switch `ai = "off"`. The story program reads it at its start.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system, or a file that does not read as TOML.
    pub fn set_dev_mode(&self, mode: DevMode) -> Result<(), DesktopError> {
        let ai = self
            .settings()?
            .map_or(Ai::On, |text| Ai::of_settings(&text));
        self.write_settings(mode, ai)
    }

    /// Writes `ai = "off"` into the settings of the story program, or takes it out, and
    /// keeps dev mode. The story program reads it at its start.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system, or a file that does not read as TOML.
    pub fn set_dev_ai(&self, ai: Ai) -> Result<(), DesktopError> {
        let mode = self
            .settings()?
            .map_or(DevMode::Off, |text| DevMode::of_settings(&text));
        self.write_settings(mode, ai)
    }

    /// The text of the settings, or None with no file. A file that does not read stops a
    /// change, so a switch that a person wrote by hand is never lost.
    fn settings(&self) -> Result<Option<String>, DesktopError> {
        let path = self.story.join(SETTINGS_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(DesktopError::Io { path, source }),
        };
        match text.parse::<toml::Table>() {
            Ok(_) => Ok(Some(text)),
            Err(_) => Err(DesktopError::BadSettings { path }),
        }
    }

    /// The story program reads only these two keys, so the file holds only them.
    fn write_settings(&self, mode: DevMode, ai: Ai) -> Result<(), DesktopError> {
        let path = self.story.join(SETTINGS_FILE);
        let mut text = format!("{SETTINGS_HEADER}dev = {}\n", mode.is_on());
        if ai == Ai::Off {
            text.push_str("ai = \"off\"\n");
        }
        std::fs::create_dir_all(&self.story)
            .and_then(|()| std::fs::write(&path, text))
            .map_err(|source| DesktopError::Io { path, source })
    }
}

const SETTINGS_HEADER: &str = "# Dev mode of Timeways: /twdev in the game (TESTING.md, \"Dev mode\").\n\
                               # The story program reads this file at its start.\n";

fn home_path(path: &str) -> PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    match (path.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}
