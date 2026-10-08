use timeways_story::check::words_of;
use timeways_story::npc_voice::{
    Asked, MOST_SENTENCE_WORDS, MOST_WORDS, NpcFault, npc_faults, npc_slop_words,
};
use timeways_story::samples::Voice;
use timeways_story::talk::{self, Scene};

fn faults(say: &str) -> Vec<NpcFault> {
    npc_faults(say, Asked::Question, "")
}

fn passes(say: &str) -> bool {
    faults(say).is_empty()
}

#[test]
fn every_npc_sample_passes_the_npc_checks() {
    for sample in Voice::NpcReply.samples() {
        assert_eq!(faults(sample), [], "{sample}");
    }
}

#[test]
fn the_bench_lines_that_the_user_found_cringe_are_refused() {
    let lines = [
        "Road? Trouble lingers, Blackrock orcs and gnolls still raid our shores. We need brave \
         souls to drive them off, but Stormwind's too busy with their own strife. Still, I've \
         taken to standing watch myself, no one's supposed to be left to face the enemy alone.",
        "The road is no safe place these days. Gnoll raiders have been seen along the Redridge \
         trails. If you are passing through, keep your wits about you.",
    ];
    for line in lines {
        assert!(!passes(line), "{line}");
    }
}

#[test]
fn a_ban_phrase_is_refused_and_a_phrase_of_the_prompt_is_allowed() {
    let line = "These are dark times for Lakeshire.";

    assert_eq!(faults(line), [NpcFault::Slop("dark times")]);
    assert_eq!(
        npc_faults(line, Asked::Question, "The Dark Times of Lakeshire began."),
        []
    );
}

#[test]
fn a_ban_phrase_counts_only_within_one_sentence() {
    assert!(passes("The light was dark. Times are hard in Lakeshire."));
}

#[test]
fn a_stock_form_of_address_is_refused_and_a_plain_mention_passes() {
    assert_eq!(
        faults("Greetings, traveler. The inn is full."),
        [NpcFault::Slop("greetings traveler")]
    );
    assert!(passes(
        "A traveler came through last week. He paid in copper."
    ));
}

#[test]
fn every_ban_phrase_is_lower_case_and_unique() {
    let phrases: Vec<&str> = npc_slop_words().collect();
    for phrase in &phrases {
        assert_eq!(*phrase, phrase.to_lowercase(), "{phrase}");
        assert_eq!(phrases.iter().filter(|other| *other == phrase).count(), 1);
        assert!(!words_of(phrase).is_empty(), "{phrase}");
    }
}

#[test]
fn an_answer_that_opens_with_ah_is_refused() {
    for line in ["Ah, the road. It is bad.", "Ahh. The road is bad."] {
        assert!(faults(line).contains(&NpcFault::StockOpener), "{line}");
    }
}

#[test]
fn a_word_that_only_starts_with_ah_opens_no_stock_reply() {
    assert!(passes("Ahead lies the bridge. The gnolls hold it."));
    assert!(passes("The road is bad. Ah, well, we manage."));
}

#[test]
fn an_answer_to_a_question_never_ends_on_a_question() {
    let line = "Gnolls took the bridge. Will you help us?";

    assert_eq!(
        npc_faults(line, Asked::Question, ""),
        [NpcFault::ClosingQuestion("Will you help us?".to_string())]
    );
}

#[test]
fn a_question_inside_an_answer_passes() {
    assert!(passes("The road? Gnolls took the bridge last night."));
}

#[test]
fn a_greeting_gets_a_short_question_back_and_never_a_long_one() {
    let short = "Lakeshire is no place for sightseeing. What do you want?";
    let long = "Lakeshire is quiet today. Have you come all this way to help us fight the gnolls?";

    assert_eq!(npc_faults(short, Asked::NoQuestion, ""), []);
    assert!(matches!(
        npc_faults(long, Asked::NoQuestion, "").as_slice(),
        [NpcFault::ClosingQuestion(_)]
    ));
}

