//! Stories that players tell about each other, between two games with Timeways (GAMEPLAY.md
//! 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use common::Game;
use players::{Player, ada_and_corvin, deliver, exchange, peer_marks_disagree};
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
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
fn a_story_with_a_marked_title_names_the_player_in_the_chat() {
    let (ada, corvin) = party();
    ada_saw_lysara(&ada);

    send(&ada, &corvin, "Lysara at the Bridge", "We held it.");

    let title: String = corvin.eval("return ns.PlayerStories.Waiting()[1].title");
    assert_eq!(title, "{Lysara} at the Bridge");
    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Ada told a story about you: Lysara at the Bridge. Type /stories to read it."
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
    journal_with(&corvin, &[1]);
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
    // The mark of the reader stays a mark: the desktop reads it as `$N`.
    assert_eq!(paragraphs, ["{Corvin} and {Ada} held it.", "{Ada} ran."]);
    let described = inputs
        .iter()
        .position(|input| matches!(input, Input::PlayerDescribed { name, .. } if name == "Ada"));
    let story = inputs
        .iter()
        .position(|input| matches!(input, Input::StoryAccepted { .. }));
    assert!(described < story && described.is_some(), "{inputs:?}");
    journal_with(&corvin, &[1]);
    exchange(&ada, &corvin);
    let author: String = corvin.eval("return ns.PlayerStories.AuthorOf(1)");
    assert_eq!(author, "Ada-Stormrage");
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
}

