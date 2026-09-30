//! The bridge refuses an addon version out of its range (GAMEPLAY.md 5.12), and setup
//! refuses a release whose manifest names one. So a release must fit the pinned bridge.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use protocol::apps::App;
use protocol::version::{VersionFit, version_fit};

#[test]
fn the_addon_version_is_in_the_range_of_the_pinned_bridge() {
    let game = Game::new();

    let version: u32 = game.eval("return ns.App.version");

    assert_eq!(version_fit(App::Timeways, version), VersionFit::Supported);
}