#[test]
fn the_words_of_the_player_tell_whether_they_asked() {
    assert_eq!(Asked::of("Any news from the road?"), Asked::Question);
    assert_eq!(Asked::of("any news from the road"), Asked::Question);
    assert_eq!(Asked::of("where is hogger"), Asked::Question);
    assert_eq!(Asked::of("Hello."), Asked::NoQuestion);
    assert_eq!(Asked::of("I killed the gnolls."), Asked::NoQuestion);
}

#[test]
fn an_answer_over_the_word_limit_is_refused_and_one_at_the_limit_passes() {
    let at_limit = vec!["gnolls"; MOST_WORDS].join(" ");
    let over = format!("{at_limit} again");

    assert!(
        !npc_faults(&at_limit, Asked::Question, "")
            .iter()
            .any(|fault| matches!(fault, NpcFault::TooManyWords(_)))
    );
    assert!(
        npc_faults(&over, Asked::Question, "").contains(&NpcFault::TooManyWords(MOST_WORDS + 1))
    );
}

#[test]
fn an_answer_of_five_sentences_is_refused_and_one_of_four_passes() {
    let four = "Gnolls came. They took the mill. Then the bridge. We wait for the Marshal.";
    let five = format!("{four} He is late.");

    assert!(passes(four));
    assert_eq!(faults(&five), [NpcFault::TooManySentences(5)]);
}

#[test]
fn a_long_spoken_sentence_is_refused_and_one_at_the_limit_passes() {
    let words = |count: usize| vec!["gnolls"; count].join(" ");
    let at_limit = format!("{}.", words(MOST_SENTENCE_WORDS));
    let over = format!("{}.", words(MOST_SENTENCE_WORDS + 1));

    assert!(passes(&at_limit));
    assert!(matches!(
        faults(&over).as_slice(),
        [NpcFault::LongSentence(_)]
    ));
}

#[test]
fn an_npc_never_tells_the_player_how_they_look_or_feel() {
    let lines = [
        ("You look tired. Sit by the fire.", "you look tired"),
        ("You must be weary from the road.", "you must be weary"),
        ("You're lost, I can see.", "you re lost"),
        ("You seem a bit troubled.", "you seem a bit troubled"),
    ];
    for (line, clause) in lines {
        assert_eq!(
            faults(line),
            [NpcFault::PlayerFeeling(clause.to_string())],
            "{line}"
        );
    }
}

#[test]
fn a_you_that_asks_supposes_or_names_tells_no_feeling() {
    for line in [
        "If you're lost, follow the road east.",
        "You must be the one Gryan sent.",
        "You look for Hogger in the wrong hills.",
        "Tired men make mistakes. Get some sleep.",
    ] {
        assert!(passes(line), "{line}");
    }
}

#[test]
fn a_stage_direction_is_refused_and_plain_words_pass() {
    assert_eq!(
        faults("*spits* The Scarlets again."),
        [NpcFault::StageDirection]
    );
    assert!(passes("The Scarlets again. Spit on their banner for me."));
}

#[test]
fn a_heavy_accent_is_refused_and_a_light_one_passes() {
    let heavy = "Aye, lad, ye be wantin' work.";
    let light = "Aye, lad. The troggs came up through the dam.";

    assert!(matches!(faults(heavy).as_slice(), [NpcFault::Caricature(words)] if words.len() == 4));
    assert!(passes(light));
}

#[test]
fn a_talk_prompt_carries_the_manner_of_an_npc() {
    let scene = Scene {
        npc: "Marshal Dughan",
        ..Scene::default()
    };

    let prompt = talk::prompt(&scene, &[], "hello", 0);

    for rule in [
        "Your manner:",
        "Answer the player first.",
        "Never open with \"Ah\".",
        "end on your answer, not a question.",
        "in 1 to 4 short sentences and at most 40 words.",
    ] {
        assert!(prompt.contains(rule), "{rule} is missing: {prompt}");
    }
}

#[test]
fn a_talk_answer_with_a_closing_question_to_a_question_is_dropped() {
    let answer = r#"{"say": "Gnolls took the bridge. Will you help?", "trust": 0}"#;

    assert_eq!(talk::checked_answer(answer, Asked::Question, "", ""), None);
    assert!(talk::checked_answer(answer, Asked::NoQuestion, "", "").is_some());
}
