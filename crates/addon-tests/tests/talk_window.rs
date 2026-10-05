//! The talk window (GAMEPLAY.md 3.5): the talk with one NPC, the reply box, combat, and the
//! quest card of work that the NPC offers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Game, step_views};
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Journal, TalkQuest, TalkQuestState, pages};
use timeways_story::quest::{QuestView, Status, Step};
use timeways_story::story::Output;

const NOW: u64 = 1_790_000_000;
const FARLEY: &str = "Innkeeper Farley";

fn target(game: &Game, name: &str) {
    game.run(&format!("wow.units.target = {{ name = '{name}' }}"));
}

fn talk(game: &Game, words: &str) {
    game.run(&format!("wow.Slash('/talk', '{words}')"));
}

fn answer(game: &Game, npc: &str, text: Option<&str>) {
    let line = serde_json::to_string(&Output::TalkAnswer {
        id: MessageId(1),
        npc: npc.to_string(),
        text: text.map(str::to_string),
        notice: None,
    })
    .unwrap();
    game.reply(&line);
}

fn shown(game: &Game) -> bool {
    game.eval("ns.TalkWindow.IsShown()")
}

/// The lines on the parchment of the window, top to bottom.
fn page(game: &Game) -> Vec<String> {
    game.eval("wow.ShownTexts(TimewaysTalkFrameScroll:GetScrollChild())")
}

fn title(game: &Game) -> String {
    game.eval("TimewaysTalkFrame.title.text")
}

fn box_shown(game: &Game) -> bool {
    game.eval("TimewaysTalkFrameReply:IsShown()")
}

fn box_text(game: &Game) -> String {
    game.eval("TimewaysTalkFrameReply:GetText() or ''")
}

/// Types the words in the reply box and presses Enter.
fn reply(game: &Game, words: &str) {
    game.run(&format!(
        "TimewaysTalkFrameReply:SetText('{words}')
         TimewaysTalkFrameReply.scripts.OnEnterPressed(TimewaysTalkFrameReply)"
    ));
}

/// The button of the window with this label shows.
fn button_shown(game: &Game, label: &str) -> bool {
    game.eval(&format!(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysTalkFrame and widget.text == '{label}' then
                 return widget:IsShown()
             end
         end
         return false"
    ))
}

fn click(game: &Game, label: &str) {
    game.run(&format!(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysTalkFrame and widget.text == '{label}' then
                 widget:Click()
             end
         end"
    ));
}

fn talks_sent(game: &Game) -> Vec<(String, String)> {
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::TalkAsked { npc, text, .. } => Some((npc, text)),
            _ => None,
        })
        .collect()
}

fn combat(game: &Game, on: bool) {
    let event = if on {
        "PLAYER_REGEN_DISABLED"
    } else {
        "PLAYER_REGEN_ENABLED"
    };
    game.run(&format!("wow.combat = {on}; wow.Fire('{event}')"));
}

/// Farley asked "Any news?", and his answer.
fn farley_answered(game: &Game) {
    target(game, FARLEY);
    talk(game, "Any news?");
    answer(game, FARLEY, Some("Nothing but rain."));
}

#[test]
fn talk_opens_the_window_with_the_npc_and_thinking() {
    let game = Game::new();
    target(&game, FARLEY);

    talk(&game, "Any news?");

    assert!(shown(&game));
    assert_eq!(title(&game), FARLEY);
    assert_eq!(page(&game), ["You: Any news?", "Thinking..."]);
}

#[test]
fn talk_alone_says_hello_in_the_window() {
    let game = Game::new();
    target(&game, FARLEY);

    talk(&game, "");

    assert_eq!(page(&game), ["You: Hello.", "Thinking..."]);
}

#[test]
fn the_answer_shows_in_the_window_and_not_in_the_chat() {
    let game = Game::new();

    farley_answered(&game);

    assert_eq!(page(&game), ["You: Any news?", "Nothing but rain."]);
    assert!(game.printed().is_empty(), "{:?}", game.printed());
}

