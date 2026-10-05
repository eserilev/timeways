//! The writing scroll of a story (GAMEPLAY.md 4.8): a title and a body for a player of your
//! group, the question to the player's box, drafts, and the errors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, ada_and_corvin, exchange};

const PREFIX: &str = "|cffc8a064Timeways|r: ";

/// Ada and Corvin in one party, and Ada targets Corvin.
fn party() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run(
        "wow.units.target = { name = 'Corvin', player = true, guid = 'Player-1-Corvin',
             level = 11, race = 'Undead', class = 'Mage' }",
    );
    (ada, corvin)
}

/// `/story` alone, and the question goes both ways.
fn open(ada: &Player, corvin: &Player) {
    ada.run("wow.Slash('/story', '')");
    exchange(ada, corvin);
}

fn status(ada: &Player) -> String {
    ada.eval("return ns.StoryScroll.Status()")
}

/// The box of the scroll: the title, or the body of several lines.
fn box_of(multi_line: bool) -> String {
    format!(
        "(function()
             for _, widget in ipairs(wow.widgets) do
                 if widget.kind == 'EditBox' and widget.parent and (widget.parent == TimewaysStoryScroll
                     or widget.parent.parent == TimewaysStoryScroll)
                     and (widget.multiLine == true) == {multi_line} then
                     return widget
                 end
             end
         end)()"
    )
}

/// Types the text in a box, as the game fires `OnTextChanged`.
fn type_in(ada: &Player, multi_line: bool, text: &str) {
    let write: mlua::Function = ada.eval(&format!(
        "return function(text)
             local box = {}
             box:SetText(text)
             box.scripts.OnTextChanged(box)
         end",
        box_of(multi_line)
    ));
    write.call::<()>(text).unwrap();
}

fn box_text(ada: &Player, multi_line: bool) -> String {
    ada.eval(&format!("return {}:GetText()", box_of(multi_line)))
}

fn button(label: &str) -> String {
    format!(
        "(function()
             for _, widget in ipairs(wow.widgets) do
                 if widget.kind == 'Button' and widget.parent == TimewaysStoryScroll
                     and widget.text == '{label}' then
                     return widget
                 end
             end
         end)()"
    )
}

fn click(ada: &Player, label: &str) {
    ada.run(&format!("{}:Click()", button(label)));
}

fn send_is_on(ada: &Player) -> bool {
    ada.eval(&format!("return {}:IsEnabled()", button("Send")))
}

fn is_open(ada: &Player) -> bool {
    ada.eval("return ns.StoryScroll.IsShown()")
}

fn waiting(corvin: &Player) -> usize {
    corvin.eval("return #ns.PlayerStories.Waiting()")
}

fn run_timers(player: &Player) {
    player.run(
        "local timers = wow.after
         wow.after = {}
         for _, timer in ipairs(timers) do timer.callback() end",
    );
}

#[test]
fn the_scroll_does_not_open_without_a_target_in_your_group() {
    let (ada, corvin) = ada_and_corvin();
    ada.run("wow.units.target = { name = 'Corvin', player = true, guid = 'Player-1-Corvin' }");

    open(&ada, &corvin);

    assert!(!is_open(&ada));
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Target a player in your group first."))
    );
}

#[test]
fn the_scroll_asks_the_target_when_it_opens() {
    let (ada, corvin) = party();

    ada.run("wow.Slash('/story', '')");
    let checking = status(&ada);
    let asked = ada.take_sent().pop().unwrap().text;

    assert!(is_open(&ada));
    assert_eq!(checking, "Checking...");
    assert!(asked.ends_with(";story_ask"), "{asked}");
    corvin.hear("Timeways", &asked, "WHISPER", "Ada-Stormrage");
}

#[test]
fn the_header_names_the_target_with_level_race_and_class() {
    let (ada, corvin) = party();

    open(&ada, &corvin);

    let texts: Vec<String> = ada.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent == TimewaysStoryScroll and widget.text then
                 table.insert(out, widget.text)
             end
         end
         return out",
    );
    for text in [
        "Tell a Story",
        "A story about",
        "Corvin",
        "Level 11 Undead Mage",
        "Like chat, Blizzard can read what you send.",
    ] {
        assert!(texts.contains(&text.to_string()), "{text} in {texts:?}");
    }
}

