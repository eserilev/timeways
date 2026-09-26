#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};
use timeways_story::lore::Answer;
use timeways_story::pack::{Origin, Passage};
use timeways_story::story::Output;

const NOW: Tick = Tick(1_790_000_000);

fn zone(zone: &str, subzone: Option<&str>) -> Input {
    Input::ZoneEntered {
        at: NOW,
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
    }
}

fn reply_line(text: Option<&str>, passages: &[(&str, &str)]) -> String {
    let passages = passages
        .iter()
        .map(|(text, source)| Passage {
            text: (*text).to_string(),
            source: (*source).to_string(),
            links: Vec::new(),
            origin: Origin::Pack,
        })
        .collect();
    let answer = Answer {
        text: text.map(str::to_string),
        passages,
    };
    serde_json::to_string(&Output::LoreAnswer {
        id: MessageId(1),
        answer,
    })
    .unwrap()
}

#[test]
fn login_sends_the_level_and_the_zone() {
    let game = Game::new();
    game.run(
        "wow.units.player = { name = 'Ada', level = 12 }
         wow.zone, wow.subzone = 'Elwynn Forest', 'Goldshire'
         wow.Fire('PLAYER_ENTERING_WORLD')
         wow.RunTickers()",
    );

    let expected = [
        Input::LevelReached { at: NOW, level: 12 },
        zone("Elwynn Forest", Some("Goldshire")),
    ];
    assert_eq!(game.sent().len(), 1);
    assert_eq!(game.sent_inputs(), expected);
}

#[test]
fn events_wait_for_the_timer() {
    let game = Game::new();
    game.run("wow.zone = 'Elwynn Forest'; wow.Fire('ZONE_CHANGED_NEW_AREA')");

    assert!(game.sent().is_empty());
    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 1);
}

#[test]
fn an_empty_subzone_is_left_out() {
    let game = Game::new();
    game.run("wow.zone = 'Elwynn Forest'; wow.Fire('ZONE_CHANGED_NEW_AREA'); wow.RunTickers()");

    assert_eq!(game.sent_inputs(), [zone("Elwynn Forest", None)]);
}

#[test]
fn the_same_place_twice_is_sent_once() {
    let game = Game::new();
    game.run(
        "wow.zone, wow.subzone = 'Elwynn Forest', 'Goldshire'
         wow.Fire('ZONE_CHANGED')
         wow.Fire('ZONE_CHANGED_INDOORS')
         wow.RunTickers()",
    );

    assert_eq!(
        game.sent_inputs(),
        [zone("Elwynn Forest", Some("Goldshire"))]
    );
}

#[test]
fn a_loading_screen_sends_no_zone() {
    let game = Game::new();
    game.run("wow.zone = ''; wow.Fire('ZONE_CHANGED_NEW_AREA'); wow.RunTickers()");

    assert!(game.sent().is_empty());
}

#[test]
fn an_npc_is_sent_once_per_session() {
    let game = Game::new();
    game.run(
        "wow.units.npc = { name = 'Innkeeper Farley' }
         wow.Fire('GOSSIP_SHOW')
         wow.Fire('QUEST_DETAIL')
         wow.RunTickers()",
    );

    let met = Input::NpcMet {
        at: NOW,
        name: "Innkeeper Farley".to_string(),
    };
    assert_eq!(game.sent_inputs(), [met]);
}

#[test]
fn a_level_up_sends_the_new_level() {
    let game = Game::new();
    game.run("wow.Fire('PLAYER_LEVEL_UP', 13); wow.RunTickers()");

    assert_eq!(
        game.sent_inputs(),
        [Input::LevelReached { at: NOW, level: 13 }]
    );
}

#[test]
fn a_question_goes_out_at_once_after_the_waiting_events() {
    let game = Game::new();
    game.run(
        "wow.zone = 'Elwynn Forest'
         wow.Fire('ZONE_CHANGED_NEW_AREA')
         wow.Slash('/lore', '  why is this tower in ruins?  ')",
    );

    let question = Input::LoreAsked {
        id: MessageId(1),
        question: "why is this tower in ruins?".to_string(),
        target: None,
    };
    assert_eq!(game.sent().len(), 1);
    assert_eq!(game.sent_inputs(), [zone("Elwynn Forest", None), question]);
}

#[test]
fn a_question_takes_the_name_of_an_npc_target() {
    let game = Game::new();
    game.run("wow.units.target = { name = 'Hogger' }; wow.Slash('/lore', 'who is this?')");

    let inputs = game.sent_inputs();
    assert!(
        matches!(&inputs[..], [Input::LoreAsked { target: Some(name), .. }] if name == "Hogger"),
        "{inputs:?}"
    );
}

