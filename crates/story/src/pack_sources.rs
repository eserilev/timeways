//! The passages of the lore pack, built from a wiki dump on the computer of the player
//! (GAMEPLAY.md 5.10). The repo holds only the list of pages in `data/pack_sources.toml`.

use crate::check::later_names;
use crate::dump::{self, DumpError, Page};
use crate::game_talk::{Cut, cut_game_talk};
use crate::outcome_passages::{self, PageKind, page_kind, with_known_bosses};
use crate::pack::{Deed, Dependency, Link, Origin, Passage};
use crate::passage_limits::pieces;
use crate::setup_passages::{self, Instances};
use crate::wikitext::{Cites, book_content, cites, listed_pages, plain, sections};
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

const BUNDLED: &str = include_str!("../data/pack_sources.toml");

/// A shorter line is a caption, a heading, or a scrap of a list.
const MIN_PARAGRAPH_CHARS: usize = 80;

/// A line that starts with one of these is a list or a table, not prose.
const NOT_PROSE_STARTS: [char; 6] = ['*', '#', ';', '|', '!', '{'];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub books: Books,
    #[serde(default)]
    pub pages: Vec<WikiPage>,
    /// A paragraph that tells of a time after 25 ADP goes out.
    #[serde(default)]
    pub later: Terms,
    /// A sentence that talks about the game, not the world, goes out: players, levels,
    /// instances, and loot (`game_talk`).
    #[serde(default)]
    pub game: Terms,
    /// The dungeons and raids of the list. A setup passage belongs to one of them
    /// (`setup_passages`).
    #[serde(default)]
    pub instances: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Books {
    /// The page whose chapter sections list the books.
    pub index: String,
    pub chapters: Vec<String>,
    #[serde(default)]
    pub title_suffix: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiPage {
    pub title: String,
    /// Keep the text before the first heading.
    #[serde(default)]
    pub lead: bool,
    pub sections: Vec<String>,
    #[serde(default)]
    pub places: Vec<String>,
    #[serde(default)]
    pub npcs: Vec<String>,
    #[serde(default)]
    pub common: bool,
    /// The foes that the page tells of with no link, by their names in the game: the boss of
    /// a place, or the head of a group. A deed of the page that names no foe is the defeat
    /// of the first (`outcome_passages::dependencies`).
    #[serde(default)]
    pub foes: Vec<String>,
}

/// Regular expressions. The match ignores case.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terms {
    pub terms: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SourcesError {
    #[error("pack sources: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("pack sources: a term is no regular expression: {0}")]
    BadTerm(#[from] regex::Error),
    #[error(transparent)]
    Dump(#[from] DumpError),
    #[error("pack sources: the dump has no page \"{0}\"")]
    NoIndex(String),
    #[error("pack sources: the page \"{0}\" has no place, no NPC, and is not common")]
    Unlinked(String),
}

impl Sources {
    /// # Errors
    ///
    /// Returns an error when the bundled list is broken. A test reads it, so it is not.
    pub fn bundled() -> Result<Sources, SourcesError> {
        Sources::parse(BUNDLED)
    }

    /// # Errors
    ///
    /// Returns an error when the text is not a valid list of sources, or when a page has
    /// no link.
    pub fn parse(text: &str) -> Result<Sources, SourcesError> {
        let sources: Sources = toml::from_str(text)?;
        if let Some(page) = sources.pages.iter().find(|page| links(page).is_empty()) {
            return Err(SourcesError::Unlinked(page.title.clone()));
        }
        Ok(sources)
    }
}

/// What the builder did with one page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// `later` counts the paragraphs that a later term dropped, `game` the other
    /// paragraphs that a game term dropped, and `cut` the game sentences of the paragraphs
    /// that stayed.
    Read {
        passages: usize,
        later: usize,
        game: usize,
        cut: usize,
    },
    Missing,
    NoBook,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageReport {
    pub title: String,
    pub outcome: Outcome,
}

#[derive(Debug, Default)]
pub struct Built {
    /// An outcome passage holds what it depends on (`outcome_passages`).
    pub passages: Vec<Passage>,
    /// One line for each book and each page, in the order of the passages.
    pub report: Vec<PageReport>,
    /// Chapters of the list that the index page lacks, for example after a rename.
    pub missing_chapters: Vec<String>,
    /// The outcome passages that wait for the kinds of the pages that they cite.
    outcomes: Vec<Tagged>,
    /// The setup passages that wait for the same.
    setups: Vec<Tagged>,
}

/// An outcome or a setup passage of `Built::passages`, with the page that it comes from and the cites
/// of its paragraph.
#[derive(Clone, Debug)]
struct Tagged {
    passage: usize,
    page: String,
    cites: Cites,
    foes: Vec<String>,
}

/// The passages of the dump, in the order of the list: the books by chapter, then the
/// pages. The same dump gives the same passages. The dump is read at most three times:
/// once for the index, the pages, and every redirect, once for the books and the targets
/// of redirects, and once for the pages that the outcome and setup passages cite.
///
/// # Errors
///
/// Returns an error when the dump cannot be read, when it lacks the index page, or when a
/// term is broken. A missing book or page is only reported.
pub fn from_dump(path: &Path, sources: &Sources) -> Result<Built, SourcesError> {
    let filters = Filters {
        later: one_pattern(&later_terms(sources))?,
        game: one_pattern(&sources.game.terms)?,
    };
    let page_titles: Vec<String> = sources
        .pages
        .iter()
        .map(|page| page.title.clone())
        .collect();
    let mut first_titles: BTreeSet<String> = page_titles.iter().cloned().collect();
    first_titles.insert(sources.books.index.clone());
    let first = dump::scan(path, &first_titles)?;
    let index = first
        .page(&sources.books.index, &BTreeMap::new())
        .ok_or_else(|| SourcesError::NoIndex(sources.books.index.clone()))?;
    let (book_titles, missing_chapters) = book_titles(&index.text, &sources.books.chapters);
    let all_titles = [book_titles.as_slice(), page_titles.as_slice()].concat();
    let more = dump::texts(path, &first.lacking(&all_titles))?;
    let found: BTreeMap<String, Page> = all_titles
        .iter()
        .filter_map(|title| Some((title.clone(), first.page(title, &more)?)))
        .collect();
    let mut built = Built {
        missing_chapters,
        ..Built::default()
    };
    add_books(&mut built, &book_titles, &found, &sources.books);
    for page in &sources.pages {
        add_page(&mut built, page, found.get(&page.title), &filters);
    }
    let bosses = known_bosses(sources);
    let instances = Instances {
        names: sources.instances.clone(),
        bosses: boss_places(sources),
    };
    tag_deeds(&mut built, path, &first, &found, &bosses, &instances)?;
    Ok(built)
}

/// Each outcome passage gets what it depends on, and each setup passage what it sets up,
/// from the kinds of the pages that its paragraph cites, and of its own page.
fn tag_deeds(
    built: &mut Built,
    path: &Path,
    first: &dump::Scan,
    found: &BTreeMap<String, Page>,
    bosses: &[String],
    instances: &Instances,
) -> Result<(), SourcesError> {
    let cited: Vec<String> = built
        .outcomes
        .iter()
        .chain(&built.setups)
        .flat_map(|tagged| tagged.cites.links.iter().chain(&tagged.cites.refs))
        .cloned()
        .collect();
    let more = dump::texts(path, &first.lacking(&cited))?;
    let kind_of = |title: &str| -> Option<PageKind> {
        let page = found
            .get(title)
            .cloned()
            .or_else(|| first.page(title, &more))?;
        Some(with_known_bosses(
            page_kind(&page.title, &page.text),
            bosses,
        ))
    };
    for tagged in std::mem::take(&mut built.outcomes) {
        let Some(passage) = built.passages.get_mut(tagged.passage) else {
            continue;
        };
        let paragraph = outcome_passages::Paragraph {
            text: &passage.text,
            cites: &tagged.cites,
            page: &tagged.page,
            bosses,
            foes: &tagged.foes,
        };
        passage.depends_on = outcome_passages::dependencies(&paragraph, kind_of);
    }
    let mut windows = BTreeMap::new();
    for tagged in std::mem::take(&mut built.setups) {
        let Some(passage) = built.passages.get_mut(tagged.passage) else {
            continue;
        };
        if let Some(window) = tag_setup(passage, &tagged, instances, kind_of) {
            windows.insert(tagged.passage, window);
        }
    }
    built.passages = with_windows(std::mem::take(&mut built.passages), windows);
    Ok(())
}

/// A paragraph that tells no end is a setup as a whole. One that tells an end, such as
/// an outcome passage, keeps its own tags, and its setup comes back as a new passage: the
/// part before the end (`setup_passages::window`).
fn tag_setup(
    passage: &mut Passage,
    tagged: &Tagged,
    instances: &Instances,
    kind_of: impl Fn(&str) -> Option<PageKind>,
) -> Option<Passage> {
    let found = setup_passages::Found {
        text: &passage.text,
        cites: &tagged.cites,
        page: &tagged.page,
        links: &passage.links,
    };
    let setup = setup_passages::setup_for(&found, instances, kind_of)?;
    if !setup_passages::tells_an_end(&passage.text) {
        passage.setup_for = Some(setup);
        return None;
    }
    // An end that names no deed of its own is the end of the deed of the setup: "in time
    // the other leaders succumbed" waits for the defeat of Whitemane.
    if passage.depends_on.is_empty() {
        passage.depends_on = vec![match &setup.deed {
            Deed::Foe(name) => Dependency::Foe(name.clone()),
            Deed::Quest(title) => Dependency::Quest(title.clone()),
        }];
    }
    let text = setup_passages::window(&passage.text, &setup.deed)?.to_string();
    Some(Passage {
        text,
        depends_on: Vec::new(),
        setup_for: Some(setup),
        ..passage.clone()
    })
}

/// Each new setup passage comes right after the paragraph that holds it, so the pack keeps
/// the order of the pages.
fn with_windows(passages: Vec<Passage>, mut windows: BTreeMap<usize, Passage>) -> Vec<Passage> {
    let mut all = Vec::with_capacity(passages.len() + windows.len());
    for (index, passage) in passages.into_iter().enumerate() {
        all.push(passage);
        all.extend(windows.remove(&index));
    }
    all
}

/// The paragraphs of a wiki page that go out. A book needs no filter: the books end
/// before the later expansions, and they never talk about the game.
struct Filters {
    later: Option<Regex>,
    game: Option<Regex>,
}

impl Filters {
    fn is_later(&self, text: &str) -> bool {
        self.later
            .as_ref()
            .is_some_and(|later| later.is_match(text))
    }

    fn is_game(&self, text: &str) -> bool {
        self.game.as_ref().is_some_and(|game| game.is_match(text))
    }
}

/// The later terms of the list, and each name of the cutoff list (GAMEPLAY.md 5.9): a name
/// that no answer may say is no lore of 25 ADP. "Pandaria" also holds "Pandarian".
fn later_terms(sources: &Sources) -> Vec<String> {
    let names = later_names().map(|name| format!(r"\b{}", regex::escape(name)));
    sources.later.terms.iter().cloned().chain(names).collect()
}

/// One pattern for all terms, so a paragraph is searched once.
fn one_pattern(terms: &[String]) -> Result<Option<Regex>, regex::Error> {
    if terms.is_empty() {
        return Ok(None);
    }
    let alternatives: Vec<String> = terms.iter().map(|term| format!("(?:{term})")).collect();
    Regex::new(&format!("(?i){}", alternatives.join("|"))).map(Some)
}

/// The listed books of the chosen chapters in page order, and the chosen chapters that
/// the page lacks.
fn book_titles(index: &str, chapters: &[String]) -> (Vec<String>, Vec<String>) {
    let mut titles = Vec::new();
    let mut seen_chapters = BTreeSet::new();
    for section in sections(index) {
        let Some(heading) = section.heading else {
            continue;
        };
        if chapters.iter().any(|chapter| chapter == heading) {
            seen_chapters.insert(heading);
            titles.extend(listed_pages(section.body));
        }
    }
    let missing = chapters
        .iter()
        .filter(|chapter| !seen_chapters.contains(chapter.as_str()))
        .cloned()
        .collect();
    (titles, missing)
}

fn add_books(built: &mut Built, titles: &[String], books: &BTreeMap<String, Page>, list: &Books) {
    let mut done = BTreeSet::new();
    for title in titles {
        let Some(page) = books.get(title) else {
            built.report.push(missing(title));
            continue;
        };
        // Two titles can redirect to one book.
        if !done.insert(page.title.clone()) {
            continue;
        }
        let Some(content) = book_content(&page.text) else {
            built.report.push(PageReport {
                title: page.title.clone(),
                outcome: Outcome::NoBook,
            });
            continue;
        };
        let name = page
            .title
            .strip_suffix(list.title_suffix.as_str())
            .unwrap_or(&page.title);
        let source = format!("the book \"{name}\"");
        let texts = uncited(paragraphs(&plain(content)));
        let outcome = Outcome::Read {
            passages: texts.len(),
            later: 0,
            game: 0,
            cut: 0,
        };
        let book = Shelf {
            source: &source,
            links: &[Link::Common],
            about: None,
            foes: &[],
        };
        push_passages(built, &page.title, outcome, texts, &book);
    }
}

fn add_page(built: &mut Built, wanted: &WikiPage, page: Option<&Page>, filters: &Filters) {
    let Some(page) = page else {
        built.report.push(missing(&wanted.title));
        return;
    };
    // A later term reads whole paragraphs: the rest of a long paragraph tells the same
    // story, and a cut can leave half of a later story.
    let mut whole: Vec<Paragraph> = kept_bodies(&page.text, wanted)
        .into_iter()
        .flat_map(cited_paragraphs)
        .collect();
    let read = whole.len();
    whole.retain(|paragraph| !filters.is_later(&paragraph.text));
    let later = read - whole.len();
    let talk = without_game_talk(whole, filters);
    let texts: Vec<Paragraph> = talk.kept.iter().flat_map(cut_in_pieces).collect();
    let outcome = Outcome::Read {
        passages: texts.len(),
        later,
        game: talk.dropped,
        cut: talk.cut,
    };
    let source = format!("the wiki page \"{}\"", page.title);
    let links = links(wanted);
    let shelf = Shelf {
        source: &source,
        links: &links,
        about: Some(subject_of(&wanted.title, &links)),
        foes: &wanted.foes,
    };
    push_passages(built, &page.title, outcome, texts, &shelf);
}

/// A paragraph of plain text, with the links and the references of its line of wikitext.
struct Paragraph {
    text: String,
    cites: Cites,
}

/// The prose lines of a body, each with its cites. A line of plain text gets the cites of
/// the line of wikitext that gives it. A line that no single line of wikitext gives, for
/// example after a template over two lines, gets no cites.
fn cited_paragraphs(body: &str) -> Vec<Paragraph> {
    let by_line: BTreeMap<String, Cites> = body
        .lines()
        .map(|raw| (one_line(&plain(raw)), cites(raw)))
        .collect();
    prose_lines(&plain(body))
        .into_iter()
        .map(|text| Paragraph {
            cites: by_line.get(&text).cloned().unwrap_or_default(),
            text,
        })
        .collect()
}

fn uncited(texts: Vec<String>) -> Vec<Paragraph> {
    texts
        .into_iter()
        .map(|text| Paragraph {
            text,
            cites: Cites::default(),
        })
        .collect()
}

/// A long paragraph in pieces that fit the bridge. Each piece keeps the cites of the whole.
fn cut_in_pieces(paragraph: &Paragraph) -> Vec<Paragraph> {
    pieces(&paragraph.text)
        .into_iter()
        .map(|text| Paragraph {
            text,
            cites: paragraph.cites.clone(),
        })
        .collect()
}

/// The paragraphs after the cut of their game sentences.
struct GameCut {
    kept: Vec<Paragraph>,
    /// Paragraphs that went whole.
    dropped: usize,
    /// Sentences that went from kept paragraphs.
    cut: usize,
}

fn without_game_talk(paragraphs: Vec<Paragraph>, filters: &Filters) -> GameCut {
    let mut done = GameCut {
        kept: Vec::new(),
        dropped: 0,
        cut: 0,
    };
    for paragraph in paragraphs {
        match cut_game_talk(&paragraph.text, |sentence| filters.is_game(sentence)) {
            Cut::Whole => done.kept.push(paragraph),
            Cut::Trimmed { text, dropped } => {
                done.kept.push(Paragraph {
                    text,
                    cites: paragraph.cites,
                });
                done.cut += dropped;
            }
            Cut::Dropped => done.dropped += 1,
        }
    }
    done
}

/// The people of the list in a place, such as "Mr. Smite" in the Deadmines: a page
/// with places only, whose subject is none of its places. A faction such as "Defias
/// Brotherhood" has that shape too, and its infobox is no NPC, so it never counts as a foe.
#[must_use]
pub fn known_bosses(sources: &Sources) -> Vec<String> {
    sources
        .pages
        .iter()
        .filter(|page| page.npcs.is_empty() && !page.common && !page.places.is_empty())
        .filter(|page| !page.places.contains(&subject_of(&page.title, &links(page))))
        .map(|page| without_suffix(&page.title).to_string())
        .collect()
}

/// The places of each known boss (`known_bosses`): "Edwin VanCleef" is in the Deadmines.
#[must_use]
pub fn boss_places(sources: &Sources) -> BTreeMap<String, Vec<String>> {
    let bosses = known_bosses(sources);
    sources
        .pages
        .iter()
        .map(|page| (without_suffix(&page.title).to_string(), page.places.clone()))
        .filter(|(name, _)| bosses.contains(name))
        .collect()
}

/// The link that a page is about, when its title names it: the page "Deadmines" is about
/// "The Deadmines", and "Shadowfang Keep (Classic)" about "Shadowfang Keep". Else the page
/// is about its own title: "Undercity" (a common page) is about "Undercity", and "Mr.
/// Smite", which links to "The Deadmines", is about "Mr. Smite".
#[must_use]
pub fn subject_of(title: &str, links: &[Link]) -> String {
    let bare = bare_title(title);
    let named = links.iter().find_map(|link| match link {
        Link::Place(name) | Link::Npc(name) if bare_title(name) == bare => Some(name.clone()),
        _ => None,
    });
    named.unwrap_or_else(|| without_suffix(title.trim()).to_string())
}

/// A title with no "(Classic)" after it.
fn without_suffix(title: &str) -> &str {
    match title.rfind(" (") {
        Some(at) if title.ends_with(')') => &title[..at],
        _ => title,
    }
}

/// A title in lower case, with no "the" before it and no "(Classic)" after it.
fn bare_title(title: &str) -> String {
    let title = without_suffix(title.trim()).to_lowercase();
    title.strip_prefix("the ").unwrap_or(&title).to_string()
}

/// The bodies of the listed sections, in page order. A subsection goes in only when its
/// parent goes in too: a page can hold "World of Warcraft" under "History" and again
/// under "Quotes".
fn kept_bodies<'a>(text: &'a str, wanted: &WikiPage) -> Vec<&'a str> {
    let mut bodies = Vec::new();
    // The headings above the section, each with its level and whether it went in.
    let mut parents: Vec<(usize, bool)> = Vec::new();
    for section in sections(text) {
        let Some(heading) = section.heading else {
            if wanted.lead {
                bodies.push(section.body);
            }
            continue;
        };
        while parents
            .last()
            .is_some_and(|&(level, _)| level >= section.level)
        {
            parents.pop();
        }
        let parent_kept = parents.last().is_none_or(|&(_, kept)| kept);
        let kept = parent_kept && wanted.sections.iter().any(|name| name == heading);
        parents.push((section.level, kept));
        if kept {
            bodies.push(section.body);
        }
    }
    bodies
}

fn links(page: &WikiPage) -> Vec<Link> {
    let places = page.places.iter().cloned().map(Link::Place);
    let npcs = page.npcs.iter().cloned().map(Link::Npc);
    let common = page.common.then_some(Link::Common);
    places.chain(npcs).chain(common).collect()
}

/// What each passage of one book or page shares.
struct Shelf<'a> {
    source: &'a str,
    links: &'a [Link],
    about: Option<String>,
    foes: &'a [String],
}

