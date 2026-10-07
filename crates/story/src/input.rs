//! What the bridge sends, one JSON object per line: game events from the addon, and the
//! answers to model calls (GAMEPLAY.md 3.1, 5.4, and 5.6).

use crate::character::Resting;
use crate::dev_fps::FpsRun;
use crate::entry_edits::{EditText, EntryKey};
use crate::places::InstanceKind;
use crate::race_class::{Class, Race};
use crate::seen::TextKind;
use crate::spot::{self, Spot};
use crate::store::Root;
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// A game event carries the time from `time()` in the addon, in seconds since the Unix epoch.
/// The bridge adds the `id` of the addon message to each line. Only a line with a reply keeps it.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    /// The first line from the bridge. The bridge compares the protocol of the reply.
    Hello,
    /// The first line of each batch: whose world the batch changes.
    CharacterEntered {
        realm: String,
        name: String,
    },
    /// The race and the class of the character, at each login (GAMEPLAY.md 3.2.1).
    CharacterDescribed {
        at: Tick,
        race: Race,
        class: Class,
    },
    /// A player whom a text of this batch names, with the race and the class that the game
    /// shows. The alias table keeps them for the card of the player (GAMEPLAY.md 5.11).
    PlayerDescribed {
        at: Tick,
        name: String,
        #[serde(default)]
        race: Option<Race>,
        #[serde(default)]
        class: Option<Class>,
    },
    /// `spot` is where the player stands as the place begins.
    ZoneEntered {
        at: Tick,
        zone: String,
        subzone: Option<String>,
        #[serde(default, deserialize_with = "spot::lenient")]
        spot: Option<Spot>,
        /// The local hour of the player, from 0 to 23.
        #[serde(default)]
        hour: Option<u8>,
        /// "yes" when you flew over the place on a flight path (docs/plans/chapters.md 9).
        #[serde(default)]
        taxi: Option<Taxi>,
    },
    /// The local hour changed while a time-of-day step is open (docs/plans/quest-variety.md
    /// 4.5).
    HourChanged {
        at: Tick,
        hour: u8,
    },
    /// The zone that just came is an instance. It follows its `ZoneEntered`.
    InstanceEntered {
        at: Tick,
        zone: String,
        kind: InstanceKind,
    },
    /// `spot` is where the player stands at the meeting.
    NpcMet {
        at: Tick,
        name: String,
        #[serde(default, deserialize_with = "spot::lenient")]
        spot: Option<Spot>,
    },
    /// You hovered or targeted an NPC. The addon sends each NPC once in a session, and never
    /// a player or a pet. `creature` is the English creature type in lower case: "beast".
    NpcSeen {
        at: Tick,
        name: String,
        reaction: Reaction,
        #[serde(default)]
        creature: Option<String>,
    },
    /// You or your group killed a creature of the kill step of a task (GAMEPLAY.md 3.4).
    NpcKilled {
        at: Tick,
        name: String,
    },
    LevelReached {
        at: Tick,
        level: u8,
    },
    /// The player killed a rare or a boss. Common mobs never come (GAMEPLAY.md 5.13).
    /// `kind` is the classification of the game, when the addon saw the unit.
    NpcDefeated {
        at: Tick,
        name: String,
        #[serde(default)]
        kind: Option<FoeKind>,
    },
    /// You won a battle in a battleground (docs/plans/chapters.md 9).
    BgWon {
        at: Tick,
        zone: String,
    },
    /// Your rank in battle against players, at each login and when it grows. 0 is no rank.
    PvpRank {
        at: Tick,
        rank: u8,
    },
    /// You started or stopped resting at an inn or in a city.
    RestChanged {
        at: Tick,
        resting: Resting,
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
    /// You took a quest of the game (GAMEPLAY.md 5.4).
    GameQuestAccepted {
        at: Tick,
        title: String,
        kind: GameQuestKind,
    },
    /// A lasting buff or debuff came on you just after an event of this quest of the game.
    QuestMarked {
        at: Tick,
        quest: String,
        mark: String,
    },
    /// You rode a mount. The addon sends each mount once in a session. `speed` is the run
    /// speed on it, in percent of a run on foot: 160 for a mount of level 40, 200 for an
    /// epic one. None when the game hid it.
    MountRidden {
        at: Tick,
        mount: String,
        #[serde(default)]
        speed: Option<u16>,
    },
    /// You put on an item of rare quality or better, in an inventory slot of the game (1 to
    /// 19). `quality` is the number of the game: 3 rare, 4 epic. `replaced` is the item
    /// level of what the slot held before, when the game told it.
    ItemEquipped {
        at: Tick,
        slot: u8,
        item: String,
        quality: u8,
        #[serde(default)]
        level: Option<u16>,
        #[serde(default)]
        replaced: Option<u16>,
        #[serde(default)]
        was: SlotWas,
    },
    /// You turned in a quest of the game.
    GameQuestDone {
        at: Tick,
        title: String,
        kind: GameQuestKind,
    },
    /// `/quest` to the NPC that you target (GAMEPLAY.md 3.4). The offer comes back as the
    /// narrator line of the batch, so this line has no reply of its own.
    QuestAsked {
        at: Tick,
        npc: String,
    },
    /// The answer to an offer that waits: the offer with this number, or with none, the
    /// newest offer.
    QuestAccepted {
        at: Tick,
        #[serde(default)]
        number: Option<u64>,
    },
    QuestDeclined {
        at: Tick,
        #[serde(default)]
        number: Option<u64>,
    },
    /// Gives up the quest with this number, open or waiting.
    QuestAbandoned {
        at: Tick,
        number: u64,
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
    /// A field of the sheet of the hero. An empty text clears it.
    HeroSet {
        at: Tick,
        field: String,
        text: String,
    },
    /// An entry of the player's own lore, about the NPC that they target when there is one.
    HeroAdded {
        at: Tick,
        text: String,
        #[serde(default)]
        npc: Option<String>,
    },
    HeroRemoved {
        at: Tick,
        number: u64,
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
    /// The count of one item in your bags, at a meeting with the NPC of an open carry step
    /// (docs/plans/quest-variety.md 4.9). The bags only, not the bank.
    ItemsHeld {
        at: Tick,
        npc: String,
        item: String,
        count: u16,
    },
    /// The text of a quest, a gossip window, or a book, as the player read it. `$N` stands
    /// for the name of the character (GAMEPLAY.md 5.10).
    TextSeen {
        at: Tick,
        kind: TextKind,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        npc: Option<String>,
        #[serde(default)]
        zone: Option<String>,
        text: String,
    },
    /// `/lore`, with the name of the target when there is one.
    LoreAsked {
        id: MessageId,
        question: String,
        target: Option<String>,
    },
    /// "Help me write this" for a player task (GAMEPLAY.md 4.7): the idea of the player, with
    /// each name of a player marked: "{Ada}" (5.11).
    DraftAsked {
        id: MessageId,
        at: Tick,
        idea: String,
    },
    /// The player accepted a story that a player of the party told about them (GAMEPLAY.md
    /// 4.8). The number is the addon's own. The title and each paragraph mark each name of
    /// a player: "{Ada}" (5.11). The bridge takes no line break, so the body is a list.
    StoryAccepted {
        at: Tick,
        number: u64,
        #[serde(default)]
        title: Option<String>,
        paragraphs: Vec<String>,
    },
    /// The player changed what a chapter, a tale, or the summary says (docs/plans/chapters.md
    /// 11). Each name of a player is marked: "{Ada}". The line has no reply.
    EntryEdited {
        at: Tick,
        entry: EntryKey,
        #[serde(default)]
        title: Option<String>,
        text: EditText,
        #[serde(default)]
        paragraphs: Vec<String>,
    },
    /// The player removed a story about them.
    StoryRemoved {
        at: Tick,
        number: u64,
    },
    /// A run of `/twdev fps`. It lands only in dev mode, and in no world.
    DevFps(FpsRun),
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

/// You flew over a place: a flight path of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Taxi {
    Yes,
}

/// The classification of a foe in the game. A world boss weighs as a raid boss.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoeKind {
    Rare,
    #[serde(rename = "rareelite")]
    RareElite,
    #[serde(rename = "worldboss")]
    WorldBoss,
    /// The boss of an encounter.
    Boss,
}

/// What an inventory slot held before an item. The JSON of the addon has no booleans.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotWas {
    #[default]
    Worn,
    /// Nothing since the login.
    Empty,
}

