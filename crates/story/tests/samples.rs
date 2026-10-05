use timeways_story::check::{
    COPIED_WORDS, copies_a_sample, in_voice, names_after_cutoff, plain_text, slop_in,
};
use timeways_story::line_check::{
    COPIED_LINE_WORDS, Checked, Grounds, LineFault, checked_line, grounded,
};
use timeways_story::moments::Moment;
use timeways_story::narrator::{Telling, Who};
use timeways_story::samples::{LineSample, Voice, every_sample, line_samples, rotated};
use timeways_story::talk::{self, Scene};
use timeways_story::{chronicle, narrator};

/// Each voice, with the most characters and words that its prompt allows.
const VOICES: [(Voice, usize, usize); 3] = [
    (Voice::NarratorLine, narrator::MAX_LINE_CHARS, 30),
    (Voice::Chapter, chronicle::MAX_CHAPTER_CHARS, 80),
    (Voice::NpcReply, talk::MAX_SAY_CHARS, 60),
];

/// What a sample was told from, as the check sees a line of the game.
fn grounds_of(sample: &LineSample) -> Grounds {
    Grounds {
        moment: sample.moment.to_string(),
        names: Vec::new(),
        lore: sample.lore.map(str::to_string),
    }
}

fn shares_a_run(text: &str, other: &str, length: usize) -> bool {
    let words = timeways_story::check::words_of(text);
    let other = timeways_story::check::words_of(other);
    words
        .windows(length)
        .any(|run| other.windows(length).any(|window| window == run))
}

#[test]
fn the_narrator_has_twenty_samples_a_chapter_five_and_an_npc_reply_three() {
    assert_eq!(Voice::NarratorLine.samples().len(), 20);
    assert_eq!(Voice::Chapter.samples().len(), 5);
    assert_eq!(Voice::NpcReply.samples().len(), 3);
}

#[test]
fn every_sample_passes_every_check() {
    for (voice, max_chars, max_words) in VOICES {
        for sample in voice.samples() {
            let others: Vec<&str> = every_sample()
                .into_iter()
                .filter(|other| *other != sample)
                .collect();

            assert_eq!(plain_text(sample, max_chars, 1600).as_deref(), Some(sample));
            assert!(in_voice(sample), "{sample}");
            assert!(names_after_cutoff(sample).is_empty(), "{sample}");
            assert!(!copies_a_sample(sample, &others), "{sample}");
            assert!(sample.split_whitespace().count() <= max_words, "{sample}");
        }
    }
}

#[test]
fn every_narrator_sample_is_grounded_in_its_moment_or_its_lore() {
    for sample in line_samples() {
        assert!(
            grounded(sample.line, &grounds_of(&sample)),
            "{}",
            sample.line
        );
    }
}

#[test]
fn no_narrator_sample_holds_slop_that_its_lore_lacks() {
    for sample in line_samples() {
        let told = format!("{}\n{}", sample.moment, sample.lore.unwrap_or_default());
        assert_eq!(
            slop_in(sample.line, &told),
            Vec::<&str>::new(),
            "{}",
            sample.line
        );
    }
}

#[test]
fn no_narrator_sample_shares_four_words_with_another_sample() {
    for sample in line_samples() {
        for other in every_sample()
            .into_iter()
            .filter(|other| *other != sample.line)
        {
            assert!(
                !shares_a_run(sample.line, other, COPIED_LINE_WORDS),
                "{} / {other}",
                sample.line
            );
        }
    }
}

#[test]
fn every_narrator_sample_has_a_moment_and_a_naming() {
    for sample in line_samples() {
        assert!(!sample.moment.is_empty(), "{sample:?}");
        let naming = sample.hero;
        let known = naming == "$N"
            || naming == "no name"
            || naming.starts_with("the ")
            || naming.starts_with("by the title \"");
        assert!(known, "{sample:?}");
    }
}

#[test]
fn the_narrator_samples_mix_the_name_the_kind_no_name_and_a_title() {
    let samples = line_samples();
    let count = |test: fn(&str) -> bool| samples.iter().filter(|sample| test(sample.hero)).count();

    assert!(count(|hero| hero == "$N") >= 6);
    assert!(count(|hero| hero.starts_with("the ")) >= 6);
    assert!(count(|hero| hero == "no name") >= 2);
    assert!(count(|hero| hero.starts_with("by the title")) >= 1);
}

#[test]
fn a_narrator_sample_names_the_hero_as_it_says() {
    for sample in line_samples() {
        let marks = sample.line.matches("$N").count();
        let expected = usize::from(sample.hero == "$N");
        assert_eq!(marks, expected, "{}", sample.line);
    }
}

#[test]
fn no_sample_of_any_voice_says_our_hero() {
    for sample in every_sample() {
        assert!(!sample.to_lowercase().contains("our hero"), "{sample}");
    }
}

