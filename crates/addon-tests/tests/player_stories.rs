//! Stories that players tell about each other, between two games with Timeways (GAMEPLAY.md
//! 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use common::Game;
use players::{Player, ada_and_corvin, deliver, exchange};
use proptest::prelude::*;
use timeways_story::aliases::unmarked;
use timeways_story::input::Input;
use timeways_story::stories::is_good_story;

const PREFIX: &str = "|cffc8a064Timeways|r: ";

/// Ada and Corvin, in one party and one guild.
fn party() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    (ada, corvin)
}

/// Ada targets Corvin and tells a quick story, and the messages go both ways.
fn tell(ada: &Player, corvin: &Player, text: &str) {
    ada.run("wow.units.target = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");
    ada.run(&format!("wow.Slash('/story', '{text}')"));
    exchange(ada, corvin);
}

/// Ada asks Corvin, and sends the story on "open", as the scroll does.
fn send(ada: &Player, corvin: &Player, title: &str, body: &str) {
    ada.run(&format!(
        "ns.PlayerStories.Ask('Corvin-Stormrage', function(room)
             lastRoom = room
             if room == 'open' then ns.PlayerStories.Send('Corvin-Stormrage', '{title}', '{body}') end
         end)"
    ));
    exchange(ada, corvin);
}

fn last_room(ada: &Player) -> String {
    ada.eval("return lastRoom")
}

/// The room that Corvin's box answers to Ada's question.
fn asked_room(ada: &Player, corvin: &Player) -> Option<String> {
    ada.run(
        "lastRoom = nil ns.PlayerStories.Ask('Corvin-Stormrage', function(room) lastRoom = room end)",
    );
    exchange(ada, corvin);
    ada.eval("return lastRoom")
}

fn waiting(corvin: &Player) -> usize {
    corvin.eval("return #ns.PlayerStories.Waiting()")
}

/// The inputs that the game sent to the desktop, after the outbox flushed.
fn sent_inputs(player: &Player) -> Vec<Input> {
    player.run("wow.RunTickers()");
    player.game.sent_inputs()
}

fn accepted(inputs: &[Input]) -> Option<(Option<String>, Vec<String>)> {
    inputs.iter().find_map(|input| match input {
        Input::StoryAccepted {
            title, paragraphs, ..
        } => Some((title.clone(), paragraphs.clone())),
        _ => None,
    })
}

/// Runs the one-shot timers of the game, such as the wait for an answer.
fn run_timers(player: &Player) {
    player.run(
        "local timers = wow.after
         wow.after = {}
         for _, timer in ipairs(timers) do timer.callback() end",
    );
}

/// A story message of Ada, as one part on the wire.
fn hear_story(corvin: &Player, fields: &str) {
    corvin.hear_logged(
        "Timeways",
        &format!("1:1:1:1;story;{fields}"),
        "WHISPER",
        "Ada-Stormrage",
    );
}

fn told_status(ada: &Player) -> String {
    ada.eval("local _, story = next(TimewaysStories.told) return story.status")
}

#[test]
fn a_story_from_a_party_member_waits_for_an_answer() {
    let (ada, corvin) = party();

    tell(&ada, &corvin, "Corvin held the bridge alone.");

    assert_eq!(waiting(&corvin), 1);
    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Ada told a story about you. Type /stories to read it."
    )));
    assert!(ada.printed().contains(&format!(
        "{PREFIX}Sent to Corvin. They'll decide if it's part of their story."
    )));
}

#[test]
fn a_story_with_a_title_names_it_in_the_chat() {
    let (ada, corvin) = party();

    send(&ada, &corvin, "The Bridge", "We held it.");

    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Ada told a story about you: The Bridge. Type /stories to read it."
    )));
}

#[test]
fn a_story_needs_a_target_in_your_group() {
    let (ada, corvin) = ada_and_corvin();

    tell(&ada, &corvin, "Corvin held the bridge alone.");

    assert_eq!(waiting(&corvin), 0);
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Target a player in your group first."))
    );
}