/// Can you attack the NPC? A hostile one is a foe to hunt, never someone to meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reaction {
    Hostile,
    Friendly,
}

/// The quest log of the game puts a quest of your class under a header with the name of
/// the class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameQuestKind {
    Normal,
    Class,
}

impl Input {
    /// A line of a model call fills its call. `hello`, a journal request, a `/lore`
    /// question, and a run of `/twdev fps` change nothing in the world. Every other line is kept as an input
    /// (GAMEPLAY.md 5.14).
    #[must_use]
    pub fn is_kept(&self) -> bool {
        !matches!(
            self,
            Input::Hello
                | Input::JournalAsked { .. }
                | Input::DevFps(_)
                | Input::LoreAsked { .. }
                | Input::ModelAnswered { .. }
                | Input::ModelFailed { .. }
        )
    }

    /// The player typed it, or clicked a button of Timeways. Every other kept line comes
    /// from the game.
    #[must_use]
    pub fn root(&self) -> Root {
        let typed = matches!(
            self,
            Input::TalkAsked { .. }
                | Input::DraftAsked { .. }
                | Input::QuestAsked { .. }
                | Input::QuestAccepted { .. }
                | Input::QuestDeclined { .. }
                | Input::QuestAbandoned { .. }
                | Input::HeroSet { .. }
                | Input::HeroAdded { .. }
                | Input::HeroRemoved { .. }
                | Input::StoryRemoved { .. }
                | Input::EntryEdited { .. }
        );
        if matches!(self, Input::StoryAccepted { .. }) {
            return Root::Shared;
        }
        if typed { Root::Player } else { Root::Game }
    }

