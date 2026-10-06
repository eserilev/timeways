//! The game text that the player saw: quests, gossip, and books (GAMEPLAY.md 5.10). The
//! client of Forever shows it, so it is canon. The player read it, so it passes the spoiler
//! limit.

use crate::pack::{Link, Origin, Passage, match_query};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The addon cuts a text at 2000 bytes. The rest is room for its JSON escapes.
pub const MAX_SEEN_BYTES: usize = 2400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    Quest,
    Gossip,
    Book,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeenText {
    pub kind: TextKind,
    pub title: Option<String>,
    pub npc: Option<String>,
    pub zone: Option<String>,
    pub text: String,
}

impl SeenText {
    #[must_use]
    pub fn passage(&self) -> Passage {
        let words: Vec<&str> = self.text.split_whitespace().collect();
        let mut links = Vec::new();
        links.extend(self.npc.clone().map(Link::Npc));
        links.extend(self.zone.clone().map(Link::Place));
        Passage {
            text: words.join(" "),
            source: self.source(),
            links,
            origin: Origin::Read,
            about: self.own_subject(),
        }
    }

    /// A quest or a book is about itself: the narrator line of a class quest tells its text.
    fn own_subject(&self) -> Option<String> {
        match self.kind {
            TextKind::Quest | TextKind::Book => self.title.clone(),
            TextKind::Gossip => None,
        }
    }

    fn source(&self) -> String {
        let place = self.zone.as_deref().unwrap_or("Azeroth");
        match (self.kind, &self.title, &self.npc) {
            (TextKind::Quest, Some(title), _) => format!("the quest \"{title}\""),
            (TextKind::Book, Some(title), _) => format!("the text of \"{title}\""),
            (_, _, Some(npc)) => format!("{npc}, in {place}"),
            _ => format!("a text in {place}"),
        }
    }

    /// The words that a search finds it by: the text, and what it is about.
    fn index_text(&self) -> String {
        let about = [&self.title, &self.npc, &self.zone];
        let mut words: Vec<&str> = about.iter().filter_map(|part| part.as_deref()).collect();
        words.push(&self.text);
        words.join("\n")
    }
}

/// A full-text index of the seen texts of one character, in memory. The file of the
/// character holds the texts, and the index is built again at each start.
pub struct SeenIndex {
    connection: Connection,
    texts: Vec<SeenText>,
    /// Each text with no zone: the zone is where you read it, and a text read again in
    /// another place is the same text.
    known: HashSet<SeenText>,
}

fn without_zone(text: &SeenText) -> SeenText {
    SeenText {
        zone: None,
        ..text.clone()
    }
}

impl SeenIndex {
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn new(texts: &[SeenText]) -> rusqlite::Result<SeenIndex> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch("CREATE VIRTUAL TABLE seen USING fts5 (text)")?;
        let mut index = SeenIndex {
            connection,
            texts: Vec::new(),
            known: HashSet::new(),
        };
        for text in texts {
            index.add(text.clone())?;
        }
        Ok(index)
    }

    #[must_use]
    pub fn contains(&self, text: &SeenText) -> bool {
        self.known.contains(&without_zone(text))
    }

    /// A text that the index holds already changes nothing.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn add(&mut self, text: SeenText) -> rusqlite::Result<()> {
        if self.contains(&text) {
            return Ok(());
        }
        let row = i64::try_from(self.texts.len()).unwrap_or(i64::MAX);
        self.connection.execute(
            "INSERT INTO seen (rowid, text) VALUES (?1, ?2)",
            params![row, text.index_text()],
        )?;
        self.known.insert(without_zone(&text));
        self.texts.push(text);
        Ok(())
    }

    /// The best texts for the words of `text`, best first, as passages.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn search(&self, text: &str, limit: u32) -> rusqlite::Result<Vec<Passage>> {
        let Some(query) = match_query(text) else {
            return Ok(Vec::new());
        };
        let mut statement = self
            .connection
            .prepare_cached("SELECT rowid FROM seen WHERE seen MATCH ?1 ORDER BY rank LIMIT ?2")?;
        let rows = statement.query_map(params![query, limit], |row| row.get::<_, i64>(0))?;
        let mut passages = Vec::new();
        for row in rows {
            let found = usize::try_from(row?)
                .ok()
                .and_then(|row| self.texts.get(row));
            passages.extend(found.map(SeenText::passage));
        }
        Ok(passages)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn gossip(npc: &str, text: &str) -> SeenText {
        SeenText {
            kind: TextKind::Gossip,
            title: None,
            npc: Some(npc.to_string()),
            zone: Some("Elwynn Forest".to_string()),
            text: text.to_string(),
        }
    }

    fn quest(title: &str, text: &str) -> SeenText {
        SeenText {
            kind: TextKind::Quest,
            title: Some(title.to_string()),
            npc: None,
            zone: None,
            text: text.to_string(),
        }
    }

    #[test]
    fn a_search_finds_a_text_by_its_words() {
        let index = SeenIndex::new(&[
            gossip("Innkeeper Farley", "The gnolls grow bold."),
            gossip("Guard Thomas", "Keep your blade sharp."),
        ])
        .unwrap();

        let found = index.search("gnolls", 5).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "The gnolls grow bold.");
    }

    #[test]
    fn a_search_finds_a_text_by_the_name_of_its_npc() {
        let index = SeenIndex::new(&[gossip("Innkeeper Farley", "Welcome, friend.")]).unwrap();

        let found = index.search("Farley", 5).unwrap();

        assert_eq!(found[0].source, "Innkeeper Farley, in Elwynn Forest");
    }

    #[test]
    fn the_same_text_twice_is_kept_once() {
        let mut index = SeenIndex::new(&[]).unwrap();

        index.add(gossip("Innkeeper Farley", "Welcome.")).unwrap();
        index.add(gossip("Innkeeper Farley", "Welcome.")).unwrap();

        assert_eq!(index.search("Welcome", 5).unwrap().len(), 1);
    }

    #[test]
    fn the_mark_of_the_name_stays_for_the_addon() {
        let text = quest("Wanted: Hogger", "Well met, $N. Hogger\n\nmust die.");

        let passage = text.passage();

        assert_eq!(passage.text, "Well met, $N. Hogger must die.");
        assert_eq!(passage.source, "the quest \"Wanted: Hogger\"");
    }

    #[test]
    fn a_passage_of_seen_text_is_marked_as_read() {
        let passage = gossip("Innkeeper Farley", "Welcome.").passage();

        assert_eq!(passage.origin, Origin::Read);
    }

    #[test]
    fn a_passage_links_its_npc_and_its_place() {
        let passage = gossip("Innkeeper Farley", "Welcome.").passage();

        assert_eq!(
            passage.links,
            [
                Link::Npc("Innkeeper Farley".to_string()),
                Link::Place("Elwynn Forest".to_string())
            ]
        );
    }

    #[test]
    fn syntax_of_the_index_in_a_question_is_plain_words() {
        let index = SeenIndex::new(&[gossip("Innkeeper Farley", "NEAR the road.")]).unwrap();

        let found = index.search("NEAR( \"road", 5).unwrap();

        assert_eq!(found.len(), 1);
    }
}