#[test]
fn a_story_from_a_player_outside_the_group_gets_no_answer() {
    let (ada, corvin) = ada_and_corvin();

    hear_story(&corvin, "a1;;A tale.");

    assert_eq!(waiting(&corvin), 0);
    assert_eq!(deliver(&corvin, &ada), 0);
}

#[test]
fn story_ask_from_outside_the_group_gets_no_answer() {
    let (ada, corvin) = ada_and_corvin();

    corvin.hear("Timeways", "1:1:1:1;story_ask", "WHISPER", "Ada-Stormrage");

    assert_eq!(deliver(&corvin, &ada), 0);
}

#[test]
fn story_ask_gets_open_when_nothing_of_yours_waits() {
    let (ada, corvin) = party();

    assert_eq!(asked_room(&ada, &corvin).as_deref(), Some("open"));
}

#[test]
fn a_second_story_from_one_author_gets_the_room_waiting() {
    let (ada, corvin) = party();
    send(&ada, &corvin, "", "The first.");

    assert_eq!(asked_room(&ada, &corvin).as_deref(), Some("waiting"));
    hear_story(&corvin, "a9;;The second, from a changed addon.");

    assert_eq!(waiting(&corvin), 1);
    let answer = corvin.take_sent().pop().unwrap().text;
    assert!(answer.ends_with("story_room;waiting"), "{answer}");
}

#[test]
fn a_twenty_first_story_gets_the_room_full() {
    let (ada, corvin) = party();
    corvin.run(
        "for n = 1, 20 do
             table.insert(ns.PlayerStories.Waiting(),
                 { id = 'w' .. n, author = 'P' .. n .. '-Stormrage', title = '', text = 'Hi.', at = 1 })
         end",
    );

    assert_eq!(asked_room(&ada, &corvin).as_deref(), Some("full"));
    hear_story(&corvin, "a1;;One too many.");
    assert_eq!(waiting(&corvin), 20);
}

#[test]
fn a_story_from_a_blocked_author_gets_the_room_blocked() {
    let (ada, corvin) = party();
    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");

    assert_eq!(asked_room(&ada, &corvin).as_deref(), Some("blocked"));
    hear_story(&corvin, "a1;;A tale.");
    assert_eq!(waiting(&corvin), 0);
}

#[test]
fn a_blocked_author_learns_it_from_the_room() {
    let (ada, corvin) = party();
    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");

    tell(&ada, &corvin, "We held it.");

    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin doesn't take stories from you."))
    );
    assert!(ada.eval::<bool>("return ns.TaskStore.Data().refusedBy['Corvin-Stormrage'] ~= nil"));
}

#[test]
fn the_author_hears_why_the_box_did_not_take_the_story() {
    let (ada, corvin) = party();
    ada.run("ns.PlayerStories.Send('Corvin-Stormrage', '', 'A story with no question.')");
    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");

    exchange(&ada, &corvin);

    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin doesn't take stories from you."))
    );
    assert_eq!(told_status(&ada), "blocked");
}

#[test]
fn the_answer_open_marks_a_sent_story_as_lost() {
    let (ada, corvin) = party();
    ada.run("ns.PlayerStories.Send('Corvin-Stormrage', '', 'A story that gets lost.')");
    ada.take_sent();

    assert_eq!(asked_room(&ada, &corvin).as_deref(), Some("open"));

    assert_eq!(told_status(&ada), "lost");
}

#[test]
fn no_answer_in_time_means_the_player_has_no_timeways() {
    let (ada, corvin) = party();
    ada.run("wow.units.target = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");
    ada.run("wow.Slash('/story', 'We held the bridge.')");
    ada.take_sent();

    run_timers(&ada);

    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin needs Timeways to get stories."))
    );
    assert_eq!(waiting(&corvin), 0);
}