/// The lines of the stories page of the journal, with the newest waiting story open.
fn story_page(corvin: &Player) -> Vec<String> {
    corvin.eval(
        "local story = ns.PlayerStories.Waiting()[#ns.PlayerStories.Waiting()]
         ns.Journal.Select('stories', 'w:' .. story.author .. ':' .. story.id)
         local out = {}
         for _, line in ipairs(ns.Journal.Lines('stories')) do out[#out + 1] = line.text end
         return out",
    )
}

/// Ada saw Lysara, a stranger to Corvin: Ada targeted her once.
fn ada_saw_lysara(ada: &Player) {
    ada.run(
        "wow.units.target = { name = 'Lysara', player = true, guid = 'Player-1-Lysara' }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.units.target = nil",
    );
}

#[test]
fn the_author_marks_a_player_that_only_the_author_knows_and_the_story_reaches_the_desktop_with_ids()
{
    let (ada, corvin) = party();
    ada_saw_lysara(&ada);
    send(
        &ada,
        &corvin,
        "Lysara and Me",
        "We held the bridge with Lysara.",
    );

    let waiting: String = corvin.eval("return ns.PlayerStories.Waiting()[1].text");
    let page = story_page(&corvin).join("\n");
    corvin.run("wow.Slash('/story', 'accept')");
    let (title, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    let mut desktop = desktop_of("Corvin");
    let stored = accept_on_desktop(&mut desktop, title, paragraphs.clone());

    assert_eq!(waiting, "We held the bridge with {Lysara}.");
    assert!(page.contains("We held the bridge with Lysara."), "{page}");
    assert!(!page.contains('{'), "{page}");
    assert_eq!(paragraphs, ["We held the bridge with {Lysara}."]);
    assert_eq!(stored, ["We held the bridge with Lysara."]);
}

#[test]
fn a_story_from_an_old_author_with_no_marks_still_shows_and_marks_what_the_reader_knows() {
    let (_, corvin) = party();

    hear_story(&corvin, "a1;;Ada and Lysara held it.");
    let page = story_page(&corvin).join("\n");
    corvin.run("wow.Slash('/story', 'accept')");

    assert!(page.contains("Ada and Lysara held it."), "{page}");
    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    assert_eq!(paragraphs, ["{Ada} and Lysara held it."]);
}

#[test]
fn a_mark_of_the_reader_in_a_story_becomes_the_reader() {
    let (ada, corvin) = party();
    send(&ada, &corvin, "", "Corvin and Ada held it.");

    corvin.run("wow.Slash('/story', 'accept')");
    let (title, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    let shown = accept_on_desktop(&mut desktop_of("Corvin"), title, paragraphs);

    assert_eq!(shown, ["$N and Ada held it."]);
}

#[test]
fn the_waiting_page_names_the_players_of_the_marks() {
    let (ada, corvin) = party();
    ada_saw_lysara(&ada);
    send(&ada, &corvin, "", "We held the bridge with Lysara.");

    let page = story_page(&corvin);

    assert!(
        page.contains(&"Players named: Lysara".to_string()),
        "{page:?}"
    );
}

#[test]
fn a_brace_that_the_author_typed_never_becomes_a_mark_also_in_a_long_story() {
    let (ada, corvin) = party();
    let filler = "a".repeat(1000 - "{grin} Corvin ".len());
    send(&ada, &corvin, "", &format!("{{grin}} Corvin {filler}"));

    let waiting: String = corvin.eval("return ns.PlayerStories.Waiting()[1].text");

    assert!(waiting.starts_with("(grin) Corvin "), "{waiting}");
}

#[test]
fn a_mark_that_touches_a_word_or_holds_no_name_is_no_mark() {
    let (_, corvin) = party();
    hear_story(&corvin, "a1;;a{Ada}b {Ada-} {123} {Ada}");

    corvin.run("wow.Slash('/story', 'accept')");

    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    // The reader knows Ada, so the words in the parentheses get marks of the reader.
    assert_eq!(paragraphs, ["a({Ada})b ({Ada}-) (123) {Ada}"]);
}

#[test]
fn accept_reads_the_known_players_once_however_many_marks() {
    let (_, corvin) = party();
    let marks: Vec<String> = (0..10u8)
        .map(|n| format!("{{Q{}}} x", char::from(b'a' + n)))
        .collect();
    hear_story(&corvin, &format!("a1;;{}", marks.join(" ")));
    corvin.run(
        "guildReads = 0
         local read = ns.TaskPeople.GuildNames
         ns.TaskPeople.GuildNames = function() guildReads = guildReads + 1 return read() end",
    );

    corvin.run("wow.Slash('/story', 'accept')");

    let reads: u32 = corvin.eval("return guildReads");
    assert_eq!(reads, 1);
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// A piece of a text from a peer: marks valid and broken, of strangers, of the reader, and
/// surname marks, marks of words that no name can be, a typed `$N` or `$` before a mark,
/// signs that touch them, braces, hidden characters, a name that the reader knows, and plain
/// words.
fn peer_piece() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => prop::sample::select(vec![
            "{Lysara}", "{Zqada-Bean}", "{~Zqada-Bean}", "{Corvin}", "{Zqada-X}", "{~Lysara-X}",
            "{123}", "{Ada-}", "{~}", "{~-X}", "{Lys ara}", "{", "}", "x", " ", "  ", ", ",
            "\u{200B}", "\u{A0}", "\u{301}", "{Ly\u{200B}sara}", "$N", "$", "{N}", "{The}", "{I}",
            "{lysara}", "{LYSARA}", "{Thequickbrownfox}", "ada", "{Abcdefghijkl}",
            "{Abcdefghijklm}", "{İda}", "{ıda}", "{Ада}", "{АДА}", "{ΟΔΥΣΣΕΥΣ}", "{Οδυσσευς}",
            "{Οδυσσευσ}", "{Σας}", "Οδυσσευς",
        ]).prop_map(String::from),
        1 => "[Q][a-z]{1,3}".prop_map(|name| format!("{{{name}}}")),
        1 => prop::sample::select(timeways_story::aliases::FUNCTION_WORDS.to_vec())
            .prop_map(|word| format!("{{{}}}", capitalized(word))),
        2 => "[e-h]{1,5}",
    ]
}

/// How many other players a story names. Most stories land at the limit of 32 or right
/// next to it.
fn players_named() -> impl Strategy<Value = usize> {
    prop_oneof![1 => 0usize..40, 3 => 31usize..=33]
}

/// The texts of a story, the title first, with marks of strangers dealt out over them until
/// the story names `players` other players, as the desktop counts them. The reader's addon
/// can mark one more, Ada.
fn story_texts(title: String, paragraphs: Vec<String>, players: usize) -> Vec<Vec<u8>> {
    let mut texts: Vec<String> = std::iter::once(title).chain(paragraphs).collect();
    let count = texts.len();
    for (n, mark) in stranger_marks(64).into_iter().enumerate() {
        if timeways_story::aliases::other_players(&texts, "Corvin") >= players {
            break;
        }
        let text = &mut texts[n % count];
        text.push(' ');
        text.push_str(&mark);
    }
    texts
        .iter()
        .map(|text| text.trim().as_bytes().to_vec())
        .collect()
}

#[test]
fn the_reader_and_the_desktop_read_the_marks_of_a_peer_alike() {
    let (_, corvin) = party();
    let mut runner = TestRunner::new(ProptestConfig::with_cases(256));
    let text = || prop::collection::vec(peer_piece(), 1..30).prop_map(|parts| parts.concat());
    let story = (
        prop_oneof![Just(String::new()), text()],
        prop::collection::vec(text(), 1..4),
        players_named(),
    );

    let result = runner.run(&story, |(title, paragraphs, players)| {
        let texts = story_texts(title, paragraphs, players);
        let differ = peer_marks_disagree(&corvin, "Corvin", &texts);
        prop_assert!(differ.is_none(), "{}", differ.unwrap_or_default());
        Ok(())
    });

    result.unwrap();
}

#[test]
fn braces_around_a_mark_that_becomes_its_word_never_make_a_new_mark() {
    let (_, corvin) = party();
    let texts = [Vec::new(), b"{{I}}{Qa}".to_vec()];

    let differ = peer_marks_disagree(&corvin, "Corvin", &texts);

    assert!(differ.is_none(), "{}", differ.unwrap_or_default());
}

#[test]
fn a_greek_name_counts_once_wherever_its_sigma_stands() {
    let (_, corvin) = party();
    let strangers = stranger_marks(31).join(" ");

    for names in ["{ΟΔΥΣΣΕΥΣ} {Οδυσσευς}", "{ΟΔΥΣΣΕΥΣ} {Οδυσσευσ}"]
    {
        let texts = [Vec::new(), format!("{strangers} {names}").into_bytes()];

        let differ = peer_marks_disagree(&corvin, "Corvin", &texts);

        assert!(differ.is_none(), "{names}: {}", differ.unwrap_or_default());
    }
}

/// Marks of strangers, "{Qaa}", "{Qab}", and so on.
fn stranger_marks(count: u8) -> Vec<String> {
    (0..count)
        .map(|n| {
            format!(
                "{{Q{}{}}}",
                char::from(b'a' + n / 26),
                char::from(b'a' + n % 26)
            )
        })
        .collect()
}

const TOO_MANY: &str =
    "This story names too many players to keep. Decline it, or ask Ada for one with fewer players.";

#[test]
fn a_story_counts_its_players_once_and_keeps_every_mark_it_sends() {
    let (_, corvin) = party();
    let marks = stranger_marks(31);
    hear_story(
        &corvin,
        &format!("a1;;{}%0A{{Corvin}} and Ada ran.", marks.join(" ")),
    );

    corvin.run("wow.Slash('/story', 'accept')");
    let (title, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    let kept = desktop_of("Corvin").handle(Input::StoryAccepted {
        at: hourglass::Tick(1),
        number: 1,
        title,
        paragraphs: paragraphs.clone(),
    });

    assert_eq!(paragraphs[1], "{Corvin} and {Ada} ran.");
    assert_eq!(paragraphs[0].matches('{').count(), 31, "{}", paragraphs[0]);
    assert!(kept.is_ok(), "{kept:?}");
}

#[test]
fn a_story_with_one_player_past_the_limit_stays_waiting_whole() {
    let (_, corvin) = party();
    let marks = stranger_marks(32);
    hear_story(&corvin, &format!("a1;;{}%0AAda ran.", marks.join(" ")));

    let problem: String = corvin.eval("return ns.PlayerStories.Accept()");

    assert_eq!(problem, TOO_MANY);
    assert_eq!(waiting(&corvin), 1);
    assert_eq!(accepted(&sent_inputs(&corvin)), None);
}

#[test]
fn the_page_names_the_players_of_a_story_that_names_too_many() {
    let (_, corvin) = party();
    let marks = stranger_marks(32);
    hear_story(&corvin, &format!("a1;;{}%0AAda ran.", marks.join(" ")));

    let page = story_page(&corvin);

    let named = format!(
        "Players named: {}",
        marks.join(", ").replace(['{', '}'], "")
    );
    assert!(page.contains(&named), "{page:?}");
}

#[test]
fn a_story_counts_a_dotted_capital_i_as_the_desktop_does() {
    let (_, corvin) = party();
    let marks = stranger_marks(30);
    hear_story(
        &corvin,
        &format!("a1;;{} {{İda}} {{ıda}} Ada", marks.join(" ")),
    );

    let problem: String = corvin.eval("return ns.PlayerStories.Accept()");

    assert_eq!(problem, TOO_MANY);
    assert_eq!(waiting(&corvin), 1);
}

#[test]
fn a_story_counts_a_cyrillic_name_in_two_cases_once() {
    let (_, corvin) = party();
    let marks = stranger_marks(30);
    hear_story(
        &corvin,
        &format!("a1;;{} {{Ада}} {{АДА}} Ada", marks.join(" ")),
    );

    let page = story_page(&corvin);
    corvin.run("wow.Slash('/story', 'accept')");

    let named = page.iter().find(|line| line.starts_with("Players named:"));
    assert!(
        named.is_some_and(|line| line.ends_with(", Qbd, Ада")),
        "{page:?}"
    );
    assert!(accepted(&sent_inputs(&corvin)).is_some());
}

#[test]
fn a_story_that_names_too_many_players_you_know_stays_waiting() {
    let (_, corvin) = party();
    let names: Vec<String> = (0..33u8)
        .map(|n| {
            format!(
                "Q{}{}",
                char::from(b'a' + n / 26),
                char::from(b'a' + n % 26)
            )
        })
        .collect();
    let roster: Vec<String> = names
        .iter()
        .map(|name| format!("{{ name = '{name}-Stormrage', online = false }}"))
        .collect();
    corvin.run(&format!(
        "wow.guild = {{ {} }} wow.Fire('GUILD_ROSTER_UPDATE', false)",
        roster.join(", ")
    ));
    hear_story(&corvin, &format!("a1;;{}", names.join(" ")));

    let problem: String = corvin.eval("return ns.PlayerStories.Accept()");

    assert_eq!(problem, TOO_MANY);
    assert_eq!(waiting(&corvin), 1);
    assert_eq!(accepted(&sent_inputs(&corvin)), None);
}

#[test]
fn a_mark_of_a_word_that_no_name_can_be_is_its_word() {
    let (_, corvin) = party();
    hear_story(&corvin, "a1;;{The} wolf {A} came {I}, {Thequickbrownfox}.");

    let page = story_page(&corvin).join("\n");
    corvin.run("wow.Slash('/story', 'accept')");

    // "The" can be a name of the game, so its mark stays: the desktop swaps it only with a
    // capital.
    assert!(
        page.contains("The wolf A came I, Thequickbrownfox."),
        "{page}"
    );
    assert!(page.contains("Players named: The\n"), "{page}");
    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    assert_eq!(paragraphs, ["{The} wolf A came I, Thequickbrownfox."]);
}

#[test]
fn a_typed_name_mark_of_a_peer_is_no_name_mark() {
    let (_, corvin) = party();
    hear_story(&corvin, "a1;;$N and {Lysara} and {lysara} and {LYSARA}");

    let page = story_page(&corvin).join("\n");
    corvin.run("wow.Slash('/story', 'accept')");

    assert!(
        page.contains("N and Lysara and lysara and LYSARA"),
        "{page}"
    );
    assert!(page.contains("Players named: Lysara\n"), "{page}");
    let (_, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    assert_eq!(paragraphs, ["N and {Lysara} and {lysara} and {LYSARA}"]);
}

#[test]
fn a_typed_name_mark_never_comes_out_of_a_peer_story() {
    let (_, corvin) = party();
    hear_story(
        &corvin,
        "a1;;We met ${N} at dawn, $${N}, $$N, ${~Ab-Nash} and ${Nadia}.",
    );

    let page = story_page(&corvin).join("\n");
    corvin.run("wow.Slash('/story', 'accept')");
    let (title, paragraphs) = accepted(&sent_inputs(&corvin)).expect("an accepted story");
    let stored = accept_on_desktop(&mut desktop_of("Corvin"), title, paragraphs.clone());

    assert!(
        page.contains("We met N at dawn, N, N, Nash and Nadia."),
        "{page}"
    );
    assert_eq!(
        paragraphs,
        ["We met N at dawn, N, N, {~Ab-Nash} and {Nadia}."]
    );
    assert_eq!(stored, ["We met N at dawn, N, N, Nash and Nadia."]);
}

/// The journal of Corvin's desktop, with these accepted stories by number, each accepted now.
fn journal_with(corvin: &Player, numbers: &[u64]) {
    let now: i64 = corvin.eval("return time()");
    let stories: Vec<(u64, i64)> = numbers.iter().map(|number| (*number, now)).collect();
    journal_of(corvin, &stories);
}

/// The journal of Corvin's desktop, with these accepted stories by number and time.
fn journal_of(corvin: &Player, stories: &[(u64, i64)]) {
    let stories: Vec<String> = stories
        .iter()
        .map(|(number, at)| format!(r#"{{"number":{number},"paragraphs":["x"],"at":{at}}}"#))
        .collect();
    corvin.game.reply(&format!(
        r#"{{"type":"journal","page":0,"pages":1,"stories":[{}]}}"#,
        stories.join(",")
    ));
}

#[test]
fn an_accepted_story_waits_as_saving_until_the_journal_shows_it_kept() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");

    corvin.run("wow.Slash('/story', 'accept')");
    exchange(&ada, &corvin);
    let saving: bool = corvin.eval("return ns.PlayerStories.Waiting()[1].keeping ~= nil");
    let heard_early = ada
        .printed()
        .contains(&format!("{PREFIX}Corvin accepted your story."));
    journal_with(&corvin, &[1]);
    exchange(&ada, &corvin);

    assert!(saving);
    assert!(!heard_early);
    assert_eq!(waiting(&corvin), 0);
    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Story accepted. It's part of your story now."
    )));
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
}

#[test]
fn a_story_that_the_desktop_never_kept_waits_for_accept_again() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.run("wow.Slash('/story', 'accept')");
    let saving = || corvin.eval::<bool>("return ns.PlayerStories.Waiting()[1].keeping ~= nil");

    let mut saving_after = Vec::new();
    for _ in 0..3 {
        journal_with(&corvin, &[]);
        saving_after.push(saving());
    }

    assert_eq!(waiting(&corvin), 1);
    assert_eq!(saving_after, [true, true, false]);
    // A full alias table refuses the story again, so the line gives no advice to retry.
    assert!(corvin.printed().contains(&format!(
        "{PREFIX}Couldn't keep the story from Ada. It's waiting for your answer again."
    )));
}

#[test]
fn decline_on_a_saving_story_sends_no_answer_and_the_story_keeps_saving() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.run("wow.Slash('/story', 'accept')");

    corvin.run("ns.PlayerStories.Decline(1)");
    exchange(&ada, &corvin);
    let declined = ada
        .printed()
        .contains(&format!("{PREFIX}Corvin declined your story."));
    journal_with(&corvin, &[1]);
    exchange(&ada, &corvin);

    assert!(!declined);
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
}

