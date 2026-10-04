//! The voice regression set (GAMEPLAY.md 3.2.1): fixed moments through the real prompt
//! code. The normal tests measure each prompt. The live test sends the voice moments to a
//! real model, and writes the answers to a file for review.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};
use timeways_story::chronicle::{self, Draft, Pick};
use timeways_story::hero::{self, Entry, Field, Hero, PROMPT_TEXT_CHARS};
use timeways_story::hero_hook::Hook;
use timeways_story::journal::{Chapter, Deed};
use timeways_story::moments::Moment;
use timeways_story::narrator;
use timeways_story::npc_memory::{self, Memory, QuestEnding, RUMOR_CHARS, Recall};
use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::places::InstanceKind;
use timeways_story::prompt::{self, Context};
use timeways_story::quest::{self, Known};
use timeways_story::story::MAX_NAME_BYTES;
use timeways_story::talk::{self, QuestTalk, Scene};
use timeways_story::tokens::{Call, estimated_tokens};
use timeways_story::{check, draft};

/// One fixed moment, its prompt, and what the player sees of an answer.
struct TestMoment {
    name: &'static str,
    call: Call,
    prompt: String,
    shown: fn(&str) -> Option<String>,
}

fn hero() -> Hero {
    let field = |field: &str, text: &str| Field {
        field: field.to_string(),
        text: text.to_string(),
    };
    let entry = |number, text: &str| Entry {
        number,
        at: Tick(number * 100),
        text: text.to_string(),
        place: None,
        npc: None,
    };
    Hero {
        sheet: vec![
            field("origin", "A farmhand from the edge of Elwynn Forest."),
            field(
                "goal",
                "To find the brother who marched north with the army.",
            ),
            field("flaw", "Trusts strangers too fast."),
        ],
        entries: vec![
            entry(1, "A stranger at the inn knew my father's name."),
            entry(2, "I kept the letter from my brother, unread."),
        ],
    }
}

fn portrait() -> Option<String> {
    hero::portrait(&hero())
}

fn line(moment: &Moment) -> String {
    narrator::prompt(moment, 0)
}

fn level(to: i64, at: u64) -> Deed {
    Deed::Level {
        from: Some(to - 1),
        to,
        at: Tick(at),
        place: None,
    }
}

fn chapter(number: usize, zones: &[&str], deeds: Vec<Deed>) -> Chapter {
    Chapter {
        number,
        began: Tick(1000),
        ended: Tick(5000),
        zones: zones.iter().map(ToString::to_string).collect(),
        people: Vec::new(),
        deeds,
        left_out: 0,
        prose: None,
        footnotes: Vec::new(),
    }
}

/// The chapter with a finished side quest, with a small moment and words of the player.
fn side_quest() -> Chapter {
    let mut chapter = chapter(
        4,
        &["Westfall"],
        vec![
            level(16, 1200),
            Deed::QuestDone {
                title: "The Lost Lantern".to_string(),
                at: Tick(1500),
                place: Some("Sentinel Hill".to_string()),
            },
        ],
    );
    chapter.people = vec!["Farmer Saldean".to_string()];
    chapter
}

fn side_quest_chapter(draft: Draft) -> String {
    let moments =
        ["The player used the emote /dance in Sentinel Hill, for the 1st time.".to_string()];
    let told = ["The lantern belonged to my brother."];
    let earlier = earlier_chapters();
    chronicle::draft_prompt(
        &[],
        &side_quest(),
        &earlier,
        &moments,
        portrait().as_deref(),
        &told,
        draft,
    )
}

/// Two drafts of the longest size that the checks let through.
fn judge() -> String {
    let chapter = chapter(4, &["Westfall"], vec![level(16, 1200)]);
    let draft = "w".repeat(chronicle::MAX_CHAPTER_CHARS);
    chronicle::judge_prompt(4, &chronicle::facts(&[], &chapter), &draft, &draft)
}

fn quiet_chapter() -> String {
    let chapter = chapter(5, &["Redridge Mountains"], Vec::new());
    let earlier = earlier_chapters();
    chronicle::prompt(&[], &chapter, &earlier, &[], portrait().as_deref(), &[])
}

/// The three chapters that a chapter prompt recalls.
fn earlier_chapters() -> Vec<Chapter> {
    let first_kill = Deed::Defeated {
        foe: "Hogger".to_string(),
        times: 1,
        at: Tick(300),
        place: None,
    };
    vec![
        chapter(1, &["Elwynn Forest"], vec![level(10, 100), first_kill]),
        chapter(2, &["Westfall", "Moonbrook"], vec![level(12, 400)]),
        chapter(3, &["Duskwood"], Vec::new()),
    ]
}