#[test]
fn send_stays_off_until_the_room_is_open() {
    let (ada, corvin) = party();
    ada.run("wow.Slash('/story', '')");
    type_in(&ada, true, "We held the bridge.");
    let before = send_is_on(&ada);

    exchange(&ada, &corvin);

    assert!(!before);
    assert!(send_is_on(&ada));
    assert_eq!(status(&ada), "");
}

#[test]
fn the_status_line_names_a_full_box_a_waiting_story_and_a_block() {
    let (ada, corvin) = party();
    corvin.run(
        "for n = 1, 20 do
             table.insert(ns.PlayerStories.Waiting(),
                 { id = 'w' .. n, author = 'P' .. n .. '-Stormrage', title = '', text = 'Hi.', at = 1 })
         end",
    );
    open(&ada, &corvin);
    let full = status(&ada);
    click(&ada, "Close");

    corvin.run("ns.PlayerStories.Waiting()[1].author = 'Ada-Stormrage'");
    open(&ada, &corvin);
    let waiting_line = status(&ada);
    click(&ada, "Close");

    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");
    open(&ada, &corvin);

    assert_eq!(full, "Corvin's story box is full.");
    assert_eq!(waiting_line, "Corvin hasn't answered your last story yet.");
    assert_eq!(status(&ada), "Corvin doesn't take stories from you.");
    assert!(!send_is_on(&ada));
}

#[test]
fn the_scroll_says_when_the_target_has_no_timeways() {
    let (ada, _corvin) = party();
    ada.run("wow.Slash('/story', '')");
    ada.take_sent();

    run_timers(&ada);

    assert_eq!(status(&ada), "Corvin needs Timeways to get stories.");
}

#[test]
fn send_asks_again_and_sends_only_on_open() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, false, "The Bridge");
    type_in(&ada, true, "We held it.\n\nThen we ran.");
    corvin.run("ns.TaskStore.Data().blocked['Ada-Stormrage'] = 1");

    click(&ada, "Send");
    let sending = status(&ada);
    exchange(&ada, &corvin);

    assert_eq!(sending, "Sending...");
    assert_eq!(waiting(&corvin), 0);
    assert_eq!(status(&ada), "Corvin doesn't take stories from you.");
    assert_eq!(box_text(&ada, true), "We held it.\n\nThen we ran.");
}

#[test]
fn a_sent_story_goes_with_its_title_and_paragraphs_and_clears_the_scroll() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, false, "  The Bridge ");
    type_in(&ada, true, "We held it.\n\n\nThen we ran.  ");

    click(&ada, "Send");
    exchange(&ada, &corvin);

    let (title, text): (String, String) = (
        corvin.eval("return ns.PlayerStories.Waiting()[1].title"),
        corvin.eval("return ns.PlayerStories.Waiting()[1].text"),
    );
    assert_eq!(
        (title.as_str(), text.as_str()),
        ("The Bridge", "We held it.\nThen we ran.")
    );
    assert_eq!(
        status(&ada),
        "Sent to Corvin. They'll decide if it's part of their story."
    );
    assert_eq!(box_text(&ada, true), "");
}

#[test]
fn enter_starts_a_paragraph_and_ctrl_enter_sends() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, true, "We held it.");

    ada.run(&format!(
        "local box = {} box.scripts.OnEnterPressed(box)",
        box_of(true)
    ));
    let after_enter = box_text(&ada, true);
    ada.run(&format!(
        "wow.ctrl = true local box = {} box.scripts.OnEnterPressed(box)",
        box_of(true)
    ));
    exchange(&ada, &corvin);

    assert_eq!(after_enter, "We held it.\n");
    assert_eq!(waiting(&corvin), 1);
}

#[test]
fn close_saves_a_changed_draft() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, true, "We held it.\n\nThen we ran.");

    click(&ada, "Close");
    open(&ada, &corvin);

    assert!(is_open(&ada));
    assert_eq!(box_text(&ada, true), "We held it.\n\nThen we ran.");
}

#[test]
fn save_keeps_the_draft_and_closes_the_scroll() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, false, "A Title");

    click(&ada, "Save");

    assert!(!is_open(&ada));
    assert!(
        ada.printed()
            .contains(&format!("{PREFIX}Saved. Find it under Drafts in Stories."))
    );
    let title: String = ada.eval("return ns.StoryDrafts.For('Corvin-Stormrage').title");
    assert_eq!(title, "A Title");
}