#[test]
fn block_keeps_a_saving_story_and_its_author_still_hears_the_answer() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.run("wow.Slash('/story', 'accept')");

    corvin.run("ns.PlayerStories.Block('Ada-Stormrage')");
    let saving: bool = corvin.eval("return ns.PlayerStories.Waiting()[1].keeping ~= nil");
    journal_with(&corvin, &[1]);
    exchange(&ada, &corvin);

    assert!(saving);
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Corvin accepted your story."))
    );
    assert!(corvin.eval::<bool>("return ns.TaskStore.Data().blocked['Ada-Stormrage'] ~= nil"));
}

/// The number of the last story that the game sent to the desktop.
fn last_accepted_number(player: &Player) -> Option<u64> {
    sent_inputs(player)
        .iter()
        .rev()
        .find_map(|input| match input {
            Input::StoryAccepted { number, .. } => Some(*number),
            _ => None,
        })
}

fn heard_accepted(ada: &Player) -> usize {
    let line = format!("{PREFIX}Corvin accepted your story.");
    ada.printed()
        .iter()
        .filter(|printed| **printed == line)
        .count()
}

#[test]
fn accept_after_a_journal_takes_a_number_past_every_story_of_the_journal() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.run("ns.StorySaved.Data().nextNumber = 1");

    journal_with(&corvin, &[1, 2, 3]);
    corvin.run("wow.Slash('/story', 'accept')");

    assert_eq!(last_accepted_number(&corvin), Some(4));
}