fn passage(text: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![Link::Place("Goldshire".to_string())],
        origin: Origin::Pack,
    }
}

/// A hook at full length, so the budget test covers it.
fn longest_hook() -> Hook<'static> {
    const LONGEST: &str = "I swore to find the stranger in the grey cloak who knew my father, \
        and to learn why he left Goldshire on the night of the fire. I ask every innkeeper and \
        each guard. I keep his letter in my boot, and I read it each night by the fire of the \
        inn, though I know each word of it by heart now. Every word.";
    assert_eq!(LONGEST.chars().count(), PROMPT_TEXT_CHARS);
    Hook {
        field: "goal",
        text: LONGEST,
    }
}

/// Five memories at full length, so the budget test covers the longest block.
fn longest_memories() -> Vec<String> {
    let name = "W".repeat(MAX_NAME_BYTES);
    let rumor = || Recall::Said {
        text: format!("{}...", "word ".repeat(RUMOR_CHARS / 5).trim_end()),
    };
    let recalls = [
        rumor(),
        rumor(),
        Recall::GaveQuest {
            title: "T".repeat(quest::MAX_TITLE_CHARS),
            ending: QuestEnding::Waiting,
        },
        Recall::DefeatedNear {
            foe: name.clone(),
            place: name.clone(),
        },
        Recall::DiedNear {
            place: name.clone(),
            killer: Some(name),
        },
    ];
    recalls
        .into_iter()
        .map(|recall| {
            let memory = Memory {
                at: Tick(0),
                recall,
                sources: Vec::new(),
            };
            npc_memory::line(&memory, Tick(1))
        })
        .collect()
}

fn npc_talk() -> String {
    let scene = Scene {
        npc: "Innkeeper Farley",
        place: Some("Goldshire"),
        level: Some(12),
        trust: Some(-20),
        slapped: Some(2),
        own_lore: vec!["A stranger at the inn knew my father's name."],
        memories: longest_memories(),
        // A title and a topic at their longest: 60 characters each.
        quests: vec![QuestTalk {
            giver: "Marshal Dughan",
            title: "The Stranger in the Grey Cloak and the Letter Left Unread...",
            about: Some("the stranger in the grey cloak who asked for my fathers name"),
        }],
        hook: Some(longest_hook()),
    };
    let lore = [passage(
        "The Lion's Pride Inn stands at the crossroads of Goldshire.",
    )];
    talk::prompt(
        &scene,
        &lore,
        "Have you seen a stranger in a grey cloak?",
        0,
    )
}

fn quest_known() -> Known<'static> {
    Known {
        giver: "Innkeeper Farley",
        zones: vec!["Elwynn Forest", "Westfall"],
        subzones: vec!["Goldshire", "Fargodeep Mine", "Sentinel Hill"],
        npcs: vec!["Marshal Dughan", "Farmer Saldean"],
        foes: vec!["Defias Thug", "Riverpaw Gnoll"],
        last_targets: Vec::new(),
        seen: &[],
    }
}

fn quest_offer() -> String {
    quest::prompt(&quest_known(), Some("Goldshire"), Some(longest_hook()))
}

fn draft_known() -> draft::Known<'static> {
    draft::Known {
        zones: vec!["Elwynn Forest", "Westfall"],
        subzones: vec!["Goldshire", "Fargodeep Mine"],
        npcs: vec!["Marshal Dughan", "Farmer Saldean"],
        foes: vec!["Defias Thug", "Hogger"],
    }
}

fn task_draft() -> String {
    draft::prompt(
        &draft_known(),
        "get my friend to kill hogger and then meet me in goldshire",
    )
}

fn lore_question() -> String {
    let context = Context {
        places: vec!["Goldshire", "Elwynn Forest"],
        target: Some("Marshal Dughan"),
        level: Some(12),
    };
    let passages = [
        passage("The Defias Brotherhood began as the Stonemasons' Guild."),
        passage("The stonemasons rebuilt Stormwind after the Second War."),
        passage("The House of Nobles refused to pay the stonemasons."),
    ];
    prompt::lore("Who are the Defias?", &context, &passages)
}

fn narrator_shown(answer: &str) -> Option<String> {
    narrator::checked_line(answer, "")
}

fn saga_shown(answer: &str) -> Option<String> {
    let saga = chronicle::checked_saga(answer, 1, "")?;
    let footnotes: Vec<String> = saga.footnotes.into_iter().map(|(_, text)| text).collect();
    Some(format!(
        "{} | Footnotes: {}",
        saga.text,
        footnotes.join(" / ")
    ))
}

