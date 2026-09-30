//! Pages from a MediaWiki XML export, as a `.xml` file or inside a `.7z` archive.
//!
//! A dump holds about a gigabyte of text, so the reader streams it and keeps only the
//! pages that it looks for.

use crate::wikitext::redirect_target;
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, Event};
use sevenz_rust2::{ArchiveReader, Password};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

/// Articles live in namespace 0. A talk page or a user page with the same name does not.
const ARTICLES: &str = "0";

#[derive(Debug, Error)]
pub enum DumpError {
    #[error("dump: {0}")]
    Io(#[from] std::io::Error),
    #[error("dump: broken XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("dump: broken 7z archive: {0}")]
    Archive(#[from] sevenz_rust2::Error),
    #[error("dump: the 7z archive holds no .xml file")]
    NoXml,
    #[error("dump: the XML ends inside an element")]
    Truncated,
    #[error("dump: unknown XML entity &{0};")]
    Entity(String),
}

/// A page of the dump. After a redirect, the title is the title of the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub title: String,
    pub text: String,
}

/// One read of the whole dump: the wanted articles, and the target of every redirect
/// article.
#[derive(Debug, Default)]
pub struct Scan {
    pub texts: BTreeMap<String, String>,
    pub redirects: BTreeMap<String, String>,
}

impl Scan {
    /// The page of `title`, after one step of a redirect. `more` holds the texts of a
    /// later read.
    #[must_use]
    pub fn page(&self, title: &str, more: &BTreeMap<String, String>) -> Option<Page> {
        let title = self.redirects.get(title).map_or(title, String::as_str);
        let text = self.texts.get(title).or_else(|| more.get(title))?;
        Some(Page {
            title: title.to_string(),
            text: text.clone(),
        })
    }

    /// The titles of `titles` and their redirect targets that this read lacks.
    #[must_use]
    pub fn lacking(&self, titles: &[String]) -> BTreeSet<String> {
        let targets = titles
            .iter()
            .map(|title| self.redirects.get(title).unwrap_or(title));
        targets
            .filter(|title| !self.texts.contains_key(*title))
            .cloned()
            .collect()
    }
}

/// The redirect of a page starts its text, so the reader keeps only this much of the
/// text of a page that nobody wants.
const REDIRECT_HEAD_BYTES: usize = 1024;

/// When a read of the dump ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// A plain read stops when it holds every wanted article.
    AllFound,
    /// A scan reads to the end, for the redirects.
    End,
}

/// The pages with the wanted titles, keyed by the wanted title. A redirect page gives its
/// target, one step only, so a loop of redirects cannot run forever. A missing page has
/// no entry. The dump is read at most twice.
///
/// # Errors
///
/// Returns an error when the dump cannot be read or is not a MediaWiki XML export.
pub fn pages(dump: &Path, titles: &[String]) -> Result<BTreeMap<String, Page>, DumpError> {
    let wanted: BTreeSet<String> = titles.iter().cloned().collect();
    let first = scan(dump, &wanted)?;
    let more = texts(dump, &first.lacking(titles))?;
    let found = titles
        .iter()
        .filter_map(|title| Some((title.clone(), first.page(title, &more)?)));
    Ok(found.collect())
}

/// The wanted articles and every redirect, in one read of the whole dump.
///
/// # Errors
///
/// Returns an error when the dump cannot be read or is not a MediaWiki XML export.
pub fn scan(dump: &Path, wanted: &BTreeSet<String>) -> Result<Scan, DumpError> {
    read_dump(dump, wanted, Stop::End)
}