#[test]
fn a_story_number_that_the_desktop_already_holds_never_counts_as_kept() {
    let (ada, corvin) = party();
    tell(&ada, &corvin, "The first.");
    corvin.run("wow.Slash('/story', 'accept')");
    let first_at: i64 = corvin.eval("return time()");
    journal_with(&corvin, &[1]);
    exchange(&ada, &corvin);
    // A crash before the game wrote the saved variables takes the number back.
    corvin.run("ns.StorySaved.Data().nextNumber = 1 wow.now = wow.now + 60");
    tell(&ada, &corvin, "The second.");

    corvin.run("wow.Slash('/story', 'accept')");
    let reused = last_accepted_number(&corvin);
    for _ in 0..3 {
        journal_of(&corvin, &[(1, first_at)]);
    }
    exchange(&ada, &corvin);
    let heard_while_refused = heard_accepted(&ada);
    corvin.run("wow.Slash('/story', 'accept')");
    let fresh = last_accepted_number(&corvin);
    journal_with(&corvin, &[1, 2]);
    exchange(&ada, &corvin);

    assert_eq!(reused, Some(1));
    assert_eq!(heard_while_refused, 1);
    assert_eq!(fresh, Some(2));
    assert_eq!(heard_accepted(&ada), 2);
    assert_eq!(waiting(&corvin), 0);
}

