#![allow(clippy::unwrap_used)]

use timeways_rules::narrator_shapes::Token;
use timeways_story::arrival::arrival_in;
use timeways_story::check::{banned_words_in, slop_in};
use timeways_story::inside_hero::{inside_hero_in, recognition_in};
use timeways_story::narrator_render::{
    Values, count_number, count_words, ordinal, render, with_article,
};
use timeways_story::narrator_templates::{DEED_KINDS, Kind, TEMPLATES, TemplateError, Templates};

fn templates() -> &'static Templates {
    TEMPLATES.as_ref().unwrap()
}

#[test]
fn the_templates_load() {
    for kind in DEED_KINDS {
        assert!(!templates().shapes(kind).is_empty(), "{kind:?}");
    }
    assert!(templates().shapes(Kind::Arrival).is_empty());
}

/// The bundled data with "{Foe} fell to {hero}." written with another word before the hero.
fn with_word_before_the_hero(word: &str) -> String {
    let bundled = include_str!("../data/narrator_templates.toml");
    let deed = "{Foe} fell to {hero}.";
    assert!(bundled.contains(deed));
    bundled.replace(deed, &format!("{{Foe}} fell {word} {{hero}}."))
}

#[test]
fn an_inside_word_before_the_hero_fails_the_load_in_any_case() {
    for word in ["in", "In", "INSIDE", "Within", "Through", "iNTO"] {
        let loaded = Templates::load(&with_word_before_the_hero(word));

        assert!(
            matches!(loaded, Err(TemplateError::Table(Kind::Kill))),
            "{word}"
        );
    }
}

#[test]
fn another_word_before_the_hero_still_loads() {
    assert!(Templates::load(&with_word_before_the_hero("before")).is_ok());
}

/// Each part alone, with its slots as plain words, holds no banned word, no slop, no
/// arrival of the hero, and nothing inside the hero.
#[test]
fn every_template_part_passes_the_line_checks() {
    let values = Values {
        lore: String::new(),
        hero: Some("$N".to_string()),
        slots: std::collections::HashMap::default(),
    };
    for part in &templates().table.parts {
        let words: Vec<Token> = part
            .tokens
            .iter()
            .map(|token| match token {
                Token::Slot(_) => Token::Hero,
                other => *other,
            })
            .collect();
        let text = render(templates(), &words, &values);
        assert!(banned_words_in(&text).is_empty(), "{text}");
        assert!(slop_in(&text, "").is_empty(), "{text}");
        assert_eq!(arrival_in(&text, &[]), None, "{text}");
        assert_eq!(inside_hero_in(&text, &[]), None, "{text}");
        assert_eq!(recognition_in(&text, &[]), None, "{text}");
    }
}

#[test]
fn counts_and_ordinals_render_in_words_to_ten() {
    assert_eq!(count_words(1), "once");
    assert_eq!(count_words(2), "twice");
    assert_eq!(count_words(3), "three times");
    assert_eq!(count_words(10), "ten times");
    assert_eq!(count_words(11), "11 times");
    assert_eq!(count_number(2), "two");
    assert_eq!(count_number(10), "ten");
    assert_eq!(count_number(11), "11");
    assert_eq!(ordinal(2), "second");
    assert_eq!(ordinal(10), "tenth");
    assert_eq!(ordinal(11), "11th");
    assert_eq!(ordinal(12), "12th");
    assert_eq!(ordinal(13), "13th");
    assert_eq!(ordinal(21), "21st");
    assert_eq!(ordinal(22), "22nd");
    assert_eq!(ordinal(23), "23rd");
    assert_eq!(ordinal(101), "101st");
    assert_eq!(ordinal(111), "111th");
}

#[test]
fn a_mount_takes_a_or_an_by_its_first_letter() {
    assert_eq!(with_article(templates(), "Gray Ram"), "a Gray Ram");
    assert_eq!(with_article(templates(), "Ivory Raptor"), "an Ivory Raptor");
    assert_eq!(with_article(templates(), "Unicorn"), "a Unicorn");
}

#[test]
fn a_line_starts_each_sentence_with_a_capital() {
    let all = templates();
    let shape = &all.shapes(Kind::Kill)[0];
    let tokens = timeways_rules::narrator_shapes::skeleton(&all.table, &shape.shape).unwrap();
    let values = Values {
        lore: "the gnolls of Elwynn raid its farms".to_string(),
        hero: Some("the paladin".to_string()),
        slots: [(
            timeways_story::narrator_templates::Slot::Foe,
            "Hogger".to_string(),
        )]
        .into_iter()
        .collect(),
    };

    let line = render(all, &tokens, &values);

    assert_eq!(
        line,
        "The gnolls of Elwynn raid its farms. Hogger fell to the paladin."
    );
}
