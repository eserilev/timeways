use hourglass::Tick;
use timeways_story::journal::{Chapter, Deed};
use timeways_story::memory::summary;

fn chapter(deeds: Vec<Deed>) -> Chapter {
    Chapter {
        number: 2,
        began: Tick(100),
        ended: Tick(200),
        zones: Vec::new(),
        people: Vec::new(),
        deeds,
        left_out: 0,
        prose: None,
        footnotes: Vec::new(),
    }
}

fn level(to: i64) -> Deed {
    Deed::Level {
        from: Some(to - 1),
        to,
        at: Tick(110),
        place: None,
    }
}

fn defeated(foe: &str, times: i64) -> Deed {
    Deed::Defeated {
        foe: foe.to_string(),
        times,
        at: Tick(120),
        place: None,
    }
}

fn died() -> Deed {
    Deed::Died {
        killer: None,
        at: Tick(130),
        place: None,
    }
}

#[test]
fn a_summary_tells_the_facts_of_its_chapter_in_one_line() {
    let mut chapter = chapter(vec![
        level(12),
        level(13),
        defeated("Mother Fang", 1),
        Deed::QuestDone {
            title: "The Lost Lantern".to_string(),
            at: Tick(140),
            place: None,
        },
        died(),
    ]);
    chapter.zones = vec!["Westfall".to_string()];
    chapter.people = vec!["Gryan Stoutmantle".to_string()];

    let line = summary(&chapter);

    assert_eq!(
        line,
        "Chapter 2: traveled to Westfall; met Gryan Stoutmantle; reached level 13; defeated \
         Mother Fang; finished the quest \"The Lost Lantern\"; died once."
    );
}

#[test]
fn a_chapter_with_no_facts_is_nothing_of_note() {
    assert_eq!(summary(&chapter(Vec::new())), "Chapter 2: nothing of note.");
}

#[test]
fn a_summary_names_at_most_three_zones_and_three_foes() {
    let mut chapter = chapter(
        ["A", "B", "C", "D"]
            .iter()
            .map(|foe| defeated(foe, 1))
            .collect(),
    );
    chapter.zones = ["W", "X", "Y", "Z"].map(String::from).to_vec();

    let line = summary(&chapter);

    assert_eq!(line, "Chapter 2: traveled to W, X, Y; defeated A, B, C.");
}

#[test]
fn a_repeat_kill_is_no_first_kill_and_deaths_are_counted() {
    let line = summary(&chapter(vec![defeated("Hogger", 2), died(), died()]));

    assert_eq!(line, "Chapter 2: died 2 times.");
}
