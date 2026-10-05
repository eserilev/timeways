//! The journal of a character: the chronicle, the places, the people, the deeds, the hero,
//! what you learned, and your side quests. No model takes part.

use crate::chapters::{Book, ChapterSpan, SpanState, TaleSpan, VisitSpan};
use crate::character::{Character, title_of_game_quest, title_of_mark};
use crate::entry_edits::EditView;
use crate::gear::title_of_item;
use crate::hero::{Entry, Field, Hero};
use crate::learned::Learned;
use crate::mounts::title_of_mount;
use crate::places::{self, PlaceKind};
use crate::quest::{QuestView, title_of_thing};
use crate::reply_size::{MAX_LINE, MAX_SLOT, Size};
use crate::spot::{Spot, spot_of};
use crate::stories::PlayerStory;
use crate::story::why::TrustWhy;
use crate::vocabulary::{BG_WON, PVP_RANK};
use crate::vocabulary::{
    CLASS_QUEST, DEATHS, DEFEATED, FIRST_EPIC_ITEM, FIRST_EPIC_MOUNT, FIRST_MOUNT, GAME_QUEST_DONE,
    LEVEL, MARK_OF, MARKED_BY, MET, QUEST_DONE, SLAPPED, TITLE, TRUSTS, UPGRADED, VISITED,
};
use hourglass::{EntityId, EventId, EventKind, LOCATED_IN, Tick, World};
use serde::Serialize;
use timeways_rules::chapters::{Break, Opening};

/// The bridge takes at most 200 items in one list (Gnomish Relay SPEC.md 9.8).
const PAGE_LIST_ITEMS: usize = 200;

/// Room for the type, the id, the page numbers, and the empty lists of a page line. In the
/// slot, also room for the record and the note of the bridge.
const FRAME: Size = Size {
    line: 320,
    slot: 2112,
};

/// The entries of each list of one chapter. With names of at most
/// `story::MAX_NAME_BYTES`, a chapter always fits on one page.
const CHAPTER_LIST: usize = 20;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Journal {
    /// The sheet and the entries of the hero are lists like the others.
    pub hero: Hero,
    /// Why the last edit of the hero did not stand. It shows once.
    /// A box keeps the page small, as for `talk_quest`.
    pub hero_refused: Option<Box<str>>,
    pub chapters: Vec<Chapter>,
    /// One tale for each dungeon, raid, and battleground (docs/plans/chapters.md 6).
    pub tales: Vec<Tale>,
    pub places: Vec<Place>,
    pub people: Vec<Person>,
    pub deeds: Vec<Deed>,
    /// What you read and heard (GAMEPLAY.md 3.1.1).
    pub learned: Vec<Learned>,
    /// The side quests that you did not decline (3.4).
    pub quests: Vec<QuestView>,
    /// The stories that players told about you, and that you accepted (4.8).
    pub stories: Vec<PlayerStory>,
    /// "Your history here" of each zone, the newest one of each (docs/plans/chapters.md 10).
    pub histories: Vec<History>,
    /// The standing edit of each entry that shows words of the player (11). Each is its
    /// own item, so a full chapter and a full edit never have to share a page.
    pub edits: Vec<EditView>,
    /// Why the last edit of an entry did not stand. It shows once.
    pub edit_refused: Option<Box<str>>,
    /// Who the character has become, for the title page of the Chronicle
    /// (docs/plans/hero-stories.md 3.5). Only the first page carries it.
    pub summary: Option<Box<str>>,
    /// The quest that the newest talk with work asked for (3.5). The talk window shows it.
    /// A box keeps the page small, and every answer holds a page.
    pub talk_quest: Option<Box<TalkQuest>>,
}

/// The quest that a talk asked for, as the talk window shows it (GAMEPLAY.md 3.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TalkQuest {
    pub npc: String,
    /// The time of the talk. The window shows only a quest of its own talk.
    pub at: Tick,
    #[serde(flatten)]
    pub state: TalkQuestState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TalkQuestState {
    /// The model writes the quest now.
    Writing,
    /// The offer, by its number in the quest log.
    Offered { number: u64 },
    /// No quest, and why, in the words of `/quest`.
    Refused { line: String },
}