#[test]
fn the_author_can_send_again_after_accept_or_decline() {
    let (ada, corvin) = party();
    send(&ada, &corvin, "", "The first.");
    corvin.run("ns.PlayerStories.Accept()");
    exchange(&ada, &corvin);

    send(&ada, &corvin, "", "The second.");
    corvin.run("ns.PlayerStories.Decline()");
    exchange(&ada, &corvin);
    send(&ada, &corvin, "", "The third.");

    assert_eq!(last_room(&ada), "open");
    assert_eq!(waiting(&corvin), 1);
}

#[test]
fn block_player_removes_the_waiting_story_of_that_author() {
    let (ada, corvin) = party();
    send(&ada, &corvin, "", "We held it.");

    corvin.run("ns.PlayerStories.Block('Ada-Stormrage')");

    assert_eq!(waiting(&corvin), 0);
    assert_eq!(deliver(&corvin, &ada), 0);
    assert!(corvin.eval::<bool>("return ns.TaskStore.Data().blocked['Ada-Stormrage'] ~= nil"));
}

#[test]
fn accept_sends_the_title_and_the_paragraphs_with_names_marked_and_the_author_hears() {
    let (ada, corvin) = party();
    send(
        &ada,
        &corvin,
        "Ada at the Bridge",
        "Corvin and Ada held it.\\nAda ran.",
    );

    corvin.run("wow.Slash('/story', 'accept')");
    exchange(&ada, &corvin);

    let inputs = sent_inputs(&corvin);
    let (title, paragraphs) = accepted(&inputs).expect("an accepted story");
    assert_eq!(title.as_deref(), Some("{Ada} at the Bridge"));
    assert_eq!(paragraphs, ["$N and {Ada} held it.", "{Ada} ran."]);
    let described = inputs
        .iter()
        .position(|input| matches!(input, Input::PlayerDescribed { name, .. } if name == "Ada"));
    let story = inputs
        .iter()
        .position(|input| matches!(input, Input::StoryAccepted { .. }));
    assert!(described < story && described.is_some(), "{inputs:?}");
    let author: String = corvin.eval("return ns.PlayerStories.AuthorOf(1)");
    assert_eq!(author, "Ada-Stormrage");
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
}

#[test]
fn a_declined_story_never_reaches_the_desktop_and_the_author_hears() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Corvin fell in the river.");

    corvin.run("wow.Slash('/story', 'decline')");
    exchange(&ada, &corvin);

    assert_eq!(accepted(&sent_inputs(&corvin)), None);
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin declined your story."))
    );
}

#[test]
fn a_story_whose_marks_make_it_too_long_for_the_strip_stays_waiting() {
    let (ada, corvin) = party();
    send(&ada, &corvin, "", "Ada and Ada held it.");
    corvin.run("linkLimit = 100");

    let problem: String = corvin.eval("return ns.PlayerStories.Accept()");

    assert_eq!(
        problem,
        "This story is too long to keep. Decline it, or ask Ada for a shorter one."
    );
    assert_eq!(waiting(&corvin), 1);
    assert_eq!(accepted(&sent_inputs(&corvin)), None);
}

#[test]
fn a_story_full_of_names_at_the_limit_still_fits_the_desktop() {
    let (ada, corvin) = party();
    let text = "ada ".repeat(250);
    send(&ada, &corvin, "", text.trim_end());

    corvin.run("ns.PlayerStories.Accept()");

    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    assert!(paragraphs[0].len() > text.len(), "{}", paragraphs[0]);
    let plain = unmarked(&paragraphs[0]).text;
    assert!(is_good_story(None, &[plain]));
}

#[test]
fn a_story_accepted_after_its_author_left_still_marks_the_name() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.leave_party();
    corvin.run("wow.guild = {} wow.Fire('GUILD_ROSTER_UPDATE', false)");

    corvin.run("wow.Slash('/story', 'accept')");

    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    assert_eq!(paragraphs, ["{Ada} and I held the bridge."]);
}