#[test]
fn the_reply_box_shows_only_after_an_answer() {
    let game = Game::new();
    target(&game, FARLEY);
    talk(&game, "Any news?");
    assert!(!box_shown(&game));

    answer(&game, FARLEY, Some("Nothing but rain."));

    assert!(box_shown(&game));
    let hint: String = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent == TimewaysTalkFrameReply then
                 return widget.text
             end
         end",
    );
    assert_eq!(hint, "Say something...");
}

#[test]
fn enter_sends_the_reply_to_the_same_npc_after_the_target_changed() {
    let game = Game::new();
    farley_answered(&game);
    target(&game, "Marshal Dughan");

    reply(&game, "  Rain again?  ");

    let sent = talks_sent(&game);
    assert_eq!(
        sent.last().unwrap(),
        &(FARLEY.to_string(), "Rain again?".to_string())
    );
    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Nothing but rain.",
            "You: Rain again?",
            "Thinking..."
        ]
    );
    assert!(!box_shown(&game));
}

#[test]
fn an_empty_reply_sends_nothing() {
    let game = Game::new();
    farley_answered(&game);

    reply(&game, "   ");

    assert_eq!(talks_sent(&game).len(), 1);
}

#[test]
fn the_reply_box_takes_one_chat_line() {
    let game = Game::new();

    farley_answered(&game);

    let bytes: u32 = game.eval("TimewaysTalkFrameReply.maxBytes or 0");
    assert_eq!(bytes, 255);
}

#[test]
fn escape_closes_the_window() {
    let game = Game::new();
    farley_answered(&game);

    let special: bool = game.eval(
        "for _, name in ipairs(UISpecialFrames) do
             if name == 'TimewaysTalkFrame' then return true end
         end
         return false",
    );
    game.run("TimewaysTalkFrameReply.scripts.OnEscapePressed(TimewaysTalkFrameReply)");

    assert!(special);
    assert!(!shown(&game));
}

#[test]
fn goodbye_closes_the_window_and_sends_nothing() {
    let game = Game::new();
    farley_answered(&game);
    let before = game.sent().len();

    click(&game, "Goodbye");

    assert!(!shown(&game));
    assert_eq!(game.sent().len(), before);
}

#[test]
fn an_answer_after_the_window_closed_goes_to_the_chat() {
    let game = Game::new();
    target(&game, FARLEY);
    talk(&game, "Any news?");
    click(&game, "Goodbye");

    answer(&game, FARLEY, Some("Nothing but rain."));

    assert!(!shown(&game));
    assert_eq!(
        game.printed(),
        ["|cffffd100Innkeeper Farley says:|r Nothing but rain."]
    );
}

#[test]
fn no_words_from_the_npc_keep_the_words_of_the_player_in_the_box() {
    let game = Game::new();
    target(&game, FARLEY);
    talk(&game, "Any news?");

    answer(&game, FARLEY, None);

    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Innkeeper Farley looks at you and says nothing."
        ]
    );
    assert!(box_shown(&game));
    assert_eq!(box_text(&game), "Any news?");
}

#[test]
fn an_error_reply_says_no_answer_came_and_keeps_the_words() {
    let game = Game::new();
    game.run(
        "ns.Link.Send = function(text)
             table.insert(sent, text)
             answered = #sent
             return #sent
         end
         C_Timer.NewTicker(1, function()
             ns.Link.Receive(#sent, 'error', 'Timeways story program not running.')
         end)",
    );
    target(&game, FARLEY);
    talk(&game, "Any news?");

    game.run("wow.RunTickers()");

    assert_eq!(
        page(&game),
        ["You: Any news?", "No answer came back. Try again."]
    );
    assert_eq!(box_text(&game), "Any news?");
}

#[test]
fn talk_with_words_to_the_same_npc_adds_a_turn() {
    let game = Game::new();
    farley_answered(&game);

    talk(&game, "And the roads?");

    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Nothing but rain.",
            "You: And the roads?",
            "Thinking..."
        ]
    );
}

#[test]
fn talk_to_another_npc_starts_a_new_talk() {
    let game = Game::new();
    farley_answered(&game);
    target(&game, "Marshal Dughan");

    talk(&game, "Hello.");

    assert_eq!(title(&game), "Marshal Dughan");
    assert_eq!(page(&game), ["You: Hello.", "Thinking..."]);
}

