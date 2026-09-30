//! Tiny invented wiki dumps in the MediaWiki export format.

use std::path::{Path, PathBuf};

pub const ARTICLE: u32 = 0;

/// A page as a dump holds it: its title, its namespace, and its raw wikitext.
pub struct DumpPage<'a> {
    pub title: &'a str,
    pub namespace: u32,
    pub text: &'a str,
}

pub fn article<'a>(title: &'a str, text: &'a str) -> DumpPage<'a> {
    DumpPage {
        title,
        namespace: ARTICLE,
        text,
    }
}

pub fn xml(pages: &[DumpPage<'_>]) -> String {
    let pages: String = pages.iter().enumerate().map(page_xml).collect();
    format!(
        "<mediawiki xmlns=\"http://www.mediawiki.org/xml/export-0.11/\" version=\"0.11\">\n\
         <siteinfo><sitename>Testwiki</sitename></siteinfo>\n{pages}</mediawiki>\n"
    )
}

fn page_xml((id, page): (usize, &DumpPage<'_>)) -> String {
    format!(
        "<page>\n<title>{}</title>\n<ns>{}</ns>\n<id>{id}</id>\n\
         <revision><id>{id}</id><text bytes=\"1\" xml:space=\"preserve\">{}</text></revision>\n\
         </page>\n",
        escape(page.title),
        page.namespace,
        escape(page.text)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A fresh path in the temporary folder of the tests.
pub fn fresh(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_file(&path);
    path
}

pub fn write_dump(name: &str, pages: &[DumpPage<'_>]) -> PathBuf {
    let path = fresh(&format!("{name}.xml"));
    std::fs::write(&path, xml(pages)).unwrap();
    path
}

/// A paragraph long enough to become a passage.
pub fn long(words: &str) -> String {
    format!("{words} The rest of this invented line only makes it long enough to count as prose.")
}
