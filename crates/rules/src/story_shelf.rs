//! The shelf of accepted stories (GAMEPLAY.md 4.8): which line of the `stories` table may
//! land, and which stories stand. A number comes once, a removed story never comes back,
//! and a story that a call used stays.
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

/// One line of the `stories` table, as the rules read it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShelfLine {
    Accepted { number: u64 },
    Removed { number: u64 },
}

/// True when a line accepted this number. An index loop.
#[must_use]
pub fn is_taken(lines: &[ShelfLine], number: u64) -> bool {
    let mut index = 0;
    while index < lines.len() {
        // Aeneas matches no slot of an array, so the slot goes to a local first.
        let line = lines[index];
        if let ShelfLine::Accepted { number: n } = line
            && n == number
        {
            return true;
        }
        index += 1;
    }
    false
}

/// True when a line removed this number. An index loop.
#[must_use]
pub fn is_removed(lines: &[ShelfLine], number: u64) -> bool {
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let ShelfLine::Removed { number: n } = line
            && n == number
        {
            return true;
        }
        index += 1;
    }
    false
}

/// An index loop, because Aeneas translates no `contains`.
fn holds(numbers: &[u64], number: u64) -> bool {
    let mut index = 0;
    while index < numbers.len() {
        if numbers[index] == number {
            return true;
        }
        index += 1;
    }
    false
}

/// True when the line may land on the shelf. `used` holds the numbers that an accepted
/// call read.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn lands(lines: &[ShelfLine], line: ShelfLine, used: &[u64]) -> bool {
    match line {
        ShelfLine::Accepted { number } => !is_taken(lines, number),
        ShelfLine::Removed { number } => {
            is_taken(lines, number) && !is_removed(lines, number) && !holds(used, number)
        }
    }
}

/// The numbers that stand, oldest first. An index loop.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn standing(lines: &[ShelfLine]) -> Vec<u64> {
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let ShelfLine::Accepted { number } = line
            && !is_removed(lines, number)
        {
            numbers.push(number);
        }
        index += 1;
    }
    numbers
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCEPT_7: ShelfLine = ShelfLine::Accepted { number: 7 };
    const REMOVE_7: ShelfLine = ShelfLine::Removed { number: 7 };

    #[test]
    fn a_number_comes_once() {
        let lines = [ACCEPT_7];

        assert!(lands(&[], ACCEPT_7, &[]));
        assert!(!lands(&lines, ACCEPT_7, &[]));
    }

    #[test]
    fn a_removed_story_never_comes_back() {
        let lines = [ACCEPT_7, REMOVE_7];

        assert!(!lands(&lines, ACCEPT_7, &[]));
        assert!(!lands(&lines, REMOVE_7, &[]));
        assert_eq!(standing(&lines), Vec::<u64>::new());
    }

    #[test]
    fn a_used_story_cannot_be_removed() {
        let lines = [ACCEPT_7];

        assert!(!lands(&lines, REMOVE_7, &[7]));
        assert!(lands(&lines, REMOVE_7, &[8]));
    }

    #[test]
    fn a_story_that_never_came_cannot_be_removed() {
        assert!(!lands(&[], REMOVE_7, &[]));
    }

    #[test]
    fn the_stories_that_stand_come_oldest_first() {
        let lines = [
            ShelfLine::Accepted { number: 9 },
            ACCEPT_7,
            ShelfLine::Accepted { number: 3 },
            REMOVE_7,
        ];

        assert_eq!(standing(&lines), vec![9, 3]);
    }
}