/// A real story program behind Corvin's game: it plays each message that the game sent,
/// and its journal goes back to the game.
struct Desktop {
    story: timeways_story::story::Story,
    played: usize,
}

impl Desktop {
    fn new() -> Desktop {
        let pack = timeways_story::pack::Pack::empty().unwrap();
        let story = timeways_story::story::Story::new(pack, timeways_story::store::Store::Memory);
        Desktop { story, played: 0 }
    }

    fn play(&mut self, line: &str) {
        let served = timeways_story::serve::line(&mut self.story, line.as_bytes().to_vec());
        assert_eq!(served.error, None, "{line}");
    }

    fn sync(&mut self, corvin: &Player) {
        corvin.run("wow.RunTickers()");
        let sent = corvin.game.sent();
        for message in &sent[self.played..] {
            for line in message.lines() {
                let _ = timeways_story::serve::line(&mut self.story, line.as_bytes().to_vec());
            }
        }
        self.played = sent.len();
        let ask = r#"{"type":"journal_asked","id":1}"#;
        let journal = timeways_story::serve::line(&mut self.story, ask.as_bytes().to_vec());
        corvin.game.reply(&journal.lines[0]);
    }
}

#[test]
fn a_story_counts_as_kept_when_the_desktop_moved_its_time_up_to_the_world() {
    let (ada, corvin) = party();
    let mut desktop = Desktop::new();
    tell(&ada, &corvin, "Ada and I held the bridge.");
    corvin.run("ns.Journal.Request(0)");
    desktop.sync(&corvin);
    // The world is ahead of the clock of the game, as after the clock went back.
    let ahead: i64 = corvin.eval("return time() + 100");
    desktop.play(&format!(
        r#"{{"type":"rest_changed","at":{ahead},"resting":"yes"}}"#
    ));

    corvin.run("wow.Slash('/story', 'accept')");
    desktop.sync(&corvin);
    exchange(&ada, &corvin);

    assert_eq!(waiting(&corvin), 0);
    assert_eq!(heard_accepted(&ada), 1);
}

