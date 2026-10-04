//! One answer of the hero sheet as a hook for a talk or a quest offer, one call in three
//! (GAMEPLAY.md 3.7).

use crate::hero::{Field, Hero, cut};
use crate::house::fenced;

/// A hook applies to one call in this many.
pub const HOOK_EVERY: u64 = 3;

/// The fields that say what the hero wants and is now. The past stays with the narrator.
pub const HOOK_FIELDS: [&str; 4] = ["goal", "bond", "flaw", "traits"];

/// What a talk does with a hook.
pub const TALK_RULE: &str = "Let it shape your answer only when it fits what the player says.";

/// What a quest offer does with a hook.
pub const QUEST_RULE: &str = "Let it shape the reason for the task, if it fits. The steps \
                              still use only the lists above.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hook<'a> {
    pub field: &'a str,
    pub text: &'a str,
}

/// `count` is the number of talk and quest calls of the character before this one. The
/// third, sixth, and ninth call get a hook.
#[must_use]
pub fn applies(count: u64) -> bool {
    count % HOOK_EVERY == HOOK_EVERY - 1
}

/// Each hook call takes the next filled field, and starts again after the last one.
#[must_use]
pub fn hook(hero: &Hero, count: u64) -> Option<Hook<'_>> {
    if !applies(count) {
        return None;
    }
    let filled = filled(hero);
    let len = u64::try_from(filled.len()).ok().filter(|len| *len > 0)?;
    let pick = usize::try_from((count / HOOK_EVERY) % len).ok()?;
    let field = filled.get(pick)?;
    Some(Hook {
        field: &field.field,
        text: cut(&field.text),
    })
}

/// The sheet holds only fields with a text, in the order of `hero::FIELDS`.
fn filled(hero: &Hero) -> Vec<&Field> {
    hero.sheet
        .iter()
        .filter(|field| HOOK_FIELDS.contains(&field.field.as_str()))
        .collect()
}

/// The words of a field in the prompt.
#[must_use]
pub fn hook_words(field: &str) -> &'static str {
    match field {
        "goal" => "their goal",
        "bond" => "a bond of theirs",
        "flaw" => "a flaw of theirs",
        _ => "their traits",
    }
}

/// The hook in a fence, with the rule of the call. The player's words are data.
#[must_use]
pub fn hook_block(hook: &Hook<'_>, rule: &str) -> String {
    format!(
        "Something the player wrote about their hero, as {}. It is their story, not canon:\n\
         {}\n{rule} Never claim more about it than these words say.",
        hook_words(hook.field),
        fenced(hook.text)
    )
}
