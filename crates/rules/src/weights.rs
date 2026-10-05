//! What a step of the history weighs (docs/plans/chapters.md 4). A key weighs only the first
//! time that it happens, so a repeat adds nothing. Each rule has its own constants. A new
//! rule adds an arm to these functions, and an old arm never changes.

/// The first rule of the chapters. A row of `chapter_rules` names the rule of each epoch.
pub const RULE_ONE: u8 = 1;

/// The most that one key adds in all, and so the most that one step adds.
pub const CAP_MAX: u16 = 7;

/// The first kill of a foe that killed you adds this much more.
pub const REVENGE: u16 = 2;

/// Deaths to one foe count up to this many. A later death weighs 0.
pub const DEATHS_COUNTED: u8 = 2;

/// What a key is about. The walk of the story program gives each key a kind and a dense id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    GameQuest,
    SideQuest,
    ClassQuest,
    Subzone,
    Level,
    Talk,
    /// A rare, a rare elite, or a boss of a dungeon.
    Kill,
    /// A boss of a raid, or a world boss.
    RaidKill,
    /// A death to a foe, or with no known killer.
    Death,
    Mark,
    Title,
    Mount,
    EpicMount,
    EpicItem,
    /// A big upgrade of one slot to one quality.
    Upgrade,
    PvpRank,
    Dungeon,
    Raid,
    Battleground,
    BgWin,
}

/// The weights at which a chapter may close at a break, and must close with none.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    pub min: u16,
    pub max: u16,
}

/// The weight of the first time of a key that is no death. A kill adds its revenge apart.
#[must_use]
pub fn weight(rule: u8, kind: KeyKind) -> u16 {
    // Rule 1 is the only rule so far. A new rule matches on `rule` first.
    let _ = rule;
    match kind {
        KeyKind::GameQuest
        | KeyKind::Subzone
        | KeyKind::Talk
        | KeyKind::Mark
        | KeyKind::Title
        | KeyKind::Upgrade => 1,
        KeyKind::SideQuest | KeyKind::Level | KeyKind::PvpRank => 2,
        KeyKind::ClassQuest
        | KeyKind::Kill
        | KeyKind::Dungeon
        | KeyKind::Battleground
        | KeyKind::BgWin
        | KeyKind::Mount
        | KeyKind::EpicItem => 3,
        KeyKind::RaidKill | KeyKind::Raid | KeyKind::EpicMount => 5,
        KeyKind::Death => 0,
    }
}

/// The weight of a death to a foe that you never beat, after `deaths` such deaths before it.
#[must_use]
pub fn death_weight(rule: u8, deaths: u8) -> u16 {
    let _ = rule;
    match deaths {
        0 => 2,
        1 => 1,
        _ => 0,
    }
}

#[must_use]
pub fn limits(rule: u8) -> Limits {
    let _ = rule;
    Limits { min: 15, max: 40 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raid_boss_weighs_more_than_a_rare() {
        assert!(weight(RULE_ONE, KeyKind::RaidKill) > weight(RULE_ONE, KeyKind::Kill));
    }

    #[test]
    fn deaths_weigh_two_then_one_then_nothing() {
        let weights: Vec<u16> = (0..4)
            .map(|deaths| death_weight(RULE_ONE, deaths))
            .collect();

        assert_eq!(weights, [2, 1, 0, 0]);
    }

    #[test]
    fn no_first_time_weighs_more_than_the_cap_with_its_revenge() {
        let kinds = [
            KeyKind::GameQuest,
            KeyKind::SideQuest,
            KeyKind::ClassQuest,
            KeyKind::Subzone,
            KeyKind::Level,
            KeyKind::Talk,
            KeyKind::Kill,
            KeyKind::RaidKill,
            KeyKind::Death,
            KeyKind::Mark,
            KeyKind::Title,
            KeyKind::Mount,
            KeyKind::EpicMount,
            KeyKind::EpicItem,
            KeyKind::Upgrade,
            KeyKind::PvpRank,
            KeyKind::Dungeon,
            KeyKind::Raid,
            KeyKind::Battleground,
            KeyKind::BgWin,
        ];

        for kind in kinds {
            assert!(weight(RULE_ONE, kind) + REVENGE <= CAP_MAX, "{kind:?}");
        }
    }

    #[test]
    fn the_first_rule_closes_at_fifteen_and_must_close_at_forty() {
        assert_eq!(limits(RULE_ONE), Limits { min: 15, max: 40 });
    }
}