/// One chapter of the chronicle, as the fold cut it (docs/plans/chapters.md 5), with no
/// model: what was new in it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Chapter {
    pub number: usize,
    /// The event of its first step: the key of its saga and of its edits.
    pub first: u64,
    pub began: Tick,
    /// The tick of the last event of the chapter.
    pub ended: Tick,
    /// The title from the code: the zone where it opened, a return, or a continuation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub opened_by: OpenedBy,
    pub state: EntryState,
    /// The level at its start and at its end, once the world knows one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<[i64; 2]>,
    pub zones: Vec<String>,
    pub people: Vec<String>,
    pub deeds: Vec<Deed>,
    /// The tally lines of repeats: "Defeated Hogger again, 6 times."
    pub again: Vec<String>,
    /// The entries past the first 20 of each list. The other pages of the journal hold
    /// them all.
    pub left_out: usize,
    /// The saga of the chapter, once a model wrote it (3.3).
    pub prose: Option<String>,
    /// The footnotes of the saga: small moments of the chapter (5.4.1).
    pub footnotes: Vec<String>,
}

/// Why a chapter began. The contents of the addon mark it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenedBy {
    #[default]
    First,
    Return,
    NewZone,
    Level,
    Capital,
    Inn,
    Away,
    /// The chapter before it reached the most weight.
    Max,
    Rule,
}

/// A closed entry never changes. An open one still grows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Open,
    #[default]
    Closed,
}

/// The one tale of a dungeon, a raid, or a battleground (docs/plans/chapters.md 6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tale {
    /// The event of its first step: the key of its texts and of its edits.
    pub first: u64,
    /// The first event of the chapter that it follows in the book.
    pub chapter: u64,
    pub instance: String,
    pub kind: PlaceKind,
    pub began: Tick,
    /// The closed visits: "Molten Core, 7 runs."
    pub runs: u32,
    /// The first kills, deaths, and quests of its visits.
    pub deeds: Vec<Deed>,
    pub again: Vec<String>,
    pub left_out: usize,
    /// The newest text of the narrator, once a model wrote one.
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Place {
    pub name: String,
    pub kind: PlaceKind,
    /// The place around it: the zone of a subzone. None for a zone.
    pub within: Option<String>,
    pub first_visit: Tick,
    /// Where you stood on the map at the first visit that had a position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<Spot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Person {
    pub name: String,
    pub place: Option<String>,
    pub first_met: Tick,
    /// How much this NPC trusts you, from -100 to 100, once anything changed it.
    pub trust: Option<i64>,
    pub slapped: Option<i64>,
    /// Where you stood on the map at the first meeting that had a position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<Spot>,
    /// The cause of the newest change of `trust`. The world alone does not know it, so
    /// the story program adds it from the database (GAMEPLAY.md 5.14).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_why: Option<TrustWhy>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Deed {
    /// `from` is None for the first level that the world saw, at the first login.
    Level {
        from: Option<i64>,
        to: i64,
        at: Tick,
        place: Option<String>,
    },
    /// `times` is 1 for the true kill, and more for each echo after a reset (5.13).
    Defeated {
        foe: String,
        times: i64,
        at: Tick,
        place: Option<String>,
    },
    /// A joke title of the journal (5.4.1).
    Titled {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A side quest that you finished (3.4).
    QuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A quest of the game that you turned in.
    GameQuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A quest of the game that only your class gets.
    ClassQuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A quest of the game left a lasting buff or debuff on you.
    QuestMarked {
        mark: String,
        quest: String,
        at: Tick,
        place: Option<String>,
    },
    /// The first mount that you rode, or with `epic`, the first epic mount.
    Mounted {
        mount: String,
        epic: bool,
        at: Tick,
        place: Option<String>,
    },
    /// The first item of epic quality that you put on.
    EpicItem {
        item: String,
        at: Tick,
        place: Option<String>,
    },
    /// An item far better than what its slot held.
    Upgraded {
        item: String,
        at: Tick,
        place: Option<String>,
    },
    /// The first win in a battleground.
    WonBattle {
        battleground: String,
        at: Tick,
        place: Option<String>,
    },
    /// A new rank in battle against players.
    PvpRank {
        rank: i64,
        at: Tick,
        place: Option<String>,
    },
    /// `killer` is None when the addon did not know it, or when it was a player.
    Died {
        killer: Option<String>,
        at: Tick,
        place: Option<String>,
    },
}

/// "Your history here": what the chronicle tells of you in one zone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct History {
    pub zone: String,
    pub text: String,
}

