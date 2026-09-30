use hourglass::Tick;
use timeways_story::pace::{BUSY_CALLS, Pace, WINDOW_SECONDS};

#[test]
fn a_quiet_window_is_not_tight() {
    let pace = Pace::default();

    assert!(!pace.is_tight(Tick(1000)));
}

#[test]
fn a_failure_makes_the_window_tight_until_the_window_has_passed() {
    let mut pace = Pace::default();

    pace.failed(Tick(1000));

    assert!(pace.is_tight(Tick(1000)));
    assert!(pace.is_tight(Tick(1000 + WINDOW_SECONDS - 1)));
    assert!(!pace.is_tight(Tick(1000 + WINDOW_SECONDS)));
}

#[test]
fn five_calls_in_one_window_make_it_tight() {
    let mut pace = Pace::default();

    for minute in 0..BUSY_CALLS as u64 {
        pace.opened(Tick(1000 + minute * 60));
    }

    assert!(pace.is_tight(Tick(1000 + WINDOW_SECONDS - 1)));
    assert!(!pace.is_tight(Tick(1000 + WINDOW_SECONDS)));
}

#[test]
fn four_calls_in_one_window_leave_it_calm() {
    let mut pace = Pace::default();

    for minute in 0..BUSY_CALLS as u64 - 1 {
        pace.opened(Tick(1000 + minute * 60));
    }

    assert!(!pace.is_tight(Tick(1300)));
}
