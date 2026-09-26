//! What the bridge sends, one JSON object per line: game events from the addon, and the
//! answers to model calls (GAMEPLAY.md 3.1, 5.4, and 5.6).

use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// A game event carries the time from `time()` in the addon, in seconds since the Unix epoch.
/// The bridge adds the `id` of the addon message to each line. Only a line with a reply keeps it.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    /// The first line from the bridge. The bridge compares the protocol of the reply.
    Hello,
    /// The first line of each batch: whose world the batch changes.
    CharacterEntered {
        realm: String,
        name: String,
    },
    ZoneEntered {
        at: Tick,
        zone: String,
        subzone: Option<String>,
    },
    NpcMet {
        at: Tick,
        name: String,
    },
    LevelReached {
        at: Tick,
        level: u8,
    },
    /// The player killed a rare or a boss. Common mobs never come (GAMEPLAY.md 5.13).
    NpcDefeated {
        at: Tick,
        name: String,
    },
    /// You slapped an NPC with `/slap`. Never a player (5.11).
    NpcSlapped {
        at: Tick,
        name: String,
    },
    /// `/talk` to the NPC that you target (GAMEPLAY.md 3.5). Never a player (5.11).
    TalkAsked {
        id: MessageId,
        at: Tick,
        npc: String,
        text: String,
    },
    /// You died. `killer` is an NPC that the addon is sure of, and never a player (5.11).
    Died {
        at: Tick,
        killer: Option<String>,
        /// The killer of the world, from the death recap: "falling", "drowning", "lava".
        #[serde(default)]
        cause: Option<String>,
        #[serde(default)]
        killer_level: Option<u8>,
        /// The local hour of the player, from 0 to 23.
        #[serde(default)]
        hour: Option<u8>,
    },
    /// Any emote of yours, with its NPC target when it has one (GAMEPLAY.md 5.4.1).
    EmoteDone {
        at: Tick,
        emote: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        hour: Option<u8>,
    },
    /// `/lore`, with the name of the target when there is one.
    LoreAsked {
        id: MessageId,
        question: String,
        target: Option<String>,
    },
    /// The journal window opened, and needs its pages. Page 0 takes a new snapshot.
    JournalAsked {
        id: MessageId,
        #[serde(default)]
        page: usize,
    },
    /// The bridge sends it after the lines of each batch. It gets `events_seen`.
    BatchEnd {
        id: MessageId,
    },
    ModelAnswered {
        call: CallId,
        text: String,
    },
    /// No model, a timeout, or a spent budget. The bridge decides which.
    ModelFailed {
        call: CallId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CallId(pub u64);

/// The addon message of a question. Its answer carries it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub u64);