/// One reply of the journal. The addon joins the lists of pages 0 to `pages - 1`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Page {
    pub page: usize,
    pub pages: usize,
    #[serde(flatten)]
    pub journal: Journal,
}

/// Cuts the journal into pages that each fit in one reply, in the order of the lists.
/// There is always at least one page, so an empty journal still answers.
#[must_use]
pub fn pages(journal: Journal) -> Vec<Page> {
    let mut pages = Vec::new();
    let mut current = Journal {
        hero_refused: journal.hero_refused,
        edit_refused: journal.edit_refused,
        talk_quest: journal.talk_quest,
        summary: journal.summary,
        ..Journal::default()
    };
    let mut used = Size::of(&current.hero_refused)
        .plus(Size::of(&current.edit_refused))
        .plus(Size::of(&current.talk_quest))
        .plus(Size::of(&current.summary));
    let budget = Size {
        line: MAX_LINE - FRAME.line,
        slot: MAX_SLOT - FRAME.slot,
    };
    // In the order of the lists, so each list stays in order across the pages.
    let mut items: Vec<Item> = journal.hero.sheet.into_iter().map(Item::Field).collect();
    items.extend(journal.hero.entries.into_iter().map(Item::Entry));
    items.extend(journal.chapters.into_iter().map(Item::Chapter));
    items.extend(journal.tales.into_iter().map(Item::Tale));
    items.extend(journal.histories.into_iter().map(Item::History));
    items.extend(journal.edits.into_iter().map(Item::Edit));
    items.extend(journal.places.into_iter().map(Item::Place));
    items.extend(journal.people.into_iter().map(Item::Person));
    items.extend(journal.deeds.into_iter().map(Item::Deed));
    items.extend(journal.learned.into_iter().map(Item::Learned));
    items.extend(journal.quests.into_iter().map(Item::Quest));
    items.extend(journal.stories.into_iter().map(Item::Story));
    // The comma after an item.
    let comma = Size { line: 1, slot: 1 };
    for item in items {
        let size = item.size().plus(comma);
        let list_full = item.list_len(&current) >= PAGE_LIST_ITEMS;
        let page_full = !used.plus(size).fits(budget);
        if (page_full || list_full) && used != Size::default() {
            pages.push(std::mem::take(&mut current));
            used = Size::default();
        }
        used = used.plus(size);
        item.add_to(&mut current);
    }
    pages.push(current);
    let count = pages.len();
    pages
        .into_iter()
        .enumerate()
        .map(|(page, journal)| Page {
            page,
            pages: count,
            journal,
        })
        .collect()
}

enum Item {
    Field(Field),
    Entry(Entry),
    Chapter(Chapter),
    Tale(Tale),
    History(History),
    Edit(EditView),
    Place(Place),
    Person(Person),
    Deed(Deed),
    Learned(Learned),
    Quest(QuestView),
    Story(PlayerStory),
}

impl Item {
    fn size(&self) -> Size {
        match self {
            Item::Field(field) => Size::of(field),
            Item::Entry(entry) => Size::of(entry),
            Item::Chapter(chapter) => Size::of(chapter),
            Item::Tale(tale) => Size::of(tale),
            Item::History(history) => Size::of(history),
            Item::Edit(edit) => Size::of(edit),
            Item::Place(place) => Size::of(place),
            Item::Person(person) => Size::of(person),
            Item::Deed(deed) => Size::of(deed),
            Item::Learned(learned) => Size::of(learned),
            Item::Quest(quest) => Size::of(quest),
            Item::Story(story) => Size::of(story),
        }
    }