#[test]
fn the_answer_of_a_story_never_touches_a_quest_with_the_same_id() {
    let (ada, corvin) = party();
    let id: String = ada.eval(
        "return ns.PlayerTasks.Give({ title = 'A Walk', text = '', reward = '',
             steps = { { kind = 'place', target = 'Brill', count = 1 } } }, 'Corvin-Stormrage')",
    );
    exchange(&ada, &corvin);

    ada.hear(
        "Timeways",
        &format!("1;story_accept;{id}"),
        "WHISPER",
        "Corvin-Stormrage",
    );

    let status: String = ada.eval(&format!("return ns.TaskStore.Data().given['{id}'].status"));
    assert_eq!(status, "offered");
}

#[test]
fn a_told_story_keeps_no_text() {
    let (ada, corvin) = party();

    send(&ada, &corvin, "The Bridge", "A secret text.");

    let text: Option<String> =
        ada.eval("local _, story = next(TimewaysStories.told) return story.text");
    let title: String = ada.eval("local _, story = next(TimewaysStories.told) return story.title");
    assert_eq!((text, title.as_str()), (None, "The Bridge"));
}

#[test]
fn a_story_with_the_pipe_sign_says_so_in_a_line_that_shows_the_sign() {
    let (ada, corvin) = party();

    tell(&ada, &corvin, "a | b");

    assert_eq!(waiting(&corvin), 0);
    assert_eq!(
        ada.printed().pop().unwrap(),
        format!("{PREFIX}Stories can't hold the || sign. Take it out and try again.")
    );
}

#[test]
fn a_story_too_long_to_send_asks_for_a_shorter_one() {
    let (ada, corvin) = party();

    tell(&ada, &corvin, &"é".repeat(601));

    assert_eq!(waiting(&corvin), 0);
    assert_eq!(
        ada.printed().pop().unwrap(),
        format!("{PREFIX}Too long to send. Try a shorter version.")
    );
}

#[test]
fn the_story_command_alone_says_that_blizzard_can_read_a_story() {
    let (ada, _) = party();

    ada.run("wow.Slash('/story', '')");

    let last = ada.printed().pop().unwrap();
    assert!(last.ends_with("Like chat, Blizzard can read what you send."));
}

// The wire -------------------------------------------------------------------------------------

