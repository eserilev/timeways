//! Why an NPC trusts you as it does: the cause of the newest change of its trust
//! (GAMEPLAY.md 5.14). It follows one link back to the line or the call that made the
//! change, never the reads of a call, which hold far more than the cause.

use super::Active;
use crate::store::{Database, Node, StoreError, Table};
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// The three things that change trust (`Character`): a talk, a slap, and a finished quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustCause {
    Talk,
    Slap,
    Quest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustWhy {
    pub by: TrustCause,
    pub up: bool,
    pub at: Tick,
}

/// None when nothing changed the trust yet, or when the cause is lost.
pub(super) fn trust_why(active: &Active, npc: &str) -> Result<Option<TrustWhy>, StoreError> {
    let character = &active.character;
    let Some((event, up)) = character.last_trust_change(npc) else {
        return Ok(None);
    };
    let Some(at) = character
        .world()
        .history()
        .get(event)
        .map(|event| event.tick)
    else {
        return Ok(None);
    };
    let cause = cause_of(&active.database, Node::Row(Table::Events, event.0))?;
    Ok(cause.map(|by| TrustWhy { by, up, at }))
}

/// A talk changes trust in the answer of its call. A slap and a finished quest change it
/// on a line of the game.
fn cause_of(database: &Database, event: Node) -> Result<Option<TrustCause>, StoreError> {
    match database.origin(event)? {
        Some(Node::Call(call)) => {
            let talk = database.call(call)?.is_some_and(|call| call.kind == "talk");
            Ok(talk.then_some(TrustCause::Talk))
        }
        Some(Node::Input(input)) => Ok(match database.input_kind(input)?.as_deref() {
            Some("npc_slapped") => Some(TrustCause::Slap),
            Some(_) => Some(TrustCause::Quest),
            None => None,
        }),
        _ => Ok(None),
    }
}