    /// The length of the list of `journal` that this item goes into.
    fn list_len(&self, journal: &Journal) -> usize {
        match self {
            Item::Field(_) => journal.hero.sheet.len(),
            Item::Entry(_) => journal.hero.entries.len(),
            Item::Chapter(_) => journal.chapters.len(),
            Item::Tale(_) => journal.tales.len(),
            Item::History(_) => journal.histories.len(),
            Item::Edit(_) => journal.edits.len(),
            Item::Place(_) => journal.places.len(),
            Item::Person(_) => journal.people.len(),
            Item::Deed(_) => journal.deeds.len(),
            Item::Learned(_) => journal.learned.len(),
            Item::Quest(_) => journal.quests.len(),
            Item::Story(_) => journal.stories.len(),
        }
    }

    fn add_to(self, journal: &mut Journal) {
        match self {
            Item::Field(field) => journal.hero.sheet.push(field),
            Item::Entry(entry) => journal.hero.entries.push(entry),
            Item::Chapter(chapter) => journal.chapters.push(chapter),
            Item::Tale(tale) => journal.tales.push(tale),
            Item::History(history) => journal.histories.push(history),
            Item::Edit(edit) => journal.edits.push(edit),
            Item::Place(place) => journal.places.push(place),
            Item::Person(person) => journal.people.push(person),
            Item::Deed(deed) => journal.deeds.push(deed),
            Item::Learned(learned) => journal.learned.push(learned),
            Item::Quest(quest) => journal.quests.push(quest),
            Item::Story(story) => journal.stories.push(story),
        }
    }
}

/// Each list is in the order of the history, oldest first. It folds the whole history
/// under the first rule. The story program keeps its book and calls `journal_of`.
#[must_use]
pub fn journal(character: &Character) -> Journal {
    journal_of(character, &Book::of(character))
}

/// The journal with the chapters and the tales of `book`.
#[must_use]
pub fn journal_of(character: &Character, book: &Book) -> Journal {
    let world = character.world();
    let you = character.you();
    let visits = first_links(world, you, VISITED);
    let places: Vec<Place> = visits
        .iter()
        .map(|link| Place {
            name: name_of(world, link.target),
            kind: places::kind_of(world, link.target),
            within: world
                .location_of(link.target)
                .map(|zone| name_of(world, zone)),
            first_visit: link.tick,
            spot: spot_of(world, link.target),
        })
        .collect();
    let meetings = first_links(world, you, MET);
    let people: Vec<Person> = meetings
        .iter()
        .map(|link| Person {
            name: name_of(world, link.target),
            place: world
                .location_of(link.target)
                .map(|place| name_of(world, place)),
            first_met: link.tick,
            trust: fact_value(world, link.target, TRUSTS, you),
            slapped: fact_value(world, you, SLAPPED, link.target),
            spot: spot_of(world, link.target),
            trust_why: None,
        })
        .collect();
    let deed_rows = deeds_with_events(world, you);
    let facts = Facts {
        people: people
            .iter()
            .zip(&meetings)
            .map(|(person, link)| (person, link.event))
            .collect(),
        deeds: &deed_rows,
    };
    let chapters = chapters(world, book, &facts);
    let tales = tales(world, book, &facts, &chapters);
    let deeds = deed_rows.into_iter().map(|row| row.deed).collect();
    Journal {
        chapters,
        tales,
        places,
        people,
        deeds,
        ..Journal::default()
    }
}

/// The facts of the journal, each with the event that made it.
struct Facts<'a> {
    people: Vec<(&'a Person, EventId)>,
    deeds: &'a [DeedRow],
}

/// The event is in the range of the chapter, on either track.
fn in_span(book: &Book, span: &ChapterSpan, event: EventId) -> bool {
    book.step_of(event)
        .is_some_and(|step| span.steps.contains(&step))
}

/// The event is in the range of the chapter, in the open world.
fn in_world_of(book: &Book, span: &ChapterSpan, event: EventId) -> bool {
    book.step_of(event)
        .is_some_and(|step| span.steps.contains(&step) && book.is_world_step(step))
}

