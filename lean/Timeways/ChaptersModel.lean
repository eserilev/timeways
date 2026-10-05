-- The pure model of the chapter fold (docs/plans/chapters.md 8), and
-- the proof that the Rust fold computes it. Each function of the model
-- gives back only the fields that it changes. So the laws in
-- `Chapters.lean` see at once what a part of the fold leaves alone.
import Timeways.Funs

set_option linter.unusedSimpArgs false

open Aeneas Aeneas.Std Result
open timeways_rules.weights

namespace timeways_rules.chapters

open alloc.vec

/-! ## Helpers -/

/-- `Vec::push`, with no change when the vector is full. The bridge
shows that the Rust push never meets a full vector. -/
def pushM {α : Type} (v : Vec α) (x : α) : Vec α :=
  if h : v.val.length < Usize.max then Vec.from (v.val ++ [x]) (by simp; omega) else v

theorem pushM_val {α : Type} (v : Vec α) (x : α) (h : v.val.length < Usize.max) :
    (pushM v x).val = v.val ++ [x] := by
  simp [pushM, h]

theorem pushM_val_prefix {α : Type} (v : Vec α) (x : α) :
    ∃ l, (pushM v x).val = v.val ++ l := by
  unfold pushM
  split
  · exact ⟨[x], by simp⟩
  · exact ⟨[], by simp⟩

theorem pushM_length_le {α : Type} (v : Vec α) (x : α) :
    (pushM v x).val.length ≤ v.val.length + 1 := by
  unfold pushM
  split <;> simp

theorem push_eq {α : Type} (v : Vec α) (x : α) (h : v.val.length < Usize.max) :
    Vec.push v x = ok (pushM v x) := by
  obtain ⟨y, hy, hv⟩ := WP.spec_imp_exists (Vec.push_spec v x h)
  rw [hy]
  congr 1
  apply Vec.ext
  rw [hv, pushM_val v x h]

theorem sat_add_val {ty : UScalarTy} (x y : UScalar ty) :
    (UScalar.saturating_add x y).val = min (UScalar.max ty) (x.val + y.val) := by
  simp only [UScalar.saturating_add, UScalar.val, BitVec.toNat_ofNat]
  apply Nat.mod_eq_of_lt
  have h1 : UScalar.max ty < 2 ^ ty.numBits := by
    simp only [UScalar.max]
    have := Nat.one_le_two_pow (n := ty.numBits)
    omega
  omega

theorem sat_sub_val {ty : UScalarTy} (x y : UScalar ty) :
    (UScalar.saturating_sub x y).val = x.val - y.val := by
  simp only [UScalar.saturating_sub, UScalar.val, BitVec.toNat_ofNat, Nat.zero_max]
  apply Nat.mod_eq_of_lt
  have := x.bv.isLt
  omega

theorem u8_sat_add (x y : U8) : core.num.U8.saturating_add x y = UScalar.saturating_add x y := rfl
theorem u16_sat_add (x y : U16) : core.num.U16.saturating_add x y = UScalar.saturating_add x y := rfl
theorem u32_sat_add (x y : U32) : core.num.U32.saturating_add x y = UScalar.saturating_add x y := rfl
theorem u64_sat_add (x y : U64) : core.num.U64.saturating_add x y = UScalar.saturating_add x y := rfl
theorem usize_sat_add (x y : Usize) :
    core.num.Usize.saturating_add x y = UScalar.saturating_add x y := rfl
theorem u16_sat_sub (x y : U16) : core.num.U16.saturating_sub x y = UScalar.saturating_sub x y := rfl
theorem usize_sat_sub (x y : Usize) :
    core.num.Usize.saturating_sub x y = UScalar.saturating_sub x y := rfl

/-! ## The model -/

/-- `has_slot`: the id names a slot, or the next one while the vector has room. -/
def hasSlotM (length id : Nat) : Bool :=
  decide (id < length) || (decide (id = length) && decide (length < Usize.max))

/-- The vector after `has_slot` allowed the id: one more slot when the id is
the next one. -/
def slotted {α : Type} (v : Vec α) (i : Usize) (x : α) : Vec α :=
  if i.val = v.val.length then pushM v x else v

/-- The value of `vec[i]`, or `d` out of range. -/
def getM {α : Type} (v : Vec α) (i : Usize) (d : α) : α := v.val.getD i.val d

