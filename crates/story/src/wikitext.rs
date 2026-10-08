//! The wikitext of a wiki page: its sections, its redirect, its book, and its plain words.

use crate::check::words_of;

/// The marks that `plain` never leaves, even from broken markup.
const MARKUP_MARKS: [&str; 5] = ["[[", "]]", "{{", "}}", "''"];

/// A link to one of these is a picture or a list of pages, not a word of the text.
const NO_TEXT_LINKS: [&str; 3] = ["File:", "Image:", "Category:"];

/// Templates that show the name of a character, a creature, a quest, or an item.
const NAME_TEMPLATES: [&str; 6] = ["npc", "mob", "quest", "loot", "item", "spell"];

/// `&amp;` goes last, so `&amp;mdash;` stays the text "&mdash;".
const ENTITIES: [(&str, &str); 8] = [
    ("&nbsp;", " "),
    ("&mdash;", "—"),
    ("&ndash;", "–"),
    ("&hellip;", "…"),
    ("&quot;", "\""),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&amp;", "&"),
];

/// The part of a page under one heading. The text before the first heading has no heading.
#[derive(Debug, PartialEq, Eq)]
pub struct Section<'a> {
    pub heading: Option<&'a str>,
    /// The number of `=` marks on each side of the heading, and 0 with no heading.
    pub level: usize,
    pub body: &'a str,
}

/// The sections in page order. A subsection is a section of its own.
#[must_use]
pub fn sections(text: &str) -> Vec<Section<'_>> {
    let mut sections = Vec::new();
    let mut heading = None;
    let mut level = 0;
    let mut start = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        // The first line of a page is never a heading, as in the prototype of the builder.
        if offset > 0
            && let Some((next_level, next)) = heading_of(line)
        {
            sections.push(Section {
                heading,
                level,
                body: &text[start..offset],
            });
            heading = Some(next);
            level = next_level;
            start = offset + line.len();
        }
        offset += line.len();
    }
    sections.push(Section {
        heading,
        level,
        body: &text[start..],
    });
    sections
}

/// `== Name ==` at any level, with the same number of marks on each side.
fn heading_of(line: &str) -> Option<(usize, &str)> {
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
    Some((opening, name))
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

/// The page title of the start of a link target: `:page_name#Part|label]]` gives
/// `Page name`. MediaWiki makes the first letter upper case.
fn page_title(link: &str) -> String {
    let end = link.find(['|', ']', '#']).unwrap_or(link.len());
    let title = link[..end].trim().trim_start_matches(':');
    let title = title.replace('_', " ");
    let mut letters = title.trim().chars();
    let Some(first) = letters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(letters).collect()
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
    let content = arguments(chosen)
        .into_iter()
        .find_map(|argument| argument.trim_start().strip_prefix("content="))?;
    Some(content.trim())
}

/// The inside of each `{{Book|...}}` call, with its nested calls. A call that never
/// closes runs to the end of the page. A template such as `{{Bookshelf}}` is no book.
fn book_blocks(text: &str) -> Vec<&str> {
    template_blocks(text, "Book")
}

/// The inside of each call of the template `name`, with its nested calls. A call that
/// never closes runs to the end of the page.
fn template_blocks<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("{{") {
        let start = from + found;
        if !is_call_of(&text[start + "{{".len()..], name) {
            from = start + "{{".len();
            continue;
        }
        let (inside_end, next) = match closing_braces(text, start) {
            Some(end) => (end - "}}".len(), end),
            None => (text.len(), text.len()),
        };
        blocks.push(&text[start + "{{".len()..inside_end]);
        from = next;
    }
    blocks
}

/// The name of a template takes either case in its first letter. The name ends at a `|`
/// or a line break, so `{{Bookshelf}}` is no call of `Book`.
fn is_call_of(call: &str, name: &str) -> bool {
    let (upper, lower) = first_letter_cases(name);
    let Some(rest) = call
        .strip_prefix(&upper)
        .or_else(|| call.strip_prefix(&lower))
    else {
        return false;
    };
    rest.trim_start_matches(' ').starts_with(['|', '\n'])
}

/// `Npcbox` gives `Npcbox` and `npcbox`.
fn first_letter_cases(name: &str) -> (String, String) {
    let mut letters = name.chars();
    let Some(first) = letters.next() else {
        return (String::new(), String::new());
    };
    let rest = letters.as_str();
    let upper = first.to_uppercase().chain(rest.chars()).collect();
    let lower = first.to_lowercase().chain(rest.chars()).collect();
    (upper, lower)
}

/// The fields of the first call of the template `name` on a page, such as an infobox:
/// `| faction = [[Alliance]]` gives `("faction", "[[Alliance]]")`. A key is in lower case,
/// and a value keeps its markup. None when the page has no such call.
#[must_use]
pub fn template_fields(text: &str, name: &str) -> Option<Vec<(String, String)>> {
    let block = template_blocks(text, name).into_iter().next()?;
    let fields = arguments(block)
        .into_iter()
        .skip(1)
        .filter_map(|argument| argument.split_once('='))
        .map(|(key, value)| (key.trim().to_lowercase(), value.trim().to_string()))
        .collect();
    Some(fields)
}

/// The pages that one line of wikitext links to, and the pages that its references cite,
/// each in the order of the line. A link inside a reference is a citation, not a link.
/// `parts` cuts the line after each group of references, so a sentence gets the references
/// right after it (`of_sentence`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cites {
    pub links: Vec<String>,
    pub refs: Vec<String>,
    pub parts: Vec<CitedPart>,
}