/// The fields of a story after a trip through the wire: the title, then each paragraph.
fn round_trip(game: &Game, title: &str, body: &str) -> Option<Vec<String>> {
    let trip: mlua::Function = game.eval(
        "return function(title, body)
             local message = ns.TaskWire.Decode(ns.TaskWire.Encode({ type = 'story', id = 'a1',
                 story_title = title, body = body }))
             if not message then return nil end
             local fields = { message.story_title }
             for _, paragraph in ipairs(ns.StoryText.Paragraphs(message.body)) do
                 fields[#fields + 1] = paragraph
             end
             return fields
         end",
    );
    trip.call((title, body)).unwrap()
}

fn decodes(game: &Game, text: &str) -> bool {
    let decode: mlua::Function =
        game.eval("return function(text) return ns.TaskWire.Decode(text) ~= nil end");
    decode.call(text).unwrap()
}

/// The parts of a message on the wire, or 99 when it does not fit 16.
fn parts_of(game: &Game, title: &str, body: &str) -> usize {
    let split: mlua::Function = game.eval(
        "return function(title, body)
             local text = ns.TaskWire.Encode({ type = 'story', id = 'abcdefghijklmnop',
                 story_title = title, body = body })
             local parts = ns.TaskChunks.Split(text, 1)
             return parts and #parts or 99
         end",
    );
    split.call((title, body)).unwrap()
}

fn is_body(game: &Game, body: &str) -> bool {
    let check: mlua::Function = game.eval("return ns.StoryText.IsBody");
    check.call(body).unwrap()
}

#[test]
fn a_story_with_a_title_and_paragraphs_crosses_the_wire_unchanged() {
    let game = Game::new();

    let fields = round_trip(
        &game,
        "100% of it; \\ too",
        "We held; 100%.\nThen \\ ran.\nThe end.",
    );

    assert_eq!(
        fields.unwrap(),
        [
            "100% of it; \\ too",
            "We held; 100%.",
            "Then \\ ran.",
            "The end."
        ]
    );
}

#[test]
fn a_paragraph_break_travels_as_an_escape() {
    let game = Game::new();

    let encoded: String = game.eval(
        "return ns.TaskWire.Encode({ type = 'story', id = 'a1', story_title = '', body = 'One.\\nTwo.' })",
    );

    assert_eq!(encoded, "1;story;a1;;One.%0ATwo.");
}

#[test]
fn a_story_with_two_breaks_in_a_row_is_dropped() {
    let game = Game::new();

    assert!(!decodes(&game, "1;story;a1;;One.%0A%0ATwo."));
    assert!(!decodes(&game, "1;story;a1;;%0AOne."));
    assert!(!decodes(&game, "1;story;a1;;One.%0A"));
    assert!(!decodes(&game, "1;story;a1;;One.%0D%0ATwo."));
    assert!(decodes(&game, "1;story;a1;;One.%0ATwo."));
}

#[test]
fn a_line_break_in_a_title_is_dropped() {
    let game = Game::new();

    assert!(!decodes(&game, "1;story;a1;A%0ATitle;One."));
    assert!(decodes(&game, "1;story;a1;A Title;One."));
}

#[test]
fn a_story_of_the_old_shape_is_dropped() {
    let game = Game::new();

    assert!(!decodes(&game, "1;story;a1;Just a text."));
}

#[test]
fn a_room_that_is_no_room_is_dropped() {
    let game = Game::new();

    assert!(decodes(&game, "1;story_room;open"));
    assert!(!decodes(&game, "1;story_room;maybe"));
}

/// Every letter that takes an escape, and the letters of 4 bytes, at the limits of a story.
#[test]
fn the_longest_story_with_every_letter_escaped_fits_sixteen_parts() {
    let game = Game::new();
    let title = format!("{}{}", "%".repeat(56), "🐉".repeat(4));
    let mut body = vec![";".repeat(40); 19].join("\n");
    body.push('\n');
    body.push_str(&"\\".repeat(1000 - body.chars().count() - 66));
    body.push_str(&"🐉".repeat(66));
    assert_eq!(body.chars().count(), 1000);
    assert!(is_body(&game, &body));

    assert!(parts_of(&game, &title, &body) <= 16);
}

/// The real strip of the transport, with a long name of a character.
#[test]
fn the_longest_story_without_names_fits_one_strip() {
    let game = Game::with_transport();
    game.run("ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Abcdefghijkl'))");
    let title = format!("{}{}", "\"".repeat(56), "🐉".repeat(4));
    let mut paragraphs = vec!["\"".repeat(45); 19];
    paragraphs.push(format!(
        "{}{}",
        "\"".repeat(1000 - 19 - 19 * 45 - 66),
        "🐉".repeat(66)
    ));
    let body = paragraphs.join("\n");
    assert!(is_body(&game, &body));

    let fits: mlua::Function = game.eval(
        "return function(title, body)
             local input = ns.Inputs.StoryAccepted(4294967295, 4294967295, title,
                 ns.StoryText.Paragraphs(body))
             return ns.Outbox.Fits(input)
         end",
    );

    assert!(fits.call::<bool>((title, body)).unwrap());
}

// The saved variables ----------------------------------------------------------------------------

#[test]
fn a_saved_waiting_story_with_a_broken_body_is_dropped() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { waiting = {
             { id = 'a1', author = 'Bram-Stormrage', title = '', text = 'Fine.\\nTwo.', at = 1 },
             { id = 'a2', author = 'Cora-Stormrage', title = '', text = 'Two\\n\\nbreaks.', at = 1 },
             { id = 'a3', author = 'Dax-Stormrage', title = 'A\\ntitle', text = 'Fine.', at = 1 },
             { id = 'a4', author = 'Eli-Stormrage', text = 'No title.', at = 1 } } }",
    );

    let waiting: usize = game.eval("return #ns.PlayerStories.Waiting()");

    assert_eq!(waiting, 1);
}

