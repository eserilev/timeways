//! The small rules of Timeways that Lean proves (lean/README.md). Aeneas translates this
//! crate to Lean, and the proofs read that translation, so a proof always speaks about the
//! real code.
//!
//! Aeneas translates only a part of Rust: no closure, no iterator adapter, and no `?` on an
//! `Option`. So the loops here walk by index. That is an approved exception to the
//! readability rules of CLAUDE.md, and each such function says so. The story program keeps
//! its own types, and turns them into these and back.

// The mark `verify::start_from` names a root of the translation. It exists only under
// `--cfg charon`, so a normal build never sees it.
#![cfg_attr(charon, feature(register_tool), register_tool(verify))]
// Their fixes bring a closure, a range, `?` on an `Option`, or `vec!`, and Aeneas
// translates none of them.
#![allow(
    clippy::manual_map,
    clippy::manual_range_contains,
    clippy::question_mark,
    clippy::vec_init_then_push
)]

pub mod aliases;
pub mod budget;
pub mod chapters;
pub mod entry_edits;
pub mod hero_hook;
pub mod narrator_shapes;
pub mod outcomes;
pub mod prompts;
pub mod quest_log;
pub mod story_shelf;
pub mod thin_lore;
pub mod trust;
pub mod weights;