/// The event is a step of a visit of the tale, in its instance.
fn in_tale(book: &Book, tale: &TaleSpan, event: EventId) -> bool {
    book.step_of(event).is_some_and(|step| {
        !book.is_world_step(step) && tale.visits.iter().any(|visit| visit.steps.contains(&step))
    })
}

/// The newest event of a deed. A death holds the kill of its killer first.
fn deed_event(row: &DeedRow) -> Option<EventId> {
    row.events.last().copied()
}

fn chapters(world: &World, book: &Book, facts: &Facts<'_>) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    for span in book.chapters() {
        let mut left_out = 0;
        let zones = capped(
            book.zones_with_gain(span.steps.clone())
                .into_iter()
                .map(|zone| name_of(world, zone)),
            &mut left_out,
        );
        let met = capped(
            facts
                .people
                .iter()
                .filter(|(_, event)| in_span(book, &span, *event))
                .map(|(person, _)| person.name.clone()),
            &mut left_out,
        );
        let rows: Vec<&DeedRow> = facts
            .deeds
            .iter()
            .filter(|row| deed_event(row).is_some_and(|event| in_world_of(book, &span, event)))
            .collect();
        let (done, again) = deeds_and_again(&rows, &mut left_out);
        chapters.push(Chapter {
            number: chapters.len() + 1,
            first: span.first.0,
            began: tick_of(world, span.first),
            ended: tick_of(world, span.last),
            title: chapter_title(world, &span),
            opened_by: opened_by(span.opening),
            state: entry_state(span.state),
            levels: levels(facts.deeds, span.first, span.last),
            zones,
            people: met,
            deeds: done,
            again,
            left_out,
            prose: None,
            footnotes: Vec::new(),
        });
    }
    chapters
}

fn tales(world: &World, book: &Book, facts: &Facts<'_>, chapters: &[Chapter]) -> Vec<Tale> {
    let mut tales = Vec::new();
    for span in book.tales() {
        let rows: Vec<&DeedRow> = facts
            .deeds
            .iter()
            .filter(|row| deed_event(row).is_some_and(|event| in_tale(book, &span, event)))
            .collect();
        let mut left_out = 0;
        let (deeds, again) = deeds_and_again(&rows, &mut left_out);
        // A tale follows the chapter whose range holds its first step. That never moves.
        let chapter = chapters
            .iter()
            .rev()
            .find(|chapter| chapter.first <= span.first.0)
            .map_or(span.first.0, |chapter| chapter.first);
        tales.push(Tale {
            first: span.first.0,
            chapter,
            instance: name_of(world, span.instance),
            kind: places::kind_of(world, span.instance),
            began: tick_of(world, span.first),
            runs: span.runs,
            deeds,
            again,
            left_out,
            text: None,
        });
    }
    tales
}

/// The deeds of one run of an instance.
#[must_use]
pub fn visit_deeds(character: &Character, book: &Book, visit: &VisitSpan) -> Vec<Deed> {
    deeds_with_events(character.world(), character.you())
        .into_iter()
        .filter(|row| {
            deed_event(row)
                .and_then(|event| book.step_of(event))
                .is_some_and(|step| visit.steps.contains(&step) && !book.is_world_step(step))
        })
        .map(|row| row.deed)
        .collect()
}

/// The deeds of the open world in one zone, oldest first, with their events.
#[must_use]
pub fn deeds_in_zone(character: &Character, book: &Book, zone: EntityId) -> Vec<DeedRow> {
    deeds_with_events(character.world(), character.you())
        .into_iter()
        .filter(|row| {
            deed_event(row)
                .and_then(|event| book.step_of(event))
                .is_some_and(|step| {
                    book.is_world_step(step) && book.zone_of_step(step) == Some(zone)
                })
        })
        .collect()
}