#[test]
fn a_saved_file_with_two_waiting_stories_of_one_author_keeps_the_oldest() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { waiting = {
             { id = 'a1', author = 'Bram-Stormrage', title = '', text = 'First.', at = 1 },
             { id = 'a2', author = 'Bram-Stormrage', title = '', text = 'Second.', at = 2 } } }",
    );

    let kept: String = game.eval("return ns.PlayerStories.Waiting()[1].id");

    assert_eq!(kept, "a1");
    assert_eq!(game.eval::<usize>("return #ns.PlayerStories.Waiting()"), 1);
}

#[test]
fn authors_keep_the_newest_five_hundred_numbers() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { authors = {} }
         for n = 1, 600 do TimewaysStories.authors[n] = 'Bram-Stormrage' end",
    );

    let count: usize = game.eval(
        "local n = 0
         for _ in pairs(ns.StorySaved.Data().authors) do n = n + 1 end
         return n",
    );
    let oldest: Option<String> = game.eval("return ns.PlayerStories.AuthorOf(100)");

    assert_eq!((count, oldest), (500, None));
    assert!(game.eval::<bool>("return ns.PlayerStories.AuthorOf(101) ~= nil"));
}

#[test]
fn a_draft_survives_a_reload() {
    let game = Game::new();
    game.run("ns.StoryDrafts.Save('Corvin-Stormrage', 'A Title', 'One.\\n\\nTwo.')");
    let saved: String = game.eval("return ns.Json.Encode(TimewaysStories.drafts[1])");

    let reloaded = Game::new();
    reloaded.run(&format!(
        "TimewaysStories = {{ drafts = {{ ns.Json.Decode([[{saved}]]) }} }}"
    ));

    let text: String = reloaded.eval("return ns.StoryDrafts.For('Corvin-Stormrage').text");
    assert_eq!(text, "One.\n\nTwo.");
}

#[test]
fn the_eleventh_draft_is_refused_and_no_draft_is_dropped() {
    let game = Game::new();
    game.run(
        "for n = 1, 10 do ns.StoryDrafts.Save('P' .. n .. '-Stormrage', '', 'Text ' .. n) end",
    );

    let saved: bool = game.eval("return ns.StoryDrafts.Save('Eleven-Stormrage', '', 'More.')");
    let again: bool = game.eval("return ns.StoryDrafts.Save('P3-Stormrage', '', 'Changed.')");

    assert!(!saved);
    assert!(again);
    assert_eq!(game.eval::<usize>("return #ns.StoryDrafts.All()"), 10);
}

#[test]
fn a_sent_story_deletes_its_draft() {
    let (ada, corvin) = party();
    ada.run("ns.StoryDrafts.Save('Corvin-Stormrage', '', 'Draft.')");

    send(&ada, &corvin, "", "Draft.");

    assert!(ada.eval::<bool>("return ns.StoryDrafts.For('Corvin-Stormrage') == nil"));
}

// Properties ------------------------------------------------------------------------------------

/// What can happen to Corvin's box, from any number of authors.
#[derive(Clone, Debug)]
enum BoxStep {
    /// A story from an author, also from a changed addon that never asked.
    Story(usize),
    Ask(usize),
    Accept,
    Decline,
    Block(usize),
}

fn author_count() -> impl Strategy<Value = usize> {
    prop_oneof![Just(1usize), Just(20), Just(21), Just(30), 1usize..31]
}

fn box_steps() -> impl Strategy<Value = (usize, Vec<BoxStep>)> {
    author_count().prop_flat_map(|authors| {
        let step = prop_oneof![
            6 => (0..authors).prop_map(BoxStep::Story),
            2 => (0..authors).prop_map(BoxStep::Ask),
            1 => Just(BoxStep::Accept),
            1 => Just(BoxStep::Decline),
            1 => (0..authors).prop_map(BoxStep::Block),
        ];
        (Just(authors), prop::collection::vec(step, 0..80))
    })
}