#[test]
fn combat_hides_the_window_and_the_end_of_combat_shows_it_again() {
    let game = Game::new();
    farley_answered(&game);

    combat(&game, true);
    assert!(!shown(&game));
    combat(&game, false);

    assert!(shown(&game));
    assert_eq!(page(&game), ["You: Any news?", "Nothing but rain."]);
}

#[test]
fn an_answer_in_combat_waits_in_the_window() {
    let game = Game::new();
    target(&game, FARLEY);
    talk(&game, "Any news?");
    combat(&game, true);

    answer(&game, FARLEY, Some("Nothing but rain."));
    combat(&game, false);

    assert!(game.printed().is_empty(), "{:?}", game.printed());
    assert_eq!(page(&game), ["You: Any news?", "Nothing but rain."]);
}

#[test]
fn talk_in_combat_opens_the_window_after_combat() {
    let game = Game::new();
    target(&game, FARLEY);
    game.run("wow.combat = true");

    talk(&game, "Any news?");
    assert!(!shown(&game));
    combat(&game, false);

    assert!(shown(&game));
}

#[test]
fn combat_never_opens_a_window_that_was_closed() {
    let game = Game::new();
    farley_answered(&game);
    click(&game, "Goodbye");

    combat(&game, true);
    combat(&game, false);

    assert!(!shown(&game));
}

// The quest card.

fn lantern() -> QuestView {
    QuestView {
        number: 4,
        offered_at: Tick(NOW),
        giver: FARLEY.to_string(),
        title: "The Lost Lantern".to_string(),
        text: "I lost my lantern by the pond. Find it.".to_string(),
        steps: step_views(
            vec![
                Step::Visit {
                    place: "Mill Pond".to_string(),
                },
                Step::Meet {
                    npc: "Farmer Bram".to_string(),
                },
            ],
            Status::Offered,
            0,
        ),
        status: Status::Offered,
        done_at: None,
        any_order: None,
        has_slap: false,
        hidden_steps: 0,
    }
}

/// A journal of one page with the quest of a talk at `at` and these quests.
fn journal_reply(at: u64, state: TalkQuestState, quests: Vec<QuestView>) -> String {
    let talk_quest = TalkQuest {
        npc: FARLEY.to_string(),
        at: Tick(at),
        state,
    };
    let journal = Journal {
        quests,
        talk_quest: Some(Box::new(talk_quest)),
        ..Journal::default()
    };
    let page = pages(journal).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page: Box::new(page),
        notice: None,
    })
    .unwrap()
}

fn offered() -> String {
    journal_reply(NOW, TalkQuestState::Offered { number: 4 }, vec![lantern()])
}

fn writing() -> String {
    journal_reply(NOW, TalkQuestState::Writing, Vec::new())
}

fn journal_requests(game: &Game) -> usize {
    let asked = |input: &Input| matches!(input, Input::JournalAsked { .. });
    game.sent_inputs()
        .iter()
        .filter(|input| asked(input))
        .count()
}

/// Runs the one-shot timers that are due after `seconds`.
fn wait(game: &Game, seconds: u64) {
    game.run(&format!(
        "wow.now = wow.now + {seconds}
         local due = {{}}
         for n = #wow.after, 1, -1 do
             if wow.after[n].at <= wow.now then
                 table.insert(due, table.remove(wow.after, n))
             end
         end
         for _, timer in ipairs(due) do timer.callback() end"
    ));
}

#[test]
fn the_window_says_that_the_npc_thinks_of_a_quest_while_it_is_written() {
    let game = Game::new();
    farley_answered(&game);

    game.reply(&writing());

    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Nothing but rain.",
            "Thinking of a quest..."
        ]
    );
}

#[test]
fn the_offer_shows_as_a_quest_card_with_accept_and_decline() {
    let game = Game::new();
    farley_answered(&game);

    game.reply(&offered());

    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Nothing but rain.",
            "The Lost Lantern",
            "I lost my lantern by the pond. Find it.",
            "Quest Objectives",
            "Visit Mill Pond.",
            "Speak with Farmer Bram.",
        ]
    );
    assert!(button_shown(&game, "Accept"));
    assert!(button_shown(&game, "Decline"));
}