fn talk_shown(answer: &str) -> Option<String> {
    let answer = talk::checked_answer(answer, "")?;
    Some(format!("{} (trust {:+})", answer.say, answer.trust_change))
}

fn narrator_moment(name: &'static str, moment: &Moment) -> TestMoment {
    TestMoment {
        name,
        call: Call::NarratorLine,
        prompt: line(moment),
        shown: narrator_shown,
    }
}

/// The fixed moments of the regression set, in the order of GAMEPLAY.md 3.2.1.
fn voice_moments() -> Vec<TestMoment> {
    let zone = |zone: &str| Moment::NewZone {
        zone: zone.to_string(),
    };
    let chapter = |name, prompt| TestMoment {
        name,
        call: Call::Chapter,
        prompt,
        shown: saga_shown,
    };
    vec![
        narrator_moment(
            "a first dungeon",
            &Moment::FirstInstance {
                zone: "The Deadmines".to_string(),
                kind: InstanceKind::Dungeon,
            },
        ),
        narrator_moment(
            "a world boss",
            &Moment::FirstKill {
                foe: "Azuregos".to_string(),
            },
        ),
        narrator_moment(
            "a death",
            &Moment::SlainAgain {
                killer: "Defias Pillager".to_string(),
                times: 2,
            },
        ),
        narrator_moment("a level milestone", &Moment::LevelUp { level: 20 }),
        narrator_moment(
            "a new capital",
            &Moment::FirstCapital {
                city: "Ironforge".to_string(),
            },
        ),
        narrator_moment("a new zone", &zone("Westfall")),
        narrator_moment(
            "a finished class quest",
            &Moment::ClassQuestDone {
                title: "The Tome of Valor".to_string(),
            },
        ),
        narrator_moment(
            "a quest mark",
            &Moment::QuestMarked {
                mark: "Blessing of the Light".to_string(),
                quest: "The Tome of Valor".to_string(),
            },
        ),
        chapter("a finished side quest", side_quest_chapter(Draft::First)),
        chapter("a quiet chapter", quiet_chapter()),
        TestMoment {
            name: "an NPC talk",
            call: Call::Talk,
            prompt: npc_talk(),
            shown: talk_shown,
        },
    ]
}