/// The deeds, and one tally line for each foe killed again: "Defeated Hogger again, 6
/// times." A repeat adds no weight, but it still shows.
fn deeds_and_again(rows: &[&DeedRow], left_out: &mut usize) -> (Vec<Deed>, Vec<String>) {
    let mut again: Vec<(String, i64)> = Vec::new();
    let mut done = Vec::new();
    for row in rows {
        match &row.deed {
            Deed::Defeated { foe, times, .. } if *times > 1 => {
                match again.iter_mut().find(|(name, _)| name == foe) {
                    Some(tally) => tally.1 = *times,
                    None => again.push((foe.clone(), *times)),
                }
            }
            deed => done.push(deed.clone()),
        }
    }
    let lines = again
        .into_iter()
        .map(|(foe, times)| format!("Defeated {foe} again, {times} times."));
    (capped(done.into_iter(), left_out), capped(lines, left_out))
}

fn tick_of(world: &World, event: EventId) -> Tick {
    world
        .history()
        .get(event)
        .map_or(Tick(0), |event| event.tick)
}

/// The zone where the chapter opened, a return to it, or its continuation.
fn chapter_title(world: &World, span: &ChapterSpan) -> Option<String> {
    let zone = name_of(world, span.zone?);
    Some(match span.opening {
        Opening::Break(Break::Return) => format!("Return to {zone}"),
        Opening::Max => format!("{zone}, continued"),
        _ => zone,
    })
}

fn opened_by(opening: Opening) -> OpenedBy {
    match opening {
        Opening::First => OpenedBy::First,
        Opening::Break(Break::Return) => OpenedBy::Return,
        Opening::Break(Break::NewZone) => OpenedBy::NewZone,
        Opening::Break(Break::Level) => OpenedBy::Level,
        Opening::Break(Break::Capital) => OpenedBy::Capital,
        Opening::Break(Break::Inn) => OpenedBy::Inn,
        Opening::Break(Break::Away) => OpenedBy::Away,
        Opening::Max => OpenedBy::Max,
        Opening::Rule => OpenedBy::Rule,
    }
}

fn entry_state(state: SpanState) -> EntryState {
    match state {
        SpanState::Open => EntryState::Open,
        SpanState::Closed => EntryState::Closed,
    }
}

/// The level held at the first event of a range, and at its last.
fn levels(rows: &[DeedRow], first: EventId, last: EventId) -> Option<[i64; 2]> {
    let mut start = None;
    let mut end = None;
    for row in rows {
        let (Deed::Level { to, .. }, Some(event)) = (&row.deed, deed_event(row)) else {
            continue;
        };
        if event > last {
            break;
        }
        if event <= first {
            start = Some(*to);
        }
        end = Some(*to);
    }
    let end = end?;
    Some([start.unwrap_or(end), end])
}

fn capped<T>(items: impl Iterator<Item = T>, left_out: &mut usize) -> Vec<T> {
    let mut kept = Vec::new();
    for item in items {
        if kept.len() < CHAPTER_LIST {
            kept.push(item);
        } else {
            *left_out += 1;
        }
    }
    kept
}

impl Deed {
    #[must_use]
    pub fn at(&self) -> Tick {
        match self {
            Deed::Level { at, .. }
            | Deed::Defeated { at, .. }
            | Deed::Titled { at, .. }
            | Deed::QuestDone { at, .. }
            | Deed::GameQuestDone { at, .. }
            | Deed::ClassQuestDone { at, .. }
            | Deed::QuestMarked { at, .. }
            | Deed::Mounted { at, .. }
            | Deed::EpicItem { at, .. }
            | Deed::Upgraded { at, .. }
            | Deed::WonBattle { at, .. }
            | Deed::PvpRank { at, .. }
            | Deed::Died { at, .. } => *at,
        }
    }
}

/// A fact of `holder` that links to `target`, with the event that opened it.
struct Link {
    target: EntityId,
    tick: Tick,
    event: EventId,
}

/// The targets of one linked fact of `holder`, each with the event that opened it. The
/// facts in this list never change, so the opening event is the first time.
fn first_links(world: &World, holder: EntityId, fact: &str) -> Vec<Link> {
    let Some(entity) = world.entity(holder) else {
        return Vec::new();
    };
    let mut links: Vec<_> = entity
        .facts_named(fact)
        .filter_map(|fact| Some((fact.opened, fact.linked_to?)))
        .collect();
    links.sort();
    links
        .into_iter()
        .filter_map(|(opened, target)| {
            let tick = world.history().get(opened)?.tick;
            Some(Link {
                target,
                tick,
                event: opened,
            })
        })
        .collect()
}