/// A desktop with this character, as the story program runs it.
fn desktop_of(name: &str) -> timeways_story::story::Story {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("player-stories-pack.sqlite");
    if !path.exists() {
        timeways_story::pack::Pack::write(&path, &[]).unwrap();
    }
    let pack = timeways_story::pack::Pack::open(&path).unwrap();
    let mut story = timeways_story::story::Story::new(pack, timeways_story::store::Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: name.to_string(),
        })
        .unwrap();
    story
}

/// Accepts the story on the desktop, and gives the paragraphs that its journal shows.
fn accept_on_desktop(
    desktop: &mut timeways_story::story::Story,
    title: Option<String>,
    paragraphs: Vec<String>,
) -> Vec<String> {
    desktop
        .handle(Input::StoryAccepted {
            at: hourglass::Tick(1),
            number: 1,
            title,
            paragraphs,
        })
        .unwrap();
    let asked = Input::JournalAsked {
        id: timeways_story::input::MessageId(1),
        page: 0,
    };
    match desktop.handle(asked).unwrap().remove(0) {
        timeways_story::story::Output::Journal { page, .. } => {
            page.journal.stories[0].paragraphs.clone()
        }
        other => panic!("no journal: {other:?}"),
    }
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
fn the_story_help_says_that_blizzard_can_read_a_story() {
    let (ada, _) = party();

    ada.run("wow.Slash('/story', 'help')");

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
fn a_broken_keeping_in_the_saved_file_is_dropped_on_first_read() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { waiting = {
             { id = 'a1', author = 'Bram-Stormrage', title = '', text = 'One.', at = 1,
               keeping = { number = 1.5, journals = 0, at = 1 } },
             { id = 'a2', author = 'Cora-Stormrage', title = '', text = 'Two.', at = 1,
               keeping = { number = 2, journals = -1, at = 1 } },
             { id = 'a3', author = 'Dax-Stormrage', title = '', text = 'Three.', at = 1,
               keeping = { number = 3, journals = 0, at = 'soon' } },
             { id = 'a4', author = 'Eli-Stormrage', title = '', text = 'Four.', at = 1,
               keeping = 'saving' },
             { id = 'a5', author = 'Fen-Stormrage', title = '', text = 'Five.', at = 1,
               keeping = { number = 5, journals = 1, at = 1 } } } }",
    );

    let saving: Vec<bool> = game.eval(
        "local out = {}
         for _, story in ipairs(ns.PlayerStories.Waiting()) do out[#out + 1] = story.keeping ~= nil end
         return out",
    );

    assert_eq!(saving, [false, false, false, false, true]);
    assert!(game.eval::<bool>("return TimewaysStories.waiting[1].keeping == nil"));
}

#[test]
fn a_saved_waiting_story_with_a_time_past_the_wire_is_dropped() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { waiting = {
             { id = 'a1', author = 'Bram-Stormrage', title = '', text = 'Huge.', at = 1e300 },
             { id = 'a2', author = 'Cora-Stormrage', title = '', text = 'Fine.', at = 4294967295 } } }",
    );

    let kept: Vec<String> = game.eval(
        "local out = {}
         for _, story in ipairs(ns.PlayerStories.Waiting()) do out[#out + 1] = story.id end
         return out",
    );

    assert_eq!(kept, ["a2"]);
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
fn a_draft_with_a_typed_bar_survives_a_reload() {
    let game = Game::new();
    game.run(
        "TimewaysStories = { drafts = {
             { to = 'Corvin-Stormrage', title = 'A || title', text = 'One ||.\\n\\nTwo.', at = 1 },
             { to = 'Bram-Stormrage', title = '', text = 'A bell\\7.', at = 1 } } }",
    );

    let kept: Vec<String> = game.eval(
        "local out = {}
         for _, draft in ipairs(ns.StoryDrafts.All()) do table.insert(out, draft.to) end
         return out",
    );

    assert_eq!(kept, ["Corvin-Stormrage"]);
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
