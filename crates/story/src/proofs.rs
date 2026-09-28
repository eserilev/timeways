//! Proofs for every input, checked by Kani (`scripts/kani.sh`). Each one covers arithmetic
//! that a test samples but cannot exhaust.

use crate::character::next_trust;
use crate::narrator::Budget;
use crate::vocabulary::TRUST;
use hourglass::Tick;

#[kani::proof]
fn trust_stays_in_its_band_for_any_change() {
    let held: Option<i64> = kani::any();
    let by: i64 = kani::any();

    let to = next_trust(held, by);

    assert!(TRUST.holds(to));
}

#[kani::proof]
fn a_change_of_trust_never_moves_against_its_sign() {
    let held: i64 = kani::any();
    kani::assume(TRUST.holds(held));
    let by: i64 = kani::any();

    let to = next_trust(Some(held), by);

    assert!(by < 0 || to >= held);
    assert!(by > 0 || to <= held);
}

/// Game time only moves forward. Any start, and steps of up to 18 hours, cover every way
/// that four lines fall around one hour.
#[kani::proof]
#[kani::unwind(6)]
fn the_narrator_never_speaks_four_times_in_one_hour() {
    let mut budget = Budget::default();
    let first: u64 = kani::any();
    kani::assume(first <= u64::MAX - 4 * u64::from(u16::MAX));
    let mut at = first;
    let mut taken = 0;
    for step in 0..4 {
        if step > 0 {
            at += u64::from(kani::any::<u16>());
        }
        if budget.take(Tick(at)) {
            taken += 1;
        }
    }

    assert!(taken < 4 || at - first >= 3600);
}