#[test]
fn a_question_never_sends_the_name_of_a_player() {
    let game = Game::new();
    game.run(
        "wow.units.target = { name = 'Grimtusk', player = true }
         wow.Slash('/lore', 'who is this?')",
    );

    assert!(!game.sent()[0].contains("Grimtusk"));
    assert!(matches!(
        &game.sent_inputs()[..],
        [Input::LoreAsked { target: None, .. }]
    ));
}

#[test]
fn an_empty_question_shows_help_and_sends_nothing() {
    let game = Game::new();
    game.run("wow.Slash('/lore', '   ')");

    assert!(game.sent().is_empty());
    assert!(game.printed()[0].contains("/lore why is this tower in ruins?"));
}

#[test]
fn a_question_too_long_for_one_message_is_refused() {
    let game = Game::new();
    game.run("wow.Slash('/lore', string.rep('why ', 1000))");

    assert!(game.sent().is_empty());
    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 0);
    assert!(game.printed()[0].contains("too long"));
}

#[test]
fn events_that_do_not_fit_in_one_message_go_in_several_in_order() {
    let game = Game::new();
    game.run(
        "linkLimit = 120
         for level = 2, 6 do wow.Fire('PLAYER_LEVEL_UP', level) end
         wow.RunTickers()",
    );

    let levels: Vec<u8> = game
        .sent_inputs()
        .into_iter()
        .map(|input| match input {
            Input::LevelReached { level, .. } => level,
            other => panic!("expected a level, got {other:?}"),
        })
        .collect();
    assert!(game.sent().len() > 1);
    assert!(game.sent().iter().all(|message| message.len() <= 120));
    assert_eq!(levels, [2, 3, 4, 5, 6]);
}

#[test]
fn events_wait_while_the_link_is_down_and_then_go_in_order() {
    let game = Game::new();
    game.run(
        "linkUp = false
         wow.Fire('PLAYER_LEVEL_UP', 2)
         wow.RunTickers()
         wow.Fire('PLAYER_LEVEL_UP', 3)
         linkUp = true
         wow.RunTickers()",
    );

    let expected = [
        Input::LevelReached { at: NOW, level: 2 },
        Input::LevelReached { at: NOW, level: 3 },
    ];
    assert_eq!(game.sent_inputs(), expected);
}

#[test]
fn the_outbox_keeps_the_newest_five_hundred_events() {
    let game = Game::new();
    game.run("linkUp = false; for n = 1, 510 do wow.Fire('PLAYER_LEVEL_UP', n % 60 + 1) end");

    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 500);
}

#[test]
fn an_answer_shows_its_text_and_numbered_sources() {
    let game = Game::new();

    game.reply(&reply_line(
        Some("Goblins burned it [1]."),
        &[("The tower fell.", "https://example.test/tower")],
    ));

    let printed = game.printed();
    assert_eq!(printed.len(), 2);
    assert!(printed[0].ends_with("Goblins burned it [1]."));
    assert!(printed[1].ends_with("[1] example.test/tower"));
}

#[test]
fn an_answer_with_no_text_shows_the_passages_as_they_are() {
    let game = Game::new();

    game.reply(&reply_line(
        None,
        &[("The tower fell.", "https://example.test/tower")],
    ));

    assert_eq!(game.printed().len(), 1);
    assert!(game.printed()[0].ends_with("[1] The tower fell. (example.test/tower)"));
}

#[test]
fn an_answer_with_nothing_says_that_nobody_knows() {
    let game = Game::new();

    game.reply(&reply_line(None, &[]));

    assert!(game.printed()[0].ends_with("Nobody here knows."));
}

#[test]
fn an_answer_that_the_bridge_escaped_shows_as_it_is() {
    let game = Game::new();

    game.reply(&reply_line(
        Some("||cffff0000red||r ||Hitem:1||h[Fake]||h [1]"),
        &[("x", "y")],
    ));

    assert!(game.printed()[0].ends_with("||cffff0000red||r ||Hitem:1||h[Fake]||h [1]"));
}