    /// True for a question: a line with a reply of its own. It ends its batch, so the
    /// batch gets no `batch_end`.
    #[must_use]
    pub fn is_question(&self) -> bool {
        matches!(
            self,
            Input::LoreAsked { .. }
                | Input::TalkAsked { .. }
                | Input::JournalAsked { .. }
                | Input::DraftAsked { .. }
        )
    }

    /// The local hour of the player that the line carries, when it carries one.
    #[must_use]
    pub fn hour(&self) -> Option<u8> {
        match self {
            Input::ZoneEntered { hour, .. }
            | Input::EmoteDone { hour, .. }
            | Input::Died { hour, .. } => *hour,
            Input::HourChanged { hour, .. } => Some(*hour),
            _ => None,
        }
    }

    /// The time of a game event or a request of the player, from the clock of the addon.
    pub fn at_mut(&mut self) -> Option<&mut Tick> {
        match self {
            Input::ZoneEntered { at, .. }
            | Input::CharacterDescribed { at, .. }
            | Input::PlayerDescribed { at, .. }
            | Input::NpcMet { at, .. }
            | Input::NpcSeen { at, .. }
            | Input::NpcKilled { at, .. }
            | Input::InstanceEntered { at, .. }
            | Input::LevelReached { at, .. }
            | Input::NpcDefeated { at, .. }
            | Input::BgWon { at, .. }
            | Input::PvpRank { at, .. }
            | Input::RestChanged { at, .. }
            | Input::NpcSlapped { at, .. }
            | Input::GameQuestAccepted { at, .. }
            | Input::GameQuestDone { at, .. }
            | Input::QuestMarked { at, .. }
            | Input::MountRidden { at, .. }
            | Input::ItemEquipped { at, .. }
            | Input::TalkAsked { at, .. }
            | Input::DraftAsked { at, .. }
            | Input::QuestAsked { at, .. }
            | Input::QuestAccepted { at, .. }
            | Input::QuestDeclined { at, .. }
            | Input::QuestAbandoned { at, .. }
            | Input::Died { at, .. }
            | Input::HeroSet { at, .. }
            | Input::HeroAdded { at, .. }
            | Input::HeroRemoved { at, .. }
            | Input::StoryAccepted { at, .. }
            | Input::StoryRemoved { at, .. }
            | Input::EntryEdited { at, .. }
            | Input::EmoteDone { at, .. }
            | Input::ItemsHeld { at, .. }
            | Input::HourChanged { at, .. }
            | Input::TextSeen { at, .. } => Some(at),
            Input::Hello
            | Input::CharacterEntered { .. }
            | Input::LoreAsked { .. }
            | Input::JournalAsked { .. }
            | Input::DevFps(_)
            | Input::BatchEnd { .. }
            | Input::ModelAnswered { .. }
            | Input::ModelFailed { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CallId(pub u64);

/// The addon message of a question. Its answer carries it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub u64);
