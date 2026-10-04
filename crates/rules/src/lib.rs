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
// Their fixes bring a closure or a range, and Aeneas translates neither.
#![allow(clippy::manual_map, clippy::manual_range_contains)]

pub mod quest_log;
