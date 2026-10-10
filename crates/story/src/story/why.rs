//! Why an NPC trusts you as it does: the cause of the newest change of its trust
//! (GAMEPLAY.md 5.14). It follows one link back to the line or the call that made the
//! change, never the reads of a call, which hold far more than the cause.

use super::Active;
use crate::character::Character;
use crate::store::{Database, Node, StoreError, Table};
use crate::vocabulary::QUEST_DONE;
use hourglass::{EventId, EventKind, Tick};
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
    let cause = cause.map(|cause| match cause {
        TrustCause::Slap if finishes_a_quest(character, event) => TrustCause::Quest,
        cause => cause,
    });
    Ok(cause.map(|by| TrustWhy { by, up, at }))
}

/// A talk changes trust in the answer of its call. A slap and a finished quest change it
/// on a line of the game.
fn cause_of(database: &Database, event: Node) -> Result<Option<TrustCause>, StoreError> {
    match database.origin(event)? {
        Some(Node::Call(call)) => {
            let talk = database.call(call)?.is_some_and(|call| is_talk(&call.kind));
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

/// A slap can do the last step of a quest in the same line. `finish_quest` writes
/// `quest_done` just before the change of trust.
fn finishes_a_quest(character: &Character, trust_change: EventId) -> bool {
    let Some(before) = trust_change.0.checked_sub(1) else {
        return false;
    };
    let history = character.world().history();
    history.get(EventId(before)).is_some_and(
        |event| matches!(&event.kind, EventKind::FactStart { name, .. } if name == QUEST_DONE),
    )
}

fn is_talk(kind: &str) -> bool {
    kind == "talk" || kind == super::calls::TALK_RETRY
}