#[test]
fn a_reply_to_events_or_a_broken_reply_shows_nothing() {
    let game = Game::new();

    game.reply("");
    game.reply("not json");
    game.reply(r#"{"type":"something_else"}"#);

    assert!(game.printed().is_empty());
}

#[test]
fn every_registered_event_has_a_handler() {
    let game = Game::new();

    let registered: u32 = game.eval(
        "local count = 0
         for _, frame in ipairs(wow.widgets) do
             for event in pairs(frame.events) do
                 wow.Fire(event)
                 count = count + 1
             end
         end
         return count",
    );

    // 17 of Timeways, and the 2 screenshot events of the shared Strip.lua.
    assert_eq!(registered, 19);
}

#[test]
fn a_question_and_a_journal_request_never_share_a_batch() {
    let game = Game::new();
    game.run(
        "linkUp = false
         wow.Slash('/lore', 'who built this?')
         wow.Slash('/journal', '')
         wow.Fire('PLAYER_LEVEL_UP', 3)
         linkUp = true
         wow.RunTickers()",
    );

    let sent = game.sent();
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert!(sent[0].contains("lore_asked"));
    assert!(sent[1].contains("journal_asked"));
    assert!(sent[2].contains("level_reached"));
}

#[test]
fn every_message_starts_with_the_character_line() {
    let game = Game::new();
    game.run(
        "linkLimit = 200
         for level = 2, 8 do wow.Fire('PLAYER_LEVEL_UP', level) end
         wow.RunTickers()",
    );

    let character = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: "Ada".to_string(),
    };
    assert!(game.sent().len() > 1);
    for message in game.sent() {
        let first: Input = serde_json::from_str(message.lines().next().unwrap()).unwrap();
        assert_eq!(first, character);
    }
}

#[test]
fn login_names_the_character_of_this_session() {
    let game = Game::before_login();
    game.run(
        "wow.realm = \"Quel'Thalas\"
         wow.units.player = { name = 'Bren', level = 5 }
         wow.Fire('PLAYER_ENTERING_WORLD')
         wow.RunTickers()",
    );

    let first: Input = serde_json::from_str(game.sent()[0].lines().next().unwrap()).unwrap();
    assert_eq!(
        first,
        Input::CharacterEntered {
            realm: "Quel'Thalas".to_string(),
            name: "Bren".to_string()
        }
    );
}

#[test]
fn events_wait_until_the_character_is_known() {
    let game = Game::before_login();
    game.run("wow.Fire('PLAYER_LEVEL_UP', 2); wow.RunTickers()");

    assert!(game.sent().is_empty());
    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 1);
}

#[test]
fn a_party_member_who_shares_a_quest_is_never_sent() {
    let game = Game::new();
    game.run(
        "wow.units.npc = { name = 'Grimtusk', player = true }
         wow.Fire('QUEST_DETAIL')
         wow.RunTickers()",
    );

    assert!(game.sent().is_empty());
}

#[test]
fn a_question_that_fits_only_without_the_character_line_is_refused() {
    let game = Game::new();
    game.run("linkLimit = 120; wow.Slash('/lore', string.rep('w', 70))");

    assert!(game.sent().is_empty());
    assert!(game.printed()[0].contains("too long"));
}

#[test]
fn an_entry_that_can_never_fit_is_dropped_and_blocks_nothing() {
    let game = Game::new();
    game.run(
        "wow.units.npc = { name = string.rep('N', 200) }
         wow.Fire('GOSSIP_SHOW')
         linkLimit = 150
         wow.Fire('PLAYER_LEVEL_UP', 4)
         wow.RunTickers()",
    );

    assert_eq!(
        game.sent_inputs(),
        [Input::LevelReached { at: NOW, level: 4 }]
    );
    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 0);
}

#[test]
fn a_broken_passage_keeps_the_numbers_of_the_others() {
    let game = Game::new();

    game.reply(r#"{"type":"lore_answer","id":1,"text":"It fell [2].","passages":[1,{"text":"a","source":"https://b"}]}"#);

    assert!(game.printed()[1].ends_with("[2] b"), "{:?}", game.printed());
}

#[test]
fn a_narrator_line_shows_from_the_answer_to_a_batch() {
    let game = Game::new();

    game.reply(r#"{"type":"events_seen","id":3,"narrator":"Level 13! Your boots still squeak."}"#);

    assert_eq!(game.printed().len(), 1);
    assert!(game.printed()[0].ends_with("Narrator|r: Level 13! Your boots still squeak."));
}

#[test]
fn a_batch_with_no_narrator_line_shows_nothing() {
    let game = Game::new();

    game.reply(r#"{"type":"events_seen","id":3,"narrator":null}"#);
    game.reply(r#"{"type":"events_seen","id":4,"narrator":5}"#);

    assert!(game.printed().is_empty());
}

#[test]
fn a_narrator_line_that_the_bridge_escaped_shows_as_it_is() {
    let game = Game::new();

    game.reply(r#"{"type":"events_seen","id":3,"narrator":"||Hitem:1||h[Fake]||h"}"#);

    assert!(game.printed()[0].ends_with("Narrator|r: ||Hitem:1||h[Fake]||h"));
}

#[test]
fn a_lore_answer_can_carry_a_narrator_line_too() {
    let game = Game::new();

    game.reply(
        r#"{"type":"lore_answer","id":1,"text":null,"passages":[],"narrator":"Nobody? Figures."}"#,
    );

    assert_eq!(game.printed().len(), 2);
    assert!(game.printed()[1].ends_with("Narrator|r: Nobody? Figures."));
}
