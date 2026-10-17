//! The fake `date` of `addon/tests/wow.lua` gives the same text on every thread. The tests
//! run many Lua states at once, and Lua 5.1 keeps the time of `os.date` in one static
//! struct of the C library.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

const DAY: i64 = 86_400;
const THREADS: i64 = 16;
const CALLS: usize = 2_000;

#[test]
fn date_gives_its_own_day_on_many_threads_at_once() {
    let threads: Vec<_> = (0..THREADS)
        .map(|day| {
            std::thread::spawn(move || {
                let game = Game::without_addon();
                let expected = format!("1970-01-{:02}", day + 1);
                for _ in 0..CALLS {
                    let shown: String = game.eval(&format!("date('%Y-%m-%d', {})", day * DAY));
                    assert_eq!(shown, expected);
                }
            })
        })
        .collect();

    for thread in threads {
        thread.join().unwrap();
    }
}