/// The voice moments, and one prompt of each other kind of call.
fn every_prompt() -> Vec<(&'static str, Call, String)> {
    let mut prompts: Vec<(&'static str, Call, String)> = voice_moments()
        .into_iter()
        .map(|moment| (moment.name, moment.call, moment.prompt))
        .collect();
    prompts.push(("a quest offer", Call::Quest, quest_offer()));
    prompts.push(("a lore question", Call::Lore, lore_question()));
    prompts.push(("a judge of two drafts", Call::Judge, judge()));
    prompts.push(("a task draft", Call::Quest, task_draft()));
    prompts
}

#[test]
fn every_prompt_fits_a_local_model_with_a_context_of_2048_tokens() {
    let mut over = Vec::new();

    for (name, call, prompt) in every_prompt() {
        let tokens = estimated_tokens(&prompt);
        if tokens > call.prompt_budget() {
            over.push(format!("{name}: {tokens} of {}", call.prompt_budget()));
        }
    }

    assert!(over.is_empty(), "{over:?}");
}

#[test]
fn a_token_is_about_four_characters() {
    assert_eq!(estimated_tokens(""), 0);
    assert_eq!(estimated_tokens("abcd"), 1);
    assert_eq!(estimated_tokens("abcde"), 2);
}

#[test]
fn each_budget_leaves_room_for_the_longest_reply() {
    for call in [
        Call::NarratorLine,
        Call::Chapter,
        Call::Judge,
        Call::Talk,
        Call::Quest,
        Call::Lore,
    ] {
        assert!(
            call.prompt_budget() + call.reply_tokens() < 2048,
            "{call:?}"
        );
    }
}

/// Run with `cargo test --test voice -- --ignored --nocapture print`.
#[test]
#[ignore = "prints the size of each prompt for review"]
fn print_the_size_of_each_prompt() {
    for (name, call, prompt) in every_prompt() {
        let tokens = estimated_tokens(&prompt);
        println!("{name}: {tokens} tokens, budget {}", call.prompt_budget());
    }
}

/// `TIMEWAYS_MODEL` is a shell command that reads a prompt on stdin and writes the answer
/// on stdout. The default is Claude Code with no tools.
fn ask_model(command: &str, prompt: &str) -> String {
    let mut child = Command::new("sh")
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(prompt.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The default is Claude Code with no tools, no MCP servers, and no settings, as the
/// bridge runs it.
const CLAUDE: &str = "claude -p --tools '' --strict-mcp-config --setting-sources ''";

/// Two drafts of the side quest chapter, and the pick of the judge: 3 calls.
fn review_best_of_two(command: &str, review: &mut String) {
    let drafts = [Draft::First, Draft::Second].map(|draft| {
        let answer = ask_model(command, &side_quest_chapter(draft));
        chronicle::checked_saga(&answer, 1, "").map(|saga| saga.text)
    });
    let _ = write!(review, "\n## Best of two: a finished side quest\n");
    for (number, draft) in drafts.iter().enumerate() {
        let shown = draft.as_deref().unwrap_or("(refused)");
        let _ = write!(review, "\nDraft {}: {shown}\n", number + 1);
    }
    let [Some(first), Some(second)] = &drafts else {
        return;
    };
    let facts = chronicle::facts(&[], &side_quest());
    let answer = ask_model(command, &chronicle::judge_prompt(4, &facts, first, second));
    let pick = match chronicle::checked_pick(&answer) {
        Pick::First => 1,
        Pick::Second => 2,
    };
    let _ = write!(review, "\nJudge: {answer}\n\nPicked: draft {pick}\n");
}

#[test]
#[ignore = "calls a real model"]
fn the_voice_moments_go_to_a_real_model_for_review() {
    let command = std::env::var("TIMEWAYS_MODEL").unwrap_or_else(|_| CLAUDE.to_string());
    let mut review = format!("# Voice review\n\nModel: `{command}`\n");

    for moment in voice_moments() {
        let answer = ask_model(&command, &moment.prompt);
        let shown = (moment.shown)(&answer).unwrap_or_else(|| "(refused)".to_string());
        let _ = write!(
            review,
            "\n## {}\n\nAnswer:\n\n{answer}\n\nShown: {shown}\n",
            moment.name
        );
    }
    review_best_of_two(&command, &mut review);

    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("voice-review.md");
    std::fs::write(&path, review).unwrap();
    println!("The review is in {}", path.display());
}

/// The check of one fixed format: `Err` holds why the answer failed.
type FormatCheck = fn(&str) -> Result<(), String>;

/// How often a real model gives an answer that passes the check of a fixed format. A small
/// local model fails these more often than it writes a bad line.
#[test]
#[ignore = "calls a real model"]
fn the_answers_of_a_real_model_in_a_fixed_format_pass_their_checks() {
    let command = std::env::var("TIMEWAYS_MODEL").unwrap_or_else(|_| CLAUDE.to_string());
    let tries: usize = std::env::var("TIMEWAYS_TRIES")
        .ok()
        .and_then(|tries| tries.parse().ok())
        .unwrap_or(5);
    let mut review = format!("# Fixed formats\n\nModel: `{command}`, {tries} tries each\n");
    let formats: [(&str, String, FormatCheck); 4] = [
        ("a quest offer", quest_offer(), |answer| {
            quest::checked_quest(answer, &quest_known())
                .map(|_| ())
                .map_err(|fault| fault.to_string())
        }),
        ("a lore answer", lore_question(), |answer| {
            let faults = check::check(answer, 3);
            if faults.is_empty() {
                return Ok(());
            }
            Err(faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "))
        }),
        ("a task draft", task_draft(), |answer| {
            draft::checked_draft(answer, &draft_known())
                .map(|_| ())
                .map_err(|fault| format!("{fault:?}"))
        }),
        ("a judge", judge(), |answer| {
            let picked =
                answer.contains("\"pick\"") && (answer.contains('1') || answer.contains('2'));
            if picked {
                Ok(())
            } else {
                Err("no pick".to_string())
            }
        }),
    ];
    for (name, prompt, checked) in formats {
        let mut passed = 0;
        let mut faults = Vec::new();
        for _ in 0..tries {
            let answer = ask_model(&command, &prompt);
            match checked(&answer) {
                Ok(()) => passed += 1,
                Err(fault) => faults.push(format!("{fault} -- {}", answer.replace('\n', " "))),
            }
        }
        let _ = write!(review, "\n## {name}: {passed} of {tries}\n");
        for fault in faults {
            let _ = write!(review, "\n- {fault}\n");
        }
    }

    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("format-review.md");
    std::fs::write(&path, review).unwrap();
    println!("The review is in {}", path.display());
}