/// Corvin in a party with every author, as the box checks the group.
fn box_with_authors(authors: usize) -> Game {
    let game = Game::new();
    game.run(&format!(
        "for n = 1, {authors} do
             wow.units['party' .. n] = {{ name = 'A' .. n, player = true, guid = 'Player-1-A' .. n }}
         end"
    ));
    game
}

fn play_box_step(game: &Game, step: &BoxStep, serial: usize) {
    let author = |n: usize| format!("A{}-Stormrage", n + 1);
    match step {
        BoxStep::Story(n) => game.run(&format!(
            "ns.PlayerStories.Receive('{}', {{ type = 'story', id = 's{serial}', story_title = '', body = 'Hi.' }})",
            author(*n)
        )),
        BoxStep::Ask(n) => game.run(&format!(
            "ns.PlayerStories.Receive('{}', {{ type = 'story_ask' }})",
            author(*n)
        )),
        BoxStep::Accept => game.run("ns.PlayerStories.Accept(1)"),
        BoxStep::Decline => game.run("ns.PlayerStories.Decline(1)"),
        BoxStep::Block(n) => game.run(&format!("ns.PlayerStories.Block('{}')", author(*n))),
    }
}

/// A two-player step: who acts, and whether the messages arrive.
#[derive(Clone, Debug)]
enum PairStep {
    Ask,
    /// A send that skips the question, as a changed addon can.
    SendBlind,
    Accept,
    Decline,
    /// The messages of both sides go, or get lost.
    Carry(bool),
    ReloadAda,
    ReloadCorvin,
}

fn pair_step() -> impl Strategy<Value = PairStep> {
    prop_oneof![
        3 => Just(PairStep::Ask),
        2 => Just(PairStep::SendBlind),
        2 => Just(PairStep::Accept),
        2 => Just(PairStep::Decline),
        4 => any::<bool>().prop_map(PairStep::Carry),
        1 => Just(PairStep::ReloadAda),
        1 => Just(PairStep::ReloadCorvin),
    ]
}

const SEND_ON_OPEN: &str = "asked = function(room)
     if room == 'open' then ns.PlayerStories.Send('Corvin-Stormrage', '', 'Hi.') end
 end";

/// A new game with the saved stories of the old one, as a `/reload` gives.
fn reloaded(player: &Player, name: &'static str, other: &Player) -> Player {
    let saved: String = player.eval(
        "return ns.Json.Encode({ waiting = { ns.StorySaved.Data().waiting },
             told = { ns.StorySaved.Data().told } })",
    );
    let fresh = Player::new(name);
    fresh.in_party_with(other);
    fresh.run(&format!(
        "local saved = ns.Json.Decode([==[{saved}]==])
         TimewaysStories = {{ waiting = saved.waiting[1] or {{}}, told = saved.told[1] or {{}} }}"
    ));
    fresh.run(SEND_ON_OPEN);
    fresh
}

fn sent_by_ada(ada: &Player) -> usize {
    ada.eval(
        "local n = 0
         for _, story in pairs(ns.StorySaved.Data().told) do
             if story.status == 'sent' then n = n + 1 end
         end
         return n",
    )
}

fn waiting_from_ada(corvin: &Player) -> usize {
    corvin.eval(
        "local n = 0
         for _, story in ipairs(ns.PlayerStories.Waiting()) do
             if story.author == 'Ada-Stormrage' then n = n + 1 end
         end
         return n",
    )
}

