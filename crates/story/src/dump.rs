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

/// The pages with the wanted titles, keyed by the wanted title. A redirect page gives its
/// target, one step only, so a loop of redirects cannot run forever. A missing page has
/// no entry.
///
/// # Errors
///
/// Returns an error when the dump cannot be read or is not a MediaWiki XML export.
pub fn pages(dump: &Path, titles: &[String]) -> Result<BTreeMap<String, Page>, DumpError> {
    let wanted: BTreeSet<String> = titles.iter().cloned().collect();
    let found = texts(dump, &wanted)?;
    let redirects: BTreeMap<&String, String> = found
        .iter()
        .filter_map(|(title, text)| Some((title, redirect_target(text)?)))
        .collect();
    let targets: BTreeSet<String> = redirects.values().cloned().collect();
    let target_texts = if targets.is_empty() {
        BTreeMap::new()
    } else {
        texts(dump, &targets)?
    };
    let mut pages = BTreeMap::new();
    for (title, text) in &found {
        let page = match redirects.get(title) {
            Some(target) => target_texts.get(target).map(|text| Page {
                title: target.clone(),
                text: text.clone(),
            }),
            None => Some(Page {
                title: title.clone(),
                text: text.clone(),
            }),
        };
        if let Some(page) = page {
            pages.insert(title.clone(), page);
        }
    }
    Ok(pages)
}

/// The raw wikitext of the wanted articles, with no redirect followed.
///
/// # Errors
///
/// Returns an error when the dump cannot be read or is not a MediaWiki XML export.
pub fn texts(
    dump: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, DumpError> {
    if has_extension(dump, "7z") {
        return archive_texts(dump, wanted);
    }
    xml_texts(BufReader::new(File::open(dump)?), wanted)
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
}

fn archive_texts(
    dump: &Path,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, DumpError> {
    let mut archive = ArchiveReader::open(dump, Password::empty())?;
    let mut result = None;
    archive.for_each_entries(|entry, reader| {
        if result.is_some() {
            return Ok(false);
        }
        if entry.is_directory() || !has_extension(Path::new(entry.name()), "xml") {
            return Ok(true);
        }
        result = Some(xml_texts(BufReader::new(reader), wanted));
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
    let mut reader = Reader::from_reader(source);
    let mut buffer = Vec::new();
    let mut found = BTreeMap::new();
    let mut page = PageParts::default();
    let mut field = Field::Other;
    let mut depth = 0usize;
    while found.len() < wanted.len() {
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
                    keep_if_wanted(&mut found, std::mem::take(&mut page), wanted);
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
    Ok(found)
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
    /// wants is never copied.
    fn push(&mut self, field: Field, text: &str, wanted: &BTreeSet<String>) {
        match field {
            Field::Title => self.title.push_str(text),
            Field::Namespace => self.namespace.push_str(text),
            Field::Text if wanted.contains(&self.title) => self.text.push_str(text),
            Field::Text | Field::Other => {}
        }
    }
}

fn keep_if_wanted(
    found: &mut BTreeMap<String, String>,
    page: PageParts,
    wanted: &BTreeSet<String>,
) {
    let is_wanted = wanted.contains(&page.title) && page.namespace.trim() == ARTICLES;
    if is_wanted && !found.contains_key(&page.title) {
        found.insert(page.title, page.text);
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