def weightM : KeyKind → U16
  | .GameQuest => 1#u16
  | .SideQuest => 2#u16
  | .ClassQuest => 3#u16
  | .Subzone => 1#u16
  | .Level => 2#u16
  | .Talk => 1#u16
  | .Kill => 3#u16
  | .RaidKill => 5#u16
  | .Death => 0#u16
  | .Mark => 1#u16
  | .Title => 1#u16
  | .Mount => 3#u16
  | .EpicMount => 5#u16
  | .EpicItem => 3#u16
  | .Upgrade => 1#u16
  | .PvpRank => 2#u16
  | .Dungeon => 3#u16
  | .Raid => 5#u16
  | .Battleground => 3#u16
  | .BgWin => 3#u16

def deathWeightM (d : U8) : U16 :=
  if d.val = 0 then 2#u16 else if d.val = 1 then 1#u16 else 0#u16

def oneMoreDeathM (d : U8) : U8 :=
  if d.val < 2 then core.num.U8.saturating_add d 1#u8 else d

def firstTimeGainM (r : KeyRecord) (k : KeyKind) : U16 :=
  if r.seen then 0#u16 else weightM k

/-- `kill_gain`: the gain, the revenge, and the foes after. -/
def killGainM (foes : Vec FoeRecord) (r : KeyRecord) (key : Key) :
    (U16 × Bool) × Vec FoeRecord :=
  let base := firstTimeGainM r key.kind
  match key.foe with
  | none => ((base, false), foes)
  | some foe =>
    if hasSlotM foes.val.length foe.val then
      let v := slotted foes foe UNBEATEN
      let before := getM v foe UNBEATEN
      if r.seen then ((base, false), v.set foe { before with beaten := true })
      else if before.beaten then ((base, false), v.set foe before)
      else if before.deaths = 0#u8 then ((base, false), v.set foe { before with beaten := true })
      else ((core.num.U16.saturating_add base 2#u16, true), v.set foe { before with beaten := true })
    else ((base, false), foes)

/-- `death_gain`: the gain, the foes after, and the key record after. -/
def deathGainM (foes : Vec FoeRecord) (r : KeyRecord) (foe : Option Usize) :
    U16 × Vec FoeRecord × KeyRecord :=
  match foe with
  | none => (deathWeightM r.deaths, foes, { r with deaths := oneMoreDeathM r.deaths })
  | some foe =>
    if hasSlotM foes.val.length foe.val then
      let v := slotted foes foe UNBEATEN
      let before := getM v foe UNBEATEN
      if before.beaten then
        (0#u16, v.set foe { beaten := true, deaths := oneMoreDeathM before.deaths }, r)
      else
        (deathWeightM before.deaths,
          v.set foe { beaten := false, deaths := oneMoreDeathM before.deaths }, r)
    else (0#u16, foes, r)

/-- The raw gain of a key, its revenge, the foes after, and the key record
after, before the cap. -/
def rawGainM (foes : Vec FoeRecord) (r : KeyRecord) (key : Key) :
    U16 × Bool × Vec FoeRecord × KeyRecord :=
  match key.kind with
  | .Death =>
    let (g, f, r1) := deathGainM foes r key.foe
    (g, false, f, r1)
  | .Kill =>
    let ((g, rv), f) := killGainM foes r key
    (g, rv, f, r)
  | .RaidKill =>
    let ((g, rv), f) := killGainM foes r key
    (g, rv, f, r)
  | k => (firstTimeGainM r k, false, foes, r)

/-- `gain_of`: the gain, the revenge, the keys after, and the foes after. -/
def gainM (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key) :
    (U16 × Bool) × Vec KeyRecord × Vec FoeRecord :=
  match key with
  | none => ((0#u16, false), keys, foes)
  | some key =>
    if hasSlotM keys.val.length key.id.val then
      let v := slotted keys key.id UNSEEN
      let record := getM v key.id UNSEEN
      let (raw, revenge, foes1, record1) := rawGainM foes record key
      let room := core.num.U16.saturating_sub CAP_MAX record1.gain
      let amount := if raw < room then raw else room
      ((amount, revenge),
        v.set key.id { record1 with seen := true,
                                    gain := core.num.U16.saturating_add record1.gain amount },
        foes1)
    else ((0#u16, false), keys, foes)

/-- An empty tale, the default of a read out of range. -/
def NO_TALE : Tale :=
  { «instance» := 0#usize, first := 0#usize, weight := 0#u32, runs := 0#u32 }

/-- `close_visit`: the tales and the closed visits after. -/
def closeVisitM (tales : Vec Tale) (visits : Vec Visit) (v : Visit) : Vec Tale × Vec Visit :=
  if v.tale.val < tales.val.length then
    let t := getM tales v.tale NO_TALE
    (tales.set v.tale { t with weight := core.num.U32.saturating_add t.weight v.gain,
                               runs := core.num.U32.saturating_add t.runs 1#u32 },
      pushM visits v)
  else (tales, pushM visits v)

def RUN_GAP : U64 := 1800#u64
def AWAY : U64 := 28800#u64

/-- `leave_instance`: the tales, the closed visits, and the open visit after. -/
def leaveM (tales : Vec Tale) (visits : Vec Visit) (visit : Option Visit) (now : U64) :
    Vec Tale × Vec Visit × Option Visit :=
  match visit with
  | none => (tales, visits, none)
  | some v =>
    if (core.num.U64.saturating_add v.left_at RUN_GAP).val ≤ now.val then
      let (t, vs) := closeVisitM tales visits v
      (t, vs, none)
    else (tales, visits, visit)

/-- The index of the first tale of the instance, from index `i` on. -/
def findFrom (l : List Tale) (inst : Usize) (i : Nat) : Option Nat :=
  if h : i < l.length then
    if l[i].instance = inst then some i else findFrom l inst (i + 1)
  else none
termination_by l.length - i

/-- A `Usize` from a number. -/
def usizeOf (n : Nat) : Usize := ⟨BitVec.ofNat _ n⟩

/-- `tale_of` from index `i` on: the index of the tale, and the tales after. -/
def taleFromM (tales : Vec Tale) (inst here : Usize) (i : Nat) : Usize × Vec Tale :=
  match findFrom tales.val inst i with
  | some j => (usizeOf j, tales)
  | none =>
    (usizeOf tales.val.length,
      pushM tales { «instance» := inst, first := here, weight := 0#u32, runs := 0#u32 })

/-- `tale_of`: the index of the tale, and the tales after. -/
def taleOfM (tales : Vec Tale) (inst here : Usize) : Usize × Vec Tale :=
  taleFromM tales inst here 0

/-- `enter_instance`: the tales, the closed visits, and the open visit after. -/
def enterM (tales : Vec Tale) (visits : Vec Visit) (visit : Option Visit)
    (inst here : Usize) (now : U64) : Vec Tale × Vec Visit × Option Visit :=
  let (tale, tales1) := taleOfM tales inst here
  let fresh : Visit := { tale, first := here, last := here, left_at := now, gain := 0#u32 }
  match visit with
  | none => (tales1, visits, some fresh)
  | some v =>
    if v.tale = tale ∧ now.val < (core.num.U64.saturating_add v.left_at RUN_GAP).val then
      (tales1, visits, some { v with last := here, left_at := now })
    else
      let (t2, vs2) := closeVisitM tales1 visits v
      (t2, vs2, some fresh)

def addVisitM (visit : Option Visit) (amount : U16) : Option Visit :=
  match visit with
  | none => none
  | some v => some { v with gain := core.num.U32.saturating_add v.gain (UScalar.cast .U32 amount) }

def rankM : Break → Nat
  | .Return => 0
  | .NewZone => 1
  | .Level => 2
  | .Capital => 3
  | .Inn => 4
  | .Away => 5

/-- `wait_for_cut`: the break that waits after. -/
def waitM (pending : Option Break) (new : Break) : Option Break :=
  match pending with
  | none => some new
  | some old => if rankM old ≤ rankM new then some old else some new

/-- The waiting break after one more break, if any. -/
def withBreak (pending : Option Break) : Option Break → Option Break
  | none => pending
  | some c => waitM pending c

/-- `zone_break`: the break of the zone, and the zones after. -/
def zoneBreakM (zones : Vec ZoneRecord) (closedLen : Nat) (zone : Usize) : Option Break :=
  if zones.val.length ≤ zone.val then none
  else
    let r := getM zones zone UNSETTLED
    if r.settled then
      if (core.num.Usize.saturating_add r.last_chapter 1#usize).val < closedLen then some .Return
      else none
    else some .NewZone

/-- `note_zone`: each zone gets its record at its first step. -/
def noteZoneM (zones : Vec ZoneRecord) (zone : Usize) : Vec ZoneRecord :=
  if zone.val = zones.val.length ∧ zones.val.length < Usize.max then pushM zones UNSETTLED
  else zones

def settleM (zones : Vec ZoneRecord) (closed : Usize) (zone : Usize) : Vec ZoneRecord :=
  if zone.val < zones.val.length then zones.set zone { settled := true, last_chapter := closed }
  else zones

/-- The least weight at which a chapter closes at a break, in rule 1. -/
def MIN : Nat := 15
/-- The weight at which a chapter closes with no break, in rule 1. -/
def MAX : Nat := 40

/-- `add_to_chapter`: the zones, the closed chapters, the open chapter, and
the waiting break after. -/
def addChapterM (zones : Vec ZoneRecord) (closed : Vec ClosedChapter) (opn : Chapter)
    (pending : Option Break) (zone : Usize) (amount : U16) (here : Usize) :
    Vec ZoneRecord × Vec ClosedChapter × Chapter × Option Break :=
  if amount.val = 0 then (zones, closed, opn, pending) else
  let o := zoneBreakM zones closed.val.length zone
  let pending1 := withBreak pending o
  let (closed2, open2) : Vec ClosedChapter × Chapter := match pending1 with
    | none => (closed, opn)
    | some cut =>
      if MIN ≤ opn.weight.val ∧ opn.first.val < here.val then
        (pushM closed { chapter := opn, last := core.num.Usize.saturating_sub here 1#usize,
                        close := .Break },
          { first := here, weight := 0#u16, opening := .Break cut, zone := some zone,
            rule := opn.rule })
      else (closed, opn)
  let open3 : Chapter := { open2 with
    weight := core.num.U16.saturating_add open2.weight amount,
    zone := if open2.zone.isNone then some zone else open2.zone }
  let zones2 := settleM zones (Vec.len closed2) zone
  if MAX ≤ open3.weight.val then
    (zones2, pushM closed2 { chapter := open3, last := here, close := .Max },
      { open3 with first := core.num.Usize.saturating_add here 1#usize, weight := 0#u16,
                   opening := .Max, zone := some zone }, none)
  else (zones2, closed2, open3, none)

def ZERO_GAIN : Gain := { amount := 0#u16, track := .World, revenge := false }

def applyRuleM (st : Fold) (rule : U8) : Fold :=
  let here := Vec.len st.gains
  if st.«open».first.val < here.val then
    { st with
      closed := pushM st.closed (ClosedChapter.mk st.«open»
        (core.num.Usize.saturating_sub here 1#usize) .Rule),
      «open» := { first := here, weight := 0#u16, opening := .Rule, zone := none, rule },
      pending := none,
      gains := pushM st.gains ZERO_GAIN }
  else
    { st with «open» := { st.«open» with rule }, pending := none,
              gains := pushM st.gains ZERO_GAIN }

/-- The waiting break after the time away and the mark of the step. -/
def pendingOfM (st : Fold) (play : Play) : Option Break :=
  let p1 := match st.last_at with
    | none => st.pending
    | some last =>
      if (core.num.U64.saturating_add last AWAY).val ≤ play.«at».val then waitM st.pending .Away
      else st.pending
  match play.mark with
  | none => p1
  | some m => waitM p1 m

/-- The visit part of a step. -/
def visitsOfM (st : Fold) (play : Play) : Vec Tale × Vec Visit × Option Visit :=
  match play.track with
  | .World => leaveM st.tales st.visits st.visit play.«at»
  | .Instance i => enterM st.tales st.visits st.visit i (Vec.len st.gains) play.«at»

def applyPlayM (st : Fold) (play : Play) : Fold :=
  let here := Vec.len st.gains
  let p2 := pendingOfM st play
  let (tales1, visits1, visit1) := visitsOfM st play
  let ((amount, revenge), keys1, foes1) := gainM st.keys st.foes play.key
  match play.track with
  | .World =>
    let (zones2, closed2, open2, p3) :=
      addChapterM (noteZoneM st.zones play.zone) st.closed st.«open» p2 play.zone amount here
    { keys := keys1, foes := foes1, zones := zones2, closed := closed2, «open» := open2,
      pending := p3, tales := tales1, visits := visits1, visit := visit1,
      last_at := some play.«at»,
      gains := pushM st.gains { amount, track := .World, revenge } }
  | .Instance i =>
    { keys := keys1, foes := foes1, zones := noteZoneM st.zones play.zone, closed := st.closed,
      «open» := st.«open», pending := p2, tales := tales1, visits := visits1,
      visit := addVisitM visit1 amount, last_at := some play.«at»,
      gains := pushM st.gains { amount, track := .Instance i, revenge } }

def applyM (st : Fold) : Step → Fold
  | .Play play => applyPlayM st play
  | .Rule rule => applyRuleM st rule

def startM : Fold :=
  { keys := Vec.new _, foes := Vec.new _, zones := Vec.new _, closed := Vec.new _,
    «open» := { first := 0#usize, weight := 0#u16, opening := .First, zone := none,
                rule := 1#u8 },
    pending := none, tales := Vec.new _, visits := Vec.new _, visit := none,
    last_at := none, gains := Vec.new _ }

/-- The fold of a whole list of steps, from the start. -/
def runM (ss : List Step) : Fold := ss.foldl applyM startM

end timeways_rules.chapters