/// A clean body at its edges: escaped signs, 19 breaks, and the letter and byte limits.
fn clean_body() -> impl Strategy<Value = String> {
    let unit = prop::sample::select(vec!["%", ";", "\\", "a", "é", "🐉", "word"]);
    let paragraph = (unit, 1usize..60).prop_map(|(unit, count)| unit.repeat(count));
    prop_oneof![
        prop::collection::vec(paragraph, 1..=20).prop_map(|paragraphs| paragraphs.join("\n")),
        Just(vec!["x"; 20].join("\n")),
        Just("a".repeat(1000)),
        Just("a".repeat(1001)),
        Just("é".repeat(600)),
        Just("%".repeat(1000)),
        Just(";".repeat(1000)),
        Just("\\".repeat(1000)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// GAMEPLAY.md 4.8: the box holds at most one story of each author and 20 in all, for
    /// any order of stories, questions, answers, and blocks.
    #[test]
    fn the_waiting_box_never_holds_two_stories_of_one_author_or_more_than_twenty(
        (authors, steps) in box_steps(),
    ) {
        let game = box_with_authors(authors);
        for (serial, step) in steps.iter().enumerate() {
            play_box_step(&game, step, serial);
            let count: usize = game.eval("return #ns.PlayerStories.Waiting()");
            let distinct: usize = game.eval(
                "local seen, distinct = {}, 0
                 for _, story in ipairs(ns.PlayerStories.Waiting()) do
                     if not seen[story.author] then seen[story.author] = true distinct = distinct + 1 end
                 end
                 return distinct",
            );
            prop_assert!(count <= 20, "{}", count);
            prop_assert_eq!(count, distinct);
        }
    }

    /// One story at a time between two players: the receiver never holds two stories of the
    /// sender, and the sender never waits on two stories to the receiver.
    #[test]
    fn a_sender_never_has_two_stories_waiting_with_one_receiver(
        steps in prop::collection::vec(pair_step(), 0..30),
    ) {
        let (mut ada, mut corvin) = party();
        ada.run(SEND_ON_OPEN);
        for step in &steps {
            match step {
                PairStep::Ask => ada.run("ns.PlayerStories.Ask('Corvin-Stormrage', asked)"),
                PairStep::SendBlind => ada.run("ns.PlayerStories.Send('Corvin-Stormrage', '', 'Blind.')"),
                PairStep::Accept => corvin.run("ns.PlayerStories.Accept(1)"),
                PairStep::Decline => corvin.run("ns.PlayerStories.Decline(1)"),
                PairStep::Carry(true) => exchange(&ada, &corvin),
                PairStep::Carry(false) => {
                    ada.take_sent();
                    corvin.take_sent();
                    ada.run("wow.after = {}");
                }
                PairStep::ReloadAda => ada = reloaded(&ada, "Ada", &corvin),
                PairStep::ReloadCorvin => corvin = reloaded(&corvin, "Corvin", &ada),
            }
            prop_assert!(waiting_from_ada(&corvin) <= 1);
            prop_assert!(sent_by_ada(&ada) <= 1);
        }
    }

    #[test]
    fn the_drafts_never_pass_ten_and_save_never_drops_one(
        saves in prop::collection::vec(prop_oneof![Just(0usize), Just(9), Just(10), Just(11), 0usize..14], 0..40),
    ) {
        let game = Game::new();
        let mut players = std::collections::BTreeSet::new();
        for player in saves {
            let saved: bool = game.eval(&format!(
                "return ns.StoryDrafts.Save('P{player}-Stormrage', '', 'Text.')"
            ));
            prop_assert_eq!(saved, players.contains(&player) || players.len() < 10);
            if saved {
                players.insert(player);
            }
            prop_assert_eq!(game.eval::<usize>("return #ns.StoryDrafts.All()"), players.len());
        }
    }

    #[test]
    fn encode_then_decode_gives_any_clean_story_back(body in clean_body()) {
        let game = Game::new();
        prop_assume!(is_body(&game, &body));

        let trip: mlua::Function = game.eval(
            "return function(body)
                 return ns.TaskWire.Decode(ns.TaskWire.Encode({ type = 'story', id = 'a1',
                     story_title = '', body = body })).body
             end",
        );

        prop_assert_eq!(trip.call::<String>(body.clone()).unwrap(), body);
    }

    #[test]
    fn every_story_that_the_scroll_takes_fits_sixteen_parts(body in clean_body()) {
        let game = Game::new();
        prop_assume!(is_body(&game, &body));

        prop_assert!(parts_of(&game, &"%".repeat(60), &body) <= 16);
    }
}