/// A deed and the events behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeedRow {
    pub deed: Deed,
    /// One event, or two for a death with a killer: the kill of the killer, then the death.
    pub events: Vec<EventId>,
}

/// A walk of the history, because the state holds only the current level, place, and
/// count of kills.
pub(crate) fn deeds_with_events(world: &World, you: EntityId) -> Vec<DeedRow> {
    let mut deeds = Vec::new();
    let mut here = None;
    // `die` writes the kill of the killer just before the count of deaths.
    let mut killer: Option<(String, EventId)> = None;
    let row = |deed: Deed, events: Vec<EventId>| DeedRow { deed, events };
    for event in world.history() {
        match &event.kind {
            EventKind::FactStart {
                entity,
                name,
                linked_to,
                ..
            } if *entity == you && name == LOCATED_IN => {
                here = *linked_to;
            }
            EventKind::FactStart { entity, name, .. }
            | EventKind::FactUpdate { entity, name, .. }
                if *entity == you && name == LEVEL =>
            {
                let level = level_change(&event.kind);
                let deed = level.map(|(from, to)| level_deed(world, from, to, event.tick, here));
                deeds.extend(deed.map(|deed| row(deed, vec![event.id])));
            }
            EventKind::FactStart {
                entity,
                name,
                value: Some(times),
                linked_to: Some(foe),
            }
            | EventKind::FactUpdate {
                entity,
                name,
                to: times,
                linked_to: Some(foe),
                ..
            } if *entity == you && name == DEFEATED => {
                let place = here.map(|place| name_of(world, place));
                let foe = name_of(world, *foe);
                let deed = Deed::Defeated {
                    foe,
                    times: *times,
                    at: event.tick,
                    place,
                };
                deeds.push(row(deed, vec![event.id]));
            }
            EventKind::FactStart {
                entity: foe,
                name,
                linked_to: Some(target),
                ..
            }
            | EventKind::FactUpdate {
                entity: foe,
                name,
                linked_to: Some(target),
                ..
            } if *target == you && name == DEFEATED => {
                killer = Some((name_of(world, *foe), event.id));
            }
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(thing),
                ..
            } if *entity == you && THING_DEEDS.contains(&name.as_str()) => {
                let place = here.map(|place| name_of(world, place));
                let deed = thing_deed(world, name, *thing, event.tick, place);
                deeds.extend(deed.map(|deed| row(deed, vec![event.id])));
            }
            EventKind::FactStart { entity, name, .. }
            | EventKind::FactUpdate { entity, name, .. }
                if *entity == you && (name == BG_WON || name == PVP_RANK) =>
            {
                let place = here.map(|place| name_of(world, place));
                let deed = battle_deed(world, &event.kind, event.tick, place);
                deeds.extend(deed.map(|deed| row(deed, vec![event.id])));
            }
            EventKind::FactStart { entity, name, .. }
            | EventKind::FactUpdate { entity, name, .. }
                if *entity == you && name == DEATHS =>
            {
                let place = here.map(|place| name_of(world, place));
                deeds.push(death_row(killer.take(), event.id, event.tick, place));
            }
            _ => {}
        }
    }
    deeds
}

/// The level before and after a change of level. The first level that the world saw has no
/// level before it.
fn level_change(kind: &EventKind) -> Option<(Option<i64>, i64)> {
    match kind {
        EventKind::FactStart {
            value: Some(to), ..
        } => Some((None, *to)),
        EventKind::FactUpdate { from, to, .. } => Some((Some(*from), *to)),
        _ => None,
    }
}

/// The first win in a battleground, or a new rank. The rank that the world first knew is
/// no deed: it came at a login.
fn battle_deed(world: &World, kind: &EventKind, at: Tick, place: Option<String>) -> Option<Deed> {
    match kind {
        EventKind::FactStart {
            linked_to: Some(battleground),
            ..
        } => Some(Deed::WonBattle {
            battleground: name_of(world, *battleground),
            at,
            place,
        }),
        EventKind::FactUpdate { to, .. } => Some(Deed::PvpRank {
            rank: *to,
            at,
            place,
        }),
        _ => None,
    }
}

