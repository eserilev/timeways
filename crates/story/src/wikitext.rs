//! The wikitext of a wiki page: its sections, its redirect, its book, and its plain words.

/// The marks that `plain` never leaves, even from broken markup.
const MARKUP_MARKS: [&str; 5] = ["[[", "]]", "{{", "}}", "''"];

/// A link to one of these is a picture or a list of pages, not a word of the text.
const NO_TEXT_LINKS: [&str; 3] = ["File:", "Image:", "Category:"];

/// The part of a page under one heading. The text before the first heading has no heading.
#[derive(Debug, PartialEq, Eq)]
pub struct Section<'a> {
    pub heading: Option<&'a str>,
    pub body: &'a str,
}

/// The sections in page order. A subsection is a section of its own.
#[must_use]
pub fn sections(text: &str) -> Vec<Section<'_>> {
    let mut sections = Vec::new();
    let mut heading = None;
    let mut start = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        // The first line of a page is never a heading, as in the prototype of the builder.
        if offset > 0
            && let Some(next) = heading_of(line)
        {
            sections.push(Section {
                heading,
                body: &text[start..offset],
            });
            heading = Some(next);
            start = offset + line.len();
        }
        offset += line.len();
    }
    sections.push(Section {
        heading,
        body: &text[start..],
    });
    sections
}

/// `== Name ==` at any level, with the same number of marks on each side.
fn heading_of(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\n', '\r', ' ', '\t']);
    let opening = line.len() - line.trim_start_matches('=').len();
    let closing = line.len() - line.trim_end_matches('=').len();
    if opening < 2 || opening != closing || opening * 2 >= line.len() {
        return None;
    }
    let name = line[opening..line.len() - closing].trim();
    if name.is_empty() || name.contains('=') {
        return None;
    }
    Some(name)
}

/// The page that a `#REDIRECT [[Page]]` page points to.
#[must_use]
pub fn redirect_target(text: &str) -> Option<String> {
    let text = text.trim_start();
    let marker = text.get(.."#redirect".len())?;
    if !marker.eq_ignore_ascii_case("#redirect") {
        return None;
    }
    let link = &text[text.find("[[")? + 2..];
    let title = page_title(link);
    (!title.is_empty()).then_some(title)
}

/// The pages of the `* [[Page]]` lines of a list, in order.
#[must_use]
pub fn listed_pages(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix('*'))
        .filter_map(|line| line.trim_start_matches(['*', ' ', '\t']).strip_prefix("[["))
        .map(page_title)
        .filter(|title| !title.is_empty())
        .collect()
}

/// The page title of the start of a link target: `:Page_name#Part|label]]` gives
/// `Page name`.
fn page_title(link: &str) -> String {
    let end = link.find(['|', ']', '#']).unwrap_or(link.len());
    let title = link[..end].trim().trim_start_matches(':');
    title.replace('_', " ").trim().to_string()
}

/// The `content=` of the book on a page, as wikitext. A page can hold a copy of the book
/// from a website too, with "(site)" in its title, so a book of the game goes first.
#[must_use]
pub fn book_content(text: &str) -> Option<&str> {
    let books = book_blocks(text);
    let chosen = books
        .iter()
        .find(|block| !book_title(block).contains("(site)"))
        .or(books.first())?;
    let content = chosen.find("content=")?;
    Some(&chosen[content + "content=".len()..])
}

/// The inside of each `{{Book ...}}` call, with its nested calls. A call that never
/// closes runs to the end of the page.
fn book_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("{{Book") {
        let start = from + found;
        let (inside_end, next) = match closing_braces(text, start) {
            Some(end) => (end - "}}".len(), end),
            None => (text.len(), text.len()),
        };
        blocks.push(&text[start + "{{".len()..inside_end]);
        from = next;
    }
    blocks
}

/// The byte after the `}}` that closes the `{{` at `start`.
fn closing_braces(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut at = start;
    while at < bytes.len() {
        if bytes[at..].starts_with(b"{{") {
            depth += 1;
            at += 2;
        } else if bytes[at..].starts_with(b"}}") {
            depth = depth.saturating_sub(1);
            at += 2;
            if depth == 0 {
                return Some(at);
            }
        } else {
            at += 1;
        }
    }
    None
}