/// The raw wikitext of the wanted articles, with no redirect followed. With nothing
/// wanted, the dump is not read.
///
/// # Errors
///
/// Returns an error when the dump cannot be read or is not a MediaWiki XML export.
pub fn texts(
    dump: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, DumpError> {
    if wanted.is_empty() {
        return Ok(BTreeMap::new());
    }
    Ok(read_dump(dump, wanted, Stop::AllFound)?.texts)
}

fn read_dump(dump: &Path, wanted: &BTreeSet<String>, stop: Stop) -> Result<Scan, DumpError> {
    if has_extension(dump, "7z") {
        return read_archive(dump, wanted, stop);
    }
    read_xml(BufReader::new(File::open(dump)?), wanted, stop)
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
}

fn read_archive(dump: &Path, wanted: &BTreeSet<String>, stop: Stop) -> Result<Scan, DumpError> {
    let mut archive = ArchiveReader::open(dump, Password::empty())?;
    let mut result = None;
    archive.for_each_entries(|entry, reader| {
        if result.is_some() {
            return Ok(false);
        }
        if entry.is_directory() || !has_extension(Path::new(entry.name()), "xml") {
            return Ok(true);
        }
        result = Some(read_xml(BufReader::new(reader), wanted, stop));
        Ok(false)
    })?;
    result.unwrap_or(Err(DumpError::NoXml))
}

/// The element of a page whose text the reader keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Title,
    Namespace,
    Text,
    Other,
}

#[derive(Default)]
struct PageParts {
    title: String,
    namespace: String,
    text: String,
}

/// The raw wikitext of the wanted articles in an XML export. The reading stops when all
/// of them are found.
///
/// # Errors
///
/// Returns an error when the XML is broken or cut short.
pub fn xml_texts(
    source: impl BufRead,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, DumpError> {
    Ok(read_xml(source, wanted, Stop::AllFound)?.texts)
}

/// The wanted articles and every redirect of an XML export.
///
/// # Errors
///
/// Returns an error when the XML is broken or cut short.
pub fn xml_scan(source: impl BufRead, wanted: &BTreeSet<String>) -> Result<Scan, DumpError> {
    read_xml(source, wanted, Stop::End)
}

fn read_xml(
    source: impl BufRead,
    wanted: &BTreeSet<String>,
    stop: Stop,
) -> Result<Scan, DumpError> {
    let mut reader = Reader::from_reader(source);
    let mut buffer = Vec::new();
    let mut scan = Scan::default();
    let mut page = PageParts::default();
    let mut field = Field::Other;
    let mut depth = 0usize;
    while stop == Stop::End || scan.texts.len() < wanted.len() {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(start) => {
                depth += 1;
                field = field_of(start.local_name().as_ref());
                if start.local_name().as_ref() == "page" {
                    page = PageParts::default();
                }
            }
            Event::End(end) => {
                depth = depth.saturating_sub(1);
                field = Field::Other;
                if end.local_name().as_ref() == "page" {
                    keep(&mut scan, std::mem::take(&mut page), wanted);
                }
            }
            Event::Text(text) => page.push(field, &text.xml10_content(), wanted),
            Event::CData(data) => page.push(field, &data.xml10_content(), wanted),
            Event::GeneralRef(reference) => page.push(field, &entity(&reference)?, wanted),
            Event::Eof if depth > 0 => return Err(DumpError::Truncated),
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(scan)
}

fn field_of(name: &str) -> Field {
    match name {
        "title" => Field::Title,
        "ns" => Field::Namespace,
        "text" => Field::Text,
        _ => Field::Other,
    }
}

impl PageParts {
    /// The title comes before the text in an export, so the text of a page that nobody
    /// wants is never copied past its head.
    fn push(&mut self, field: Field, text: &str, wanted: &BTreeSet<String>) {
        match field {
            Field::Title => self.title.push_str(text),
            Field::Namespace => self.namespace.push_str(text),
            Field::Text if wanted.contains(&self.title) => self.text.push_str(text),
            Field::Text if self.text.len() < REDIRECT_HEAD_BYTES => self.text.push_str(text),
            Field::Text | Field::Other => {}
        }
    }
}

fn keep(scan: &mut Scan, page: PageParts, wanted: &BTreeSet<String>) {
    if page.namespace.trim() != ARTICLES {
        return;
    }
    if let Some(target) = redirect_target(&page.text) {
        scan.redirects.entry(page.title.clone()).or_insert(target);
    }
    if wanted.contains(&page.title) && !scan.texts.contains_key(&page.title) {
        scan.texts.insert(page.title, page.text);
    }
}

fn entity(reference: &BytesRef<'_>) -> Result<String, DumpError> {
    if let Some(character) = reference.resolve_char_ref()? {
        return Ok(character.to_string());
    }
    let name = reference.xml10_content();
    match resolve_predefined_entity(&name) {
        Some(text) => Ok(text.to_string()),
        None => Err(DumpError::Entity(name.into_owned())),
    }
}
