//! The size of a reply, as the bridge counts it twice (relay SPEC.md 9.8): the bytes of
//! its JSON line, and its bytes in the slot of the game after the Lua escape.

use serde::Serialize;

/// The largest reply line that the bridge takes.
pub const MAX_LINE: usize = 24_576;

/// A slot record holds at most 32 KB after the Lua escape (relay SPEC.md S12).
pub const MAX_SLOT: usize = 32_768;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub line: usize,
    pub slot: usize,
}

impl Size {
    #[must_use]
    pub fn of_json(json: &[u8]) -> Size {
        Size {
            line: json.len(),
            slot: json.iter().map(|byte| slot_bytes(*byte)).sum(),
        }
    }

    /// A value that does not serialize counts as too big for any reply.
    #[must_use]
    pub fn of(value: &impl Serialize) -> Size {
        serde_json::to_vec(value).map_or(
            Size {
                line: MAX_LINE,
                slot: MAX_SLOT,
            },
            |json| Size::of_json(&json),
        )
    }

    #[must_use]
    pub fn plus(self, other: Size) -> Size {
        Size {
            line: self.line + other.line,
            slot: self.slot + other.slot,
        }
    }

    #[must_use]
    pub fn fits(self, budget: Size) -> bool {
        self.line <= budget.line && self.slot <= budget.slot
    }
}

/// The bridge doubles each `|`, so the game shows it as text. Then the Lua escape writes 4
/// bytes for a quote, a backslash, and each byte outside printable ASCII.
fn slot_bytes(byte: u8) -> usize {
    match byte {
        b'|' => 2,
        b'"' | b'\\' => 4,
        b' '..=b'~' => 1,
        _ => 4,
    }
}