/// A part of a line up to the end of a group of references: its plain text, its links, and
/// the pages that the group cites.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CitedPart {
    pub text: String,
    pub links: Vec<String>,
    pub refs: Vec<String>,
}

/// The last words of a sentence that find its part.
const SENTENCE_TAIL_WORDS: usize = 4;

impl Cites {
    /// The links and the references of the part that ends the sentence: "brought the
    /// Grimtotem to justice.<ref>...</ref>" cites what its own group cites, never a group of
    /// another sentence. Cites with no parts, made by hand, are one part.
    #[must_use]
    pub fn of_sentence(&self, sentence: &str) -> Cites {
        if self.parts.is_empty() {
            return self.clone();
        }
        let words = words_of(sentence);
        let tail = &words[words.len().saturating_sub(SENTENCE_TAIL_WORDS)..];
        let part = self.parts.iter().find(|part| {
            let part_words = words_of(&part.text);
            !tail.is_empty() && part_words.windows(tail.len()).any(|window| window == tail)
        });
        part.map(|part| Cites {
            links: part.links.clone(),
            refs: part.refs.clone(),
            parts: Vec::new(),
        })
        .unwrap_or_default()
    }
}

#[must_use]
pub fn cites(line: &str) -> Cites {
    let mut cites = Cites::default();
    let mut part = CitedPart::default();
    let mut rest = line;
    while let Some(start) = rest.find("<ref") {
        let before = &rest[..start];
        if !before.trim().is_empty() && !part.refs.is_empty() {
            cites.parts.push(std::mem::take(&mut part));
        }
        part.text.push_str(&plain(before));
        part.links.extend(link_targets(before));
        let span = &rest[start..];
        let Some(len) = reference_end(span) else {
            rest = &span["<ref".len()..];
            continue;
        };
        part.refs.extend(link_targets(&span[..len]));
        rest = &span[len..];
    }
    if !rest.trim().is_empty() && !part.refs.is_empty() {
        cites.parts.push(std::mem::take(&mut part));
    }
    part.text.push_str(&plain(rest));
    part.links.extend(link_targets(rest));
    cites.parts.push(part);
    cites.links = cites
        .parts
        .iter()
        .flat_map(|part| part.links.clone())
        .collect();
    cites.refs = cites
        .parts
        .iter()
        .flat_map(|part| part.refs.clone())
        .collect();
    cites
}

/// The page of each `[[...]]` link, with no picture and no category.
fn link_targets(text: &str) -> Vec<String> {
    text.split("[[")
        .skip(1)
        .map(page_title)
        .filter(|title| !title.is_empty() && !is_no_text_link(title))
        .collect()
}

fn is_no_text_link(title: &str) -> bool {
    NO_TEXT_LINKS.iter().any(|kind| title.starts_with(kind))
}

/// The arguments of a template call, split at each `|` outside a nested call or link.
fn arguments(block: &str) -> Vec<&str> {
    let bytes = block.as_bytes();
    let mut arguments = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut at = 0;
    while at < bytes.len() {
        let pair = &bytes[at..bytes.len().min(at + 2)];
        if pair == b"{{" || pair == b"[[" {
            depth += 1;
            at += 2;
        } else if pair == b"}}" || pair == b"]]" {
            depth = depth.saturating_sub(1);
            at += 2;
        } else {
            if bytes[at] == b'|' && depth == 0 {
                arguments.push(&block[start..at]);
                start = at + 1;
            }
            at += 1;
        }
    }
    arguments.push(&block[start..]);
    arguments
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
    arguments(block).get(1).map_or("", |title| title.trim())
}

/// The words of wikitext with the markup gone: references, comments, HTML tags,
/// templates, tables, pictures, and bold and italic marks. A link keeps its label, and a
/// template that names a thing keeps the name. An HTML entity becomes its character.
#[must_use]
pub fn plain(text: &str) -> String {
    let text = remove_spans(text, "<ref", reference_end);
    let text = remove_spans(&text, "<!--", |span| span_end(span, "<!--", "-->"));
    let text = remove_spans(&text, "<gallery", |span| {
        span_end(span, "<gallery", "</gallery>")
    });
    let text = remove_spans(&text, "<", tag_end);
    let text = replace_innermost(&text, "{{", "}}", |inside| Some(template_text(inside)));
    let text = remove_spans(&text, "{|", |span| span_end(span, "{|", "|}"));
    let text = replace_innermost(&text, "[[", "]]", |inside| Some(link_text(inside)));
    let text = replace_innermost(&text, "[", "]", external_link_text);
    let text = remove_quote_runs(&text);
    decode_entities(&remove_marks(text))
}

/// `{{npc|Horde|Thrall}}` shows "Thrall": the last argument with words and no `=`. Any
/// other template shows nothing.
fn template_text(inside: &str) -> String {
    let mut parts = inside.split('|');
    let name = parts.next().unwrap_or_default().trim();
    if !NAME_TEMPLATES
        .iter()
        .any(|template| template.eq_ignore_ascii_case(name))
    {
        return String::new();
    }
    parts
        .map(str::trim)
        .rev()
        .find(|part| !part.is_empty() && !part.contains('='))
        .unwrap_or_default()
        .to_string()
}

/// Only named entities: a number entity can spell a markup mark.
fn decode_entities(text: &str) -> String {
    ENTITIES
        .iter()
        .fold(text.to_string(), |text, (entity, character)| {
            text.replace(entity, character)
        })
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