/// A death holds the kill of its killer, when the world knows one.
fn death_row(
    killer: Option<(String, EventId)>,
    death: EventId,
    at: Tick,
    place: Option<String>,
) -> DeedRow {
    let (killer, mut events) = match killer {
        Some((name, kill)) => (Some(name), vec![kill]),
        None => (None, Vec::new()),
    };
    events.push(death);
    DeedRow {
        deed: Deed::Died { killer, at, place },
        events,
    }
}

/// A title that you earned, or a quest that you finished: both are things that you hold.
/// The facts of yours that point at a thing and make a deed.
const THING_DEEDS: [&str; 8] = [
    TITLE,
    QUEST_DONE,
    GAME_QUEST_DONE,
    MARKED_BY,
    FIRST_MOUNT,
    FIRST_EPIC_MOUNT,
    FIRST_EPIC_ITEM,
    UPGRADED,
];

fn thing_deed(
    world: &World,
    fact: &str,
    thing: EntityId,
    at: Tick,
    place: Option<String>,
) -> Option<Deed> {
    match fact {
        GAME_QUEST_DONE => game_quest_deed(world, thing, at, place),
        MARKED_BY => mark_deed(world, thing, at, place),
        FIRST_MOUNT | FIRST_EPIC_MOUNT => {
            let name = name_of(world, thing);
            Some(Deed::Mounted {
                mount: title_of_mount(&name)?.to_string(),
                epic: fact == FIRST_EPIC_MOUNT,
                at,
                place,
            })
        }
        FIRST_EPIC_ITEM | UPGRADED => {
            let name = name_of(world, thing);
            let item = title_of_item(&name)?.to_string();
            if fact == UPGRADED {
                return Some(Deed::Upgraded { item, at, place });
            }
            Some(Deed::EpicItem { item, at, place })
        }
        QUEST_DONE => {
            let name = name_of(world, thing);
            let title = title_of_thing(&name).map_or_else(|| name.clone(), str::to_string);
            Some(Deed::QuestDone { title, at, place })
        }
        _ => Some(Deed::Titled {
            title: name_of(world, thing),
            at,
            place,
        }),
    }
}

fn game_quest_deed(
    world: &World,
    quest: EntityId,
    at: Tick,
    place: Option<String>,
) -> Option<Deed> {
    let entity = world.entity(quest)?;
    let title = title_of_game_quest(&entity.name)?.to_string();
    if entity.fact(CLASS_QUEST, None).is_some() {
        return Some(Deed::ClassQuestDone { title, at, place });
    }
    Some(Deed::GameQuestDone { title, at, place })
}

fn mark_deed(world: &World, mark: EntityId, at: Tick, place: Option<String>) -> Option<Deed> {
    let (mark, quest) = mark_and_quest(world, mark)?;
    Some(Deed::QuestMarked {
        mark,
        quest,
        at,
        place,
    })
}

/// The name of a mark, and the title of the quest that put it.
#[must_use]
pub fn mark_and_quest(world: &World, mark: EntityId) -> Option<(String, String)> {
    let entity = world.entity(mark)?;
    let quest = entity
        .facts_named(MARK_OF)
        .find_map(|fact| fact.linked_to)?;
    let quest = title_of_game_quest(&world.entity(quest)?.name)?.to_string();
    Some((title_of_mark(&entity.name)?.to_string(), quest))
}

fn level_deed(world: &World, from: Option<i64>, to: i64, at: Tick, here: Option<EntityId>) -> Deed {
    let place = here.map(|place| name_of(world, place));
    Deed::Level {
        from,
        to,
        at,
        place,
    }
}

fn fact_value(world: &World, holder: EntityId, fact: &str, target: EntityId) -> Option<i64> {
    world.entity(holder)?.fact(fact, Some(target))?.value
}

fn name_of(world: &World, id: EntityId) -> String {
    world
        .entity(id)
        .map(|entity| entity.name.clone())
        .unwrap_or_default()
}