#[test]
fn save_at_ten_drafts_keeps_the_text_and_says_why() {
    let (ada, corvin) = party();
    ada.run("for n = 1, 10 do ns.StoryDrafts.Save('P' .. n .. '-Stormrage', '', 'Text.') end");
    open(&ada, &corvin);
    type_in(&ada, true, "We held it.");

    click(&ada, "Save");

    assert!(is_open(&ada));
    assert_eq!(
        status(&ada),
        "You have 10 drafts. Send or delete one first."
    );
    assert_eq!(box_text(&ada, true), "We held it.");
}

#[test]
fn close_with_ten_drafts_asks_before_it_discards() {
    let (ada, corvin) = party();
    ada.run("for n = 1, 10 do ns.StoryDrafts.Save('P' .. n .. '-Stormrage', '', 'Text.') end");
    open(&ada, &corvin);
    type_in(&ada, true, "We held it.");

    click(&ada, "Close");
    let still_open = is_open(&ada);
    ada.run("wow.AcceptPopup()");

    assert!(still_open);
    assert_eq!(
        ada.eval::<String>("return wow.popups[1].which"),
        "TIMEWAYS_STORY_DISCARD"
    );
    assert!(!is_open(&ada));
}

#[test]
fn a_draft_for_a_player_out_of_the_group_can_be_written_but_not_sent() {
    let (ada, _corvin) = party();
    ada.run("ns.StoryDrafts.Save('Bram-Stormrage', '', 'A draft for Bram.')");

    ada.run("ns.StoryScroll.OpenDraft('Bram-Stormrage')");
    type_in(&ada, true, "A draft for Bram, longer.");

    assert!(is_open(&ada));
    assert_eq!(status(&ada), "Invite Bram to your group to send it.");
    assert!(!send_is_on(&ada));
}

#[test]
fn continue_opens_the_scroll_for_the_player_of_the_draft() {
    let (ada, _corvin) = party();
    ada.run(
        "ns.StoryDrafts.Save('Bram-Stormrage', 'For Bram', 'Text.')
         ns.Journal.Select('stories', 'd:Bram-Stormrage')
         for _, button in ipairs(ns.Journal.Page('stories').buttons) do
             if button.label == 'Continue' then button.run() end
         end",
    );

    assert_eq!(
        ada.eval::<String>("return ns.StoryScroll.Player()"),
        "Bram-Stormrage"
    );
    assert_eq!(box_text(&ada, false), "For Bram");
}

#[test]
fn an_error_keeps_the_text_and_selects_the_bad_sign() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, true, "We é|| held it.");

    click(&ada, "Send");

    assert_eq!(
        status(&ada),
        "Stories can't hold the || sign. Take it out and try again."
    );
    assert_eq!(box_text(&ada, true), "We é|| held it.");
    let selected: Vec<usize> = ada.eval(&format!("return {}.highlighted", box_of(true)));
    assert_eq!(selected, [5, 7]);
}

#[test]
fn the_count_shows_only_when_a_hundred_letters_are_left() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    let count = || -> Vec<String> {
        ada.eval(
            "local out = {}
             for _, widget in ipairs(wow.widgets) do
                 if widget.kind == 'FontString' and widget.parent == TimewaysStoryScroll
                     and widget.text and widget.text:find('left') then
                     table.insert(out, widget.text)
                 end
             end
             return out",
        )
    };

    type_in(&ada, true, &"a".repeat(899));
    let hidden = count();
    type_in(&ada, true, &"a".repeat(913));
    let near = count();
    type_in(&ada, true, &"a".repeat(1000));
    let full = count();

    assert!(hidden.is_empty(), "{hidden:?}");
    assert_eq!(near, ["87 left"]);
    assert_eq!(full, ["No room left"]);
}

#[test]
fn a_target_who_left_the_group_keeps_the_text_with_the_reason() {
    let (ada, corvin) = party();
    open(&ada, &corvin);
    type_in(&ada, true, "We held it.");
    ada.leave_party();

    click(&ada, "Send");

    assert_eq!(
        status(&ada),
        "Corvin left your group. Invite them back to send it."
    );
    assert_eq!(box_text(&ada, true), "We held it.");
}
