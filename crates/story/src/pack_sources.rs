//! The passages of the lore pack, built from a wiki dump on the computer of the player
//! (GAMEPLAY.md 5.10). The repo holds only the list of pages in `data/pack_sources.toml`.

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

/// A line that starts with one of these is a list, a table, or an indent, not prose.
const NOT_PROSE_STARTS: [char; 7] = ['*', '#', ':', ';', '|', '!', '{'];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub books: Books,
    #[serde(default)]
    pub pages: Vec<WikiPage>,
    #[serde(default)]
    pub later: Later,
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

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Later {
    pub terms: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SourcesError {
    #[error("pack sources: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("pack sources: a later term is no regular expression: {0}")]
    LaterTerm(#[from] regex::Error),
    #[error(transparent)]
    Dump(#[from] DumpError),
    #[error("pack sources: the dump has no page \"{0}\"")]
    NoIndex(String),
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
    /// Returns an error when the text is not a valid list of sources.
    pub fn parse(text: &str) -> Result<Sources, SourcesError> {
        Ok(toml::from_str(text)?)
    }
}

/// What the builder did with one page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Read { passages: usize },
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
/// pages. The same dump gives the same passages.
///
/// # Errors
///
/// Returns an error when the dump cannot be read, when it lacks the index page, or when a
/// later term is broken. A missing book or page is only reported.
pub fn from_dump(path: &Path, sources: &Sources) -> Result<Built, SourcesError> {
    let later = later_terms(&sources.later.terms)?;
    let mut first_titles = vec![sources.books.index.clone()];
    first_titles.extend(sources.pages.iter().map(|page| page.title.clone()));
    let first = dump::pages(path, &first_titles)?;
    let index = first
        .get(&sources.books.index)
        .ok_or_else(|| SourcesError::NoIndex(sources.books.index.clone()))?;
    let (book_titles, missing_chapters) = book_titles(&index.text, &sources.books.chapters);
    let books = dump::pages(path, &book_titles)?;
    let mut built = Built {
        missing_chapters,
        ..Built::default()
    };
    add_books(&mut built, &book_titles, &books, &sources.books);
    for page in &sources.pages {
        add_page(&mut built, page, first.get(&page.title), later.as_ref());
    }
    Ok(built)
}

/// One pattern for all terms, so a paragraph is searched once.
fn later_terms(terms: &[String]) -> Result<Option<Regex>, regex::Error> {
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
        push_passages(built, &page.title, texts, &source, &[Link::Common]);
    }
}

fn add_page(built: &mut Built, wanted: &WikiPage, page: Option<&Page>, later: Option<&Regex>) {
    let Some(page) = page else {
        built.report.push(missing(&wanted.title));
        return;
    };
    let mut texts = Vec::new();
    for section in sections(&page.text) {
        let kept = match section.heading {
            None => wanted.lead,
            Some(heading) => wanted.sections.iter().any(|name| name == heading),
        };
        if kept {
            texts.extend(paragraphs(&plain(section.body)));
        }
    }
    if let Some(later) = later {
        texts.retain(|text| !later.is_match(text));
    }
    let source = format!("the wiki page \"{}\"", page.title);
    push_passages(built, &page.title, texts, &source, &links(wanted));
}

fn links(page: &WikiPage) -> Vec<Link> {
    let places = page.places.iter().cloned().map(Link::Place);
    let npcs = page.npcs.iter().cloned().map(Link::Npc);
    let common = page.common.then_some(Link::Common);
    places.chain(npcs).chain(common).collect()
}

fn push_passages(built: &mut Built, title: &str, texts: Vec<String>, source: &str, links: &[Link]) {
    built.report.push(PageReport {
        title: title.to_string(),
        outcome: Outcome::Read {
            passages: texts.len(),
        },
    });
    built.passages.extend(texts.into_iter().map(|text| Passage {
        text,
        source: source.to_string(),
        links: links.to_vec(),
        origin: Origin::Pack,
    }));
}

fn missing(title: &str) -> PageReport {
    PageReport {
        title: title.to_string(),
        outcome: Outcome::Missing,
    }
}

/// Each line of plain text is a paragraph. Runs of spaces become one space. A paragraph
/// past the limit of the bridge becomes several.
#[must_use]
pub fn paragraphs(plain: &str) -> Vec<String> {
    let prose = plain.lines().map(one_line).filter(|line| is_prose(line));
    prose.flat_map(|line| pieces(&line)).collect()
}

fn one_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_prose(line: &str) -> bool {
    line.chars().count() >= MIN_PARAGRAPH_CHARS && !line.starts_with(NOT_PROSE_STARTS)
}
