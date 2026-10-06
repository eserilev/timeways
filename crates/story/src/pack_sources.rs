//! The passages of the lore pack, built from a wiki dump on the computer of the player
//! (GAMEPLAY.md 5.10). The repo holds only the list of pages in `data/pack_sources.toml`.

use crate::check::later_names;
use crate::dump::{self, DumpError, Page};
use crate::pack::{Link, Origin, Passage};
use crate::passage_limits::pieces;
use crate::wikitext::{book_content, listed_pages, plain, sections};
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
    /// A paragraph that talks about the game, not the world, goes out: players, levels,
    /// instances, and loot.
    #[serde(default)]
    pub game: Terms,
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
    /// `later` counts the paragraphs that a later term dropped, and `game` the other
    /// paragraphs that a game term dropped.
    Read {
        passages: usize,
        later: usize,
        game: usize,
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
    pub passages: Vec<Passage>,
    /// One line for each book and each page, in the order of the passages.
    pub report: Vec<PageReport>,
    /// Chapters of the list that the index page lacks, for example after a rename.
    pub missing_chapters: Vec<String>,
}

/// The passages of the dump, in the order of the list: the books by chapter, then the
/// pages. The same dump gives the same passages. The dump is read at most twice: once for
/// the index, the pages, and every redirect, and once for the books and the targets of
/// redirects.
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
    Ok(built)
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
        let texts = paragraphs(&plain(content));
        let outcome = Outcome::Read {
            passages: texts.len(),
            later: 0,
            game: 0,
        };
        let book = Shelf {
            source: &source,
            links: &[Link::Common],
            about: None,
        };
        push_passages(built, &page.title, outcome, texts, &book);
    }
}

fn add_page(built: &mut Built, wanted: &WikiPage, page: Option<&Page>, filters: &Filters) {
    let Some(page) = page else {
        built.report.push(missing(&wanted.title));
        return;
    };
    // The terms read whole paragraphs: the rest of a long paragraph tells the same story.
    let mut whole: Vec<String> = kept_bodies(&page.text, wanted)
        .into_iter()
        .flat_map(|body| prose_lines(&plain(body)))
        .collect();
    let read = whole.len();
    whole.retain(|text| !filters.is_later(text));
    let not_later = whole.len();
    whole.retain(|text| !filters.is_game(text));
    let game = not_later - whole.len();
    let texts: Vec<String> = whole.iter().flat_map(|text| pieces(text)).collect();
    let outcome = Outcome::Read {
        passages: texts.len(),
        later: read - not_later,
        game,
    };
    let source = format!("the wiki page \"{}\"", page.title);
    let links = links(wanted);
    let shelf = Shelf {
        source: &source,
        links: &links,
        about: subject_of(&wanted.title, &links),
    };
    push_passages(built, &page.title, outcome, texts, &shelf);
}

/// The link that a page is about, when its title names it: the page "Deadmines" is about
/// "The Deadmines", and "Shadowfang Keep (Classic)" about "Shadowfang Keep". The page "Mr.
/// Smite" links to "The Deadmines" and is about none of its links.
#[must_use]
pub fn subject_of(title: &str, links: &[Link]) -> Option<String> {
    let title = bare_title(title);
    links.iter().find_map(|link| match link {
        Link::Place(name) | Link::Npc(name) if bare_title(name) == title => Some(name.clone()),
        _ => None,
    })
}

/// A title in lower case, with no "the" before it and no "(Classic)" after it.
fn bare_title(title: &str) -> String {
    let title = title.trim().to_lowercase();
    let title = match title.rfind(" (") {
        Some(at) if title.ends_with(')') => title[..at].to_string(),
        _ => title,
    };
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
}

fn push_passages(
    built: &mut Built,
    title: &str,
    outcome: Outcome,
    texts: Vec<String>,
    shelf: &Shelf<'_>,
) {
    built.report.push(PageReport {
        title: title.to_string(),
        outcome,
    });
    built.passages.extend(texts.into_iter().map(|text| Passage {
        text,
        source: shelf.source.to_string(),
        links: shelf.links.to_vec(),
        origin: Origin::Pack,
        about: shelf.about.clone(),
    }));
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
