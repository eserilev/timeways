use timeways_story::check::{
    COPIED_WORDS, copies_a_sample, in_voice, names_after_cutoff, plain_text,
};
use timeways_story::moments::Moment;
use timeways_story::samples::{Voice, every_sample, rotated};
use timeways_story::talk::{self, Scene};
use timeways_story::{chronicle, narrator};

/// Each voice, with the most characters and words that its prompt allows.
const VOICES: [(Voice, usize, usize); 3] = [
    (Voice::NarratorLine, narrator::MAX_LINE_CHARS, 25),
    (Voice::Chapter, chronicle::MAX_CHAPTER_CHARS, 80),
    (Voice::NpcReply, talk::MAX_SAY_CHARS, 60),
];

#[test]
fn the_narrator_has_four_samples_a_chapter_five_and_an_npc_reply_three() {
    assert_eq!(Voice::NarratorLine.samples().len(), 4);
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
    assert_eq!(rotated(Voice::NarratorLine, 3), [all[3], all[0], all[1]]);
}

#[test]
fn the_last_turn_of_all_still_picks_samples() {
    assert_eq!(rotated(Voice::Chapter, usize::MAX).len(), 2);
}

#[test]
fn a_copy_of_a_sample_is_refused() {
    let sample = Voice::NarratorLine.samples()[0];
    let say = Voice::NpcReply.samples()[0];

    assert_eq!(narrator::checked_line(sample, ""), None);
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
    let moment = Moment::LevelUp { level: 20 };
    let scene = Scene {
        npc: "Marshal Dughan",
        ..Scene::default()
    };

    let line = narrator::prompt(&moment, 1);
    let talk = talk::prompt(&scene, &[], "hello", 2);

    for sample in rotated(Voice::NarratorLine, 1) {
        assert!(line.contains(sample), "{sample}");
    }
    for sample in rotated(Voice::NpcReply, 2) {
        assert!(talk.contains(sample), "{sample}");
    }
    assert!(!talk.contains(Voice::NarratorLine.samples()[1]), "{talk}");
}