/// An outcome passage waits in `Built::outcomes` for what it depends on.
fn push_passages(
    built: &mut Built,
    title: &str,
    outcome: Outcome,
    paragraphs: Vec<Paragraph>,
    shelf: &Shelf<'_>,
) {
    built.report.push(PageReport {
        title: title.to_string(),
        outcome,
    });
    for paragraph in paragraphs {
        let tagged = Tagged {
            passage: built.passages.len(),
            page: title.to_string(),
            cites: paragraph.cites,
            foes: shelf.foes.to_vec(),
        };
        if !setup_passages::setup_sentences(&paragraph.text).is_empty() {
            built.setups.push(tagged.clone());
        }
        if outcome_passages::may_tell_an_end(&paragraph.text) {
            built.outcomes.push(tagged);
        }
        built.passages.push(Passage {
            text: paragraph.text,
            source: shelf.source.to_string(),
            links: shelf.links.to_vec(),
            origin: Origin::Pack,
            about: shelf.about.clone(),
            depends_on: Vec::new(),
            setup_for: None,
        });
    }
}

fn missing(title: &str) -> PageReport {
    PageReport {
        title: title.to_string(),
        outcome: Outcome::Missing,
    }
}

/// Each line of plain text is a paragraph. Runs of spaces become one space. An indented
/// line is a quote, such as the description of a dungeon, so it counts without its indent.
/// A paragraph past the limit of the bridge becomes several.
#[must_use]
pub fn paragraphs(plain: &str) -> Vec<String> {
    prose_lines(plain)
        .iter()
        .flat_map(|line| pieces(line))
        .collect()
}

/// The lines of prose, each whole, however long.
fn prose_lines(plain: &str) -> Vec<String> {
    plain
        .lines()
        .map(one_line)
        .filter(|line| is_prose(line))
        .collect()
}

fn one_line(line: &str) -> String {
    let unindented = line.trim_start_matches(':');
    unindented.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_prose(line: &str) -> bool {
    line.chars().count() >= MIN_PARAGRAPH_CHARS && !line.starts_with(NOT_PROSE_STARTS)
}