#[test]
fn accept_on_the_card_accepts_that_quest_as_the_quests_page_does() {
    let game = Game::new();
    farley_answered(&game);
    game.reply(&offered());

    click(&game, "Accept");

    let accepted = game.sent_inputs().into_iter().any(|input| {
        matches!(
            input,
            Input::QuestAccepted {
                number: Some(4),
                ..
            }
        )
    });
    assert!(accepted);
    assert_eq!(page(&game).last().unwrap(), "Quest accepted.");
    assert!(!button_shown(&game, "Accept"));
    assert!(!button_shown(&game, "Decline"));
}

#[test]
fn decline_on_the_card_declines_that_quest_as_the_quests_page_does() {
    let game = Game::new();
    farley_answered(&game);
    game.reply(&offered());

    click(&game, "Decline");

    let declined = game.sent_inputs().into_iter().any(|input| {
        matches!(
            input,
            Input::QuestDeclined {
                number: Some(4),
                ..
            }
        )
    });
    assert!(declined);
    assert_eq!(page(&game).last().unwrap(), "Quest declined.");
}

#[test]
fn a_refused_quest_shows_its_line_and_the_words_of_the_npc_stay() {
    let game = Game::new();
    farley_answered(&game);
    let line = "You already have 3 quests. Finish one first.".to_string();

    game.reply(&journal_reply(
        NOW,
        TalkQuestState::Refused { line },
        Vec::new(),
    ));

    assert_eq!(
        page(&game),
        [
            "You: Any news?",
            "Nothing but rain.",
            "You already have 3 quests. Finish one first."
        ]
    );
    assert!(!button_shown(&game, "Accept"));
}

#[test]
fn the_quest_of_an_older_talk_shows_no_card() {
    let game = Game::new();
    farley_answered(&game);

    let older = journal_reply(
        NOW - 60,
        TalkQuestState::Offered { number: 4 },
        vec![lantern()],
    );
    game.reply(&older);

    assert_eq!(page(&game), ["You: Any news?", "Nothing but rain."]);
}

#[test]
fn an_offer_that_is_no_longer_offered_shows_no_card() {
    let game = Game::new();
    farley_answered(&game);
    let mut taken = lantern();
    taken.status = Status::Accepted;

    game.reply(&journal_reply(
        NOW,
        TalkQuestState::Offered { number: 4 },
        vec![taken],
    ));

    assert!(!button_shown(&game, "Accept"));
}

#[test]
fn the_window_asks_for_the_journal_again_while_the_quest_is_written() {
    let game = Game::new();
    farley_answered(&game);
    game.reply(&writing());
    let before = journal_requests(&game);

    wait(&game, 10);

    assert_eq!(journal_requests(&game), before + 1);
}

#[test]
fn the_window_stops_asking_after_three_minutes() {
    let game = Game::new();
    farley_answered(&game);

    for _ in 0..30 {
        game.reply(&writing());
        wait(&game, 10);
    }

    let asked_after_the_answer = journal_requests(&game) - 1;
    assert_eq!(asked_after_the_answer, 18);
}

#[test]
fn an_offer_that_comes_after_goodbye_gets_one_chat_line() {
    let game = Game::new();
    farley_answered(&game);
    game.reply(&writing());
    click(&game, "Goodbye");

    game.reply(&offered());
    game.reply(&offered());

    assert_eq!(
        game.printed(),
        [
            "|cffc8a064Timeways|r: Innkeeper Farley has a quest for you: The Lost Lantern. Type /journal to read it."
        ]
    );
}

#[test]
fn a_broken_quest_of_a_talk_shows_nothing_and_raises_no_error() {
    let game = Game::new();
    farley_answered(&game);

    let broken = writing().replace(r#""state":"writing""#, r#""state":"offered""#);
    game.reply(&broken);

    assert_eq!(page(&game), ["You: Any news?", "Nothing but rain."]);
}
