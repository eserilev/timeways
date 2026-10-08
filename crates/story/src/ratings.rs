//! The player's ratings of narrator text (GAMEPLAY.md 3.2.2): a narrator line, a chapter,
//! a tale, or the summary, with a thumb up or down, and a reason for a thumb down. Ratings are off unless the player turns
//! them on in the game. They stay in the world of the character, and only an export by
//! the player takes them off the computer. An export never holds the name of a real player.

use crate::aliases::{alias_of, text_pieces, without_names};
use crate::store::RowLog;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use timeways_rules::aliases::{Alias, Plain};

/// What a rating is about. The addon names it, and the desktop finds its text in its own
/// rows, so the addon never sends the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rated {
    /// The narrator line whose call is `line`.
    Narrator,
    /// The story of the chapter whose first event is `first`.
    Chapter,
    /// The text of the tale whose first event is `first`.
    Tale,
    /// Who the character has become, as the newest summary tells it.
    Summary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Up,
    Down,
}

/// Why the player disliked a text. The addon offers one list for narrator text and another
/// for a lore answer, so each reason names one kind of fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    WrongLore,
    MadeUpName,
    Boring,
    TooLong,
    DoesntFit,
    Other,
    /// The reasons of a lore answer, for the lore book (a later step).
    Wrong,
    NotWhatIAsked,
    Spoiler,
}

impl Reason {
    /// True for a reason that the addon offers for narrator text.
    #[must_use]
    pub fn fits_narrator_text(self) -> bool {
        !matches!(
            self,
            Reason::Wrong | Reason::NotWhatIAsked | Reason::Spoiler
        )
    }
}

/// The reason that a row keeps: only a dislike has one, and only a reason of its kind.
#[must_use]
pub fn kept_reason(rating: Rating, reason: Option<Reason>) -> Option<Reason> {
    let reason = reason.filter(|reason| reason.fits_narrator_text())?;
    (rating == Rating::Down).then_some(reason)
}

/// One row of `ratings`. The text keeps `$N` and the ID of each player (GAMEPLAY.md 5.11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatedLine {
    pub at: Tick,
    pub rated: Rated,
    /// The first event of a chapter or a tale, or the row of the call of a narrator line.
    /// The newest rating of one key stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<u64>,
    pub rating: Rating,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// The kind of the narrator moment (`first_kill`), or the kind of the entry.
    pub moment: String,
    pub text: String,
    /// The reasons of the checks that refused an earlier answer of the same line.
    #[serde(default)]
    pub faults: Vec<String>,
}

/// One row of `narrator_lines`: a line that the story program showed, so a rating finds it
/// by the row of its call, also after a restart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownLine {
    pub call: u64,
    pub moment: String,
    /// The line with `$N` and the IDs of players, as the disk keeps text.
    pub text: String,
    #[serde(default)]
    pub faults: Vec<String>,
}

/// One rating of an export: what a player can send to the makers of Timeways.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExportedRating {
    pub rated: Rated,
    pub rating: Rating,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    pub moment: String,
    pub text: String,
    pub faults: Vec<String>,
    /// The model of the config when the export ran. The story program never learns which
    /// model the bridge ran, so an older rating can come from another model.
    pub model: String,
}

/// The newest rating of each key, oldest first, with every name of a player taken out.
/// `players` is the alias table, and `own` the name of the character.
#[must_use]
pub fn export(
    ratings: &RowLog<RatedLine>,
    players: &[Alias],
    own: &str,
    model: &str,
) -> Vec<ExportedRating> {
    let rows = ratings.rows();
    let newest = rows.iter().enumerate().filter(|(n, row)| {
        !rows[n + 1..]
            .iter()
            .any(|later| later.rated == row.rated && later.key == row.key)
    });
    newest
        .map(|(_, row)| ExportedRating {
            rated: row.rated,
            rating: row.rating,
            reason: row.reason,
            moment: row.moment.clone(),
            text: shareable(&row.text, players, own),
            faults: row
                .faults
                .iter()
                .map(|fault| shareable(fault, players, own))
                .collect(),
            model: model.to_string(),
        })
        .collect()
}

/// The text with each name that the alias table knows as its ID, and the name of the
/// character as `$N`. The swap of the alias table is the proved one
/// (`timeways_rules::aliases`).
#[must_use]
pub fn shareable(text: &str, players: &[Alias], own: &str) -> String {
    let with_ids = without_names(players, text);
    let Some(own) = alias_of(own) else {
        return with_ids;
    };
    let mut shared = String::with_capacity(with_ids.len());
    for piece in text_pieces(&with_ids, std::slice::from_ref(&own)) {
        match piece {
            Plain::Word { key, .. } if key == own.key => shared.push_str("$N"),
            Plain::Word { written, .. } => shared.push_str(&written),
            Plain::Text(text) => shared.push_str(&text),
        }
    }
    shared
}