/// The first argument of a book call: `Book|Title|content=...` gives `Title`.
fn book_title(block: &str) -> &str {
    block.split('|').nth(1).unwrap_or("")
}

/// The words of wikitext with the markup gone: references, comments, HTML tags,
/// templates, tables, pictures, and bold and italic marks. A link keeps its label.
#[must_use]
pub fn plain(text: &str) -> String {
    let text = remove_spans(text, "<ref", reference_end);
    let text = remove_spans(&text, "<!--", |span| span_end(span, "<!--", "-->"));
    let text = remove_spans(&text, "<", tag_end);
    let text = replace_innermost(&text, "{{", "}}", |_| Some(String::new()));
    let text = remove_spans(&text, "{|", |span| span_end(span, "{|", "|}"));
    let text = replace_innermost(&text, "[[", "]]", |inside| Some(link_text(inside)));
    let text = replace_innermost(&text, "[", "]", external_link_text);
    let text = remove_quote_runs(&text);
    remove_marks(text)
}

/// Removes each span that `span_len` measures from an `open` mark. A mark with no
/// measure stays as text.
fn remove_spans(text: &str, open: &str, span_len: impl Fn(&str) -> Option<usize>) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        kept.push_str(&rest[..start]);
        let span = &rest[start..];
        if let Some(len) = span_len(span) {
            rest = &span[len..];
        } else {
            kept.push_str(open);
            rest = &span[open.len()..];
        }
    }
    kept.push_str(rest);
    kept
}

fn span_end(span: &str, open: &str, close: &str) -> Option<usize> {
    let inside = span[open.len()..].find(close)?;
    Some(open.len() + inside + close.len())
}

/// `<ref name="a"/>` alone, or `<ref>...</ref>` with its words.
fn reference_end(span: &str) -> Option<usize> {
    let tag_close = span.find('>')?;
    if span[..tag_close].ends_with('/') {
        return Some(tag_close + 1);
    }
    span.find("</ref>").map(|close| close + "</ref>".len())
}

/// An HTML tag: `<` with at least one character before the next `>`.
fn tag_end(span: &str) -> Option<usize> {
    let close = span.find('>')?;
    (close > 1).then_some(close + 1)
}

/// Replaces each `open ... close` pair, the innermost first, so nested markup goes from
/// the inside out. `replace` gives None to keep a pair as it is.
fn replace_innermost(
    text: &str,
    open: &str,
    close: &str,
    replace: impl Fn(&str) -> Option<String>,
) -> String {
    let mut text = text.to_string();
    let mut from = 0;
    while let Some(found) = text[from..].find(close) {
        let end = from + found;
        let Some(start) = text[..end].rfind(open) else {
            from = end + close.len();
            continue;
        };
        match replace(&text[start + open.len()..end]) {
            Some(replacement) => {
                text.replace_range(start..end + close.len(), &replacement);
                from = start;
            }
            None => from = end + close.len(),
        }
    }
    text
}

/// `Page|label` gives the label, and `Page` gives the page.
fn link_text(inside: &str) -> String {
    if NO_TEXT_LINKS.iter().any(|kind| inside.starts_with(kind)) {
        return String::new();
    }
    let label = inside.split_once('|').map_or(inside, |(_, label)| label);
    label.to_string()
}

/// `[https://example.test label]` gives the label. Other square brackets stay.
fn external_link_text(inside: &str) -> Option<String> {
    if !(inside.starts_with("http://") || inside.starts_with("https://")) {
        return None;
    }
    let label = inside
        .split_once(char::is_whitespace)
        .map_or("", |(_, label)| label);
    Some(label.to_string())
}

/// Two quote marks or more are italic or bold. One is an apostrophe.
fn remove_quote_runs(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut quotes = 0;
    for character in text.chars() {
        if character == '\'' {
            quotes += 1;
            continue;
        }
        if quotes == 1 {
            kept.push('\'');
        }
        quotes = 0;
        kept.push(character);
    }
    if quotes == 1 {
        kept.push('\'');
    }
    kept
}

/// Broken markup leaves marks with no partner. Removing one mark can join two halves
/// into a new mark, so this repeats until none is left.
fn remove_marks(mut text: String) -> String {
    while let Some(mark) = MARKUP_MARKS.iter().find(|mark| text.contains(*mark)) {
        text = text.replace(mark, "");
    }
    text
}
