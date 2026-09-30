use hourglass::Tick;
use timeways_story::chapters::{SESSION_GAP_SECONDS, sessions};

#[test]
fn a_stretch_of_thirty_minutes_with_no_event_ends_a_session() {
    let ticks = [Tick(100), Tick(100 + SESSION_GAP_SECONDS)];

    let found = sessions(ticks.into_iter());

    assert_eq!(found.len(), 2);
}

#[test]
fn a_stretch_just_under_thirty_minutes_keeps_the_session() {
    let ticks = [Tick(100), Tick(100 + SESSION_GAP_SECONDS - 1)];

    let found = sessions(ticks.into_iter());

    assert_eq!(
        found,
        vec![(Tick(100), Tick(100 + SESSION_GAP_SECONDS - 1))]
    );
}
