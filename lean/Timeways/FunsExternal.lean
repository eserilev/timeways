-- The external model: the std items that the extracted code calls.
--
-- Aeneas writes these items as opaque axioms, with no body. A proof
-- cannot see inside an axiom, so this file gives each item a body.
-- Each body states the Rust semantics of the item. This file is the
-- trusted part of the proofs: read it before you trust a theorem.
-- extract.sh never overwrites it, and it writes the new Aeneas
-- template beside it as FunsExternal_Template.lean for a diff.
module
public import Aeneas
public import Timeways.Types
@[expose] public section
open Aeneas Aeneas.Std Result ControlFlow Error
set_option linter.dupNamespace false
set_option linter.style.setOption false
set_option linter.style.longLine false

open timeways_rules

/-- `Option::clone`: `None` stays `None`, and `Some` clones its value.
    That is the derive in core. -/
@[rust_fun
  "core::option::{core::clone::Clone<core::option::Option<@T>>}::clone"]
def core.option.Option.Insts.CoreCloneClone.clone
  {T : Type} (cloneCloneInst : core.clone.Clone T) :
  Option T → Result (Option T)
  | none => ok none
  | some x => do
    let y ← cloneCloneInst.clone x
    ok (some y)

/-- `String == String` compares the contents. -/
@[rust_fun
  "alloc::string::{core::cmp::PartialEq<alloc::string::String, alloc::string::String>}::eq"]
def alloc.string.String.Insts.CoreCmpPartialEqString.eq (a b : String) : Result Bool :=
  ok (decide (a = b))

/-- `String::clone` gives an equal string. -/
@[rust_fun "alloc::string::{core::clone::Clone<alloc::string::String>}::clone"]
def alloc.string.String.Insts.CoreCloneClone.clone (s : String) : Result String :=
  ok s