#[test]
fn a_chapter_sample_names_the_hero_at_most_twice() {
    for sample in Voice::Chapter.samples() {
        assert!(sample.matches("$N").count() <= 2, "{sample}");
        assert_eq!(slop_in(sample, ""), Vec::<&str>::new(), "{sample}");
    }
}

#[test]
fn a_narrator_prompt_shows_each_sample_as_a_pair_of_moment_and_line() {
    let moment = Moment::NewZone {
        zone: "Ashenvale".to_string(),
    };
    let telling = Telling {
        moment: &moment,
        lore: None,
        who: &Who::default(),
    };

    let prompt = narrator::prompt(&telling, 0);

    let first = line_samples()[0];
    let pair = format!(
        "Moment: {}\nThe hero: {}\nLore: {}\nName the hero: {}\nLine: {}",
        first.moment,
        first.who.unwrap(),
        first.lore.unwrap(),
        first.hero,
        first.line
    );
    assert!(prompt.contains(&pair), "{prompt}");
}

#[test]
fn a_sample_with_no_lore_shows_none() {
    let murlocs = line_samples()
        .into_iter()
        .find(|sample| sample.lore.is_none())
        .unwrap();
    let index = line_samples()
        .iter()
        .position(|sample| *sample == murlocs)
        .unwrap();
    let moment = Moment::NewZone {
        zone: "Ashenvale".to_string(),
    };
    let telling = Telling {
        moment: &moment,
        lore: None,
        who: &Who::default(),
    };

    let prompt = narrator::prompt(&telling, index);

    assert!(prompt.contains(&format!("Moment: {}\nLore: none\n", murlocs.moment)));
}

#[test]
fn a_prompt_carries_two_or_three_samples() {
    assert_eq!(rotated(Voice::NarratorLine, 0).len(), 3);
    assert_eq!(rotated(Voice::Chapter, 0).len(), 2);
    assert_eq!(rotated(Voice::NpcReply, 0).len(), 2);
}

#[test]
fn the_samples_turn_from_one_prompt_to_the_next() {
    let all = Voice::NarratorLine.samples();

    let first = rotated(Voice::NarratorLine, 0);
    let second = rotated(Voice::NarratorLine, 1);

    assert_eq!(first, all[0..3]);
    assert_eq!(second, all[1..4]);
    assert_eq!(rotated(Voice::NarratorLine, 19), [all[19], all[0], all[1]]);
}

#[test]
fn the_last_turn_of_all_still_picks_samples() {
    assert_eq!(rotated(Voice::Chapter, usize::MAX).len(), 2);
}

#[test]
fn a_copy_of_a_sample_is_refused() {
    let sample = line_samples()[0];
    let say = Voice::NpcReply.samples()[0];

    let checked = checked_line(sample.line, &grounds_of(&sample), "");

    assert!(
        matches!(&checked, Checked::Refused(faults) if faults.iter().any(|fault| matches!(fault, LineFault::Copy(_)))),
        "{checked:?}"
    );
    let answer = format!(r#"{{"say": "{say}", "trust": 0}}"#);
    assert_eq!(talk::checked_answer(&answer, ""), None);
}

#[test]
fn a_phrase_shorter_than_a_copy_passes() {
    let sample = "one two three four five six seven eight nine";
    let short: Vec<&str> = sample.split(' ').take(COPIED_WORDS - 1).collect();
    let long: Vec<&str> = sample.split(' ').skip(1).take(COPIED_WORDS).collect();

    assert!(!copies_a_sample(&short.join(" "), &[sample]));
    assert!(copies_a_sample(&long.join(", ").to_uppercase(), &[sample]));
}

#[test]
fn each_prompt_of_a_voice_carries_its_samples_in_turn() {
    let moment = Moment::NewZone {
        zone: "Ashenvale".to_string(),
    };
    let telling = Telling {
        moment: &moment,
        lore: None,
        who: &Who::default(),
    };
    let scene = Scene {
        npc: "Marshal Dughan",
        ..Scene::default()
    };

    let line = narrator::prompt(&telling, 1);
    let talk = talk::prompt(&scene, &[], "hello", 2);

    for sample in rotated(Voice::NarratorLine, 1) {
        assert!(line.contains(sample), "{sample}");
    }
    for sample in rotated(Voice::NpcReply, 2) {
        assert!(talk.contains(sample), "{sample}");
    }
    assert!(!talk.contains(Voice::NarratorLine.samples()[1]), "{talk}");
}

#[test]
fn a_narrator_prompt_leaves_out_the_sample_of_its_own_moment() {
    let first = line_samples()[0];
    let moment = Moment::NewZone {
        zone: "Tirisfal Glades".to_string(),
    };
    assert_eq!(narrator::what_happened(&moment), first.moment);
    let telling = Telling {
        moment: &moment,
        lore: None,
        who: &Who::default(),
    };

    let prompt = narrator::prompt(&telling, 0);

    assert!(!prompt.contains(first.line), "{prompt}");
}
