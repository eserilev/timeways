//! What the player learned (GAMEPLAY.md 3.1.1): the game text that they read, which is
//! canon, and the words of NPCs in `/talk`, which a model wrote, so they are rumors.

use crate::seen::{SeenText, TextKind};
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// Enough for the gist on the Learned page. 240 characters take at most 960 bytes, far
/// below the limit of the bridge for one string of the journal.
pub const EXCERPT_CHARS: usize = 240;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Read {
    pub at: Tick,
    #[serde(flatten)]
    pub text: SeenText,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rumor {
    pub at: Tick,
    pub npc: String,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LearnedKind {
    Quest,
    Gossip,
    Book,
    Rumor,
}

/// One entry of the Learned page. `$N` stays in the excerpt, and the addon shows the name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Learned {
    pub kind: LearnedKind,
    pub title: Option<String>,
    pub npc: Option<String>,
    pub place: Option<String>,
    pub at: Tick,
    pub excerpt: String,
}

/// Oldest first. Of two entries at the same time, the read text comes first.
#[must_use]
pub fn learned(read: &[Read], rumors: &[Rumor]) -> Vec<Learned> {
    let mut entries: Vec<Learned> = read.iter().map(from_read).collect();
    entries.extend(rumors.iter().map(from_rumor));
    entries.sort_by_key(|entry| entry.at);
    entries
}

fn from_read(read: &Read) -> Learned {
    let kind = match read.text.kind {
        TextKind::Quest => LearnedKind::Quest,
        TextKind::Gossip => LearnedKind::Gossip,
        TextKind::Book => LearnedKind::Book,
    };
    Learned {
        kind,
        title: read.text.title.clone(),
        npc: read.text.npc.clone(),
        place: read.text.zone.clone(),
        at: read.at,
        excerpt: excerpt(&read.text.text),
    }
}

fn from_rumor(rumor: &Rumor) -> Learned {
    Learned {
        kind: LearnedKind::Rumor,
        title: None,
        npc: Some(rumor.npc.clone()),
        place: None,
        at: rumor.at,
        excerpt: excerpt(&rumor.text),
    }
}

/// The text on one line, cut at a whole word when it is longer than `EXCERPT_CHARS`.
#[must_use]
pub fn excerpt(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= EXCERPT_CHARS {
        return line;
    }
    let cut: String = line.chars().take(EXCERPT_CHARS).collect();
    let whole = cut
        .rsplit_once(' ')
        .map_or(cut.as_str(), |(words, _)| words);
    format!("{whole}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(at: u64, kind: TextKind, text: &str) -> Read {
        Read {
            at: Tick(at),
            text: SeenText {
                kind,
                title: Some("The Kingdom of Stormwind".to_string()),
                npc: None,
                zone: Some("Stormwind City".to_string()),
                text: text.to_string(),
            },
        }
    }

    fn rumor(at: u64, text: &str) -> Rumor {
        Rumor {
            at: Tick(at),
            npc: "Innkeeper Farley".to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn read_text_and_rumors_come_in_the_order_of_time() {
        let entries = learned(
            &[read(1, TextKind::Book, "a"), read(3, TextKind::Quest, "c")],
            &[rumor(2, "b")],
        );

        let kinds: Vec<LearnedKind> = entries.iter().map(|entry| entry.kind).collect();
        assert_eq!(
            kinds,
            [LearnedKind::Book, LearnedKind::Rumor, LearnedKind::Quest]
        );
    }

    #[test]
    fn a_rumor_names_its_npc_and_is_marked_as_a_rumor() {
        let entries = learned(&[], &[rumor(2, "The gnolls grow bold.")]);

        assert_eq!(entries[0].kind, LearnedKind::Rumor);
        assert_eq!(entries[0].npc.as_deref(), Some("Innkeeper Farley"));
    }

    #[test]
    fn a_book_keeps_its_title_and_its_place() {
        let entries = learned(&[read(1, TextKind::Book, "Long ago.")], &[]);

        assert_eq!(
            entries[0].title.as_deref(),
            Some("The Kingdom of Stormwind")
        );
        assert_eq!(entries[0].place.as_deref(), Some("Stormwind City"));
    }

    #[test]
    fn a_short_text_stays_whole_on_one_line() {
        assert_eq!(excerpt("Well met,\n\n$N."), "Well met, $N.");
    }

    #[test]
    fn a_long_text_is_cut_at_a_whole_word() {
        let text = "word ".repeat(100);

        let cut = excerpt(&text);

        assert!(cut.ends_with("word..."), "{cut}");
        assert!(cut.chars().count() <= EXCERPT_CHARS + 3);
    }

    #[test]
    fn a_long_word_with_no_space_is_cut_at_the_limit() {
        let cut = excerpt(&"é".repeat(EXCERPT_CHARS + 10));

        assert_eq!(cut.chars().count(), EXCERPT_CHARS + 3);
    }
}
