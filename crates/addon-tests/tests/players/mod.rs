//! Two players with Timeways in one test: each one a game of its own, with a hub that
//! carries the addon messages between them (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the harness"
)]

use crate::common::Game;
use timeways_story::aliases::key_of;

pub const REALM: &str = "Stormrage";

/// The event of the normal addon channel.
pub const NORMAL: &str = "CHAT_MSG_ADDON";
/// The event of the logged addon channel, which Blizzard support can read.
pub const LOGGED: &str = "CHAT_MSG_ADDON_LOGGED";

/// An addon message that a game sent. `event` is the event that the receiver gets.
pub struct Sent {
    pub prefix: String,
    pub text: String,
    pub channel: String,
    pub target: Option<String>,
    pub event: String,
}

pub struct Player {
    pub game: Game,
    pub name: &'static str,
}

impl Player {
    /// A player of the realm, logged in, in a guild with the others of the test.
    pub fn new(name: &'static str) -> Player {
        let game = Game::new();
        game.run(&format!(
            "wow.units.player = {{ name = '{name}', player = true, guid = 'Player-1-{name}' }}
             wow.zone = 'Tirisfal Glades'
             wow.subzone = 'Brill'"
        ));
        Player { game, name }
    }

    pub fn full_name(&self) -> String {
        format!("{}-{REALM}", self.name)
    }

    pub fn run(&self, code: &str) {
        self.game.run(code);
    }

    pub fn eval<T: mlua::FromLua>(&self, code: &str) -> T {
        self.game.eval(code)
    }

    /// Makes the other players members of the same guild, online.
    pub fn in_guild_with(&self, others: &[&Player]) {
        let mut members = vec![format!(
            "{{ name = '{}', online = true }}",
            self.full_name()
        )];
        for other in others {
            members.push(format!(
                "{{ name = '{}', online = true }}",
                other.full_name()
            ));
        }
        self.run(&format!(
            "wow.guild = {{ {} }} wow.Fire('GUILD_ROSTER_UPDATE', false)",
            members.join(", ")
        ));
    }

    /// Puts the other player in the party of this one, as `party1`.
    pub fn in_party_with(&self, other: &Player) {
        self.run(&format!(
            "wow.units.party1 = {{ name = '{}', player = true, guid = 'Player-1-{}' }}",
            other.name, other.name
        ));
    }

    pub fn leave_party(&self) {
        self.run("wow.units.party1 = nil");
    }

    /// The addon messages that this game sent since the last delivery.
    pub fn take_sent(&self) -> Vec<Sent> {
        let sent: Vec<mlua::Table> = self.eval("wow.addonSent");
        self.run("wow.addonSent = {}");
        sent.into_iter()
            .map(|message| Sent {
                prefix: message.get("prefix").unwrap(),
                text: message.get("text").unwrap(),
                channel: message.get("channel").unwrap(),
                target: message.get("target").unwrap(),
                event: message.get("event").unwrap(),
            })
            .collect()
    }

    /// Fires `CHAT_MSG_ADDON` as the game does for a message of `sender`.
    pub fn hear(&self, prefix: &str, text: &str, channel: &str, sender: &str) {
        self.hear_on(NORMAL, prefix, text, channel, sender);
    }

    /// Fires `CHAT_MSG_ADDON_LOGGED` as the game does for a logged message of `sender`.
    pub fn hear_logged(&self, prefix: &str, text: &str, channel: &str, sender: &str) {
        self.hear_on(LOGGED, prefix, text, channel, sender);
    }

    /// Fires `event`, the event of one of the two addon channels.
    pub fn hear_on(&self, event: &str, prefix: &str, text: &str, channel: &str, sender: &str) {
        let fire: mlua::Function = self.eval(
            "return function(event, prefix, text, channel, sender)
                 wow.Fire(event, prefix, text, channel, sender, '', 0, 0, '', 0)
             end",
        );
        fire.call::<()>((event, prefix, text, channel, sender))
            .unwrap();
    }

    /// Runs the timers of the game, as a second of play does.
    pub fn tick(&self) {
        self.run("wow.now = wow.now + 1; wow.RunTickers()");
    }

    pub fn printed(&self) -> Vec<String> {
        self.game.printed()
    }
}

fn is_for(target: Option<&str>, player: &Player) -> bool {
    target.is_none_or(|target| target == player.full_name() || target == player.name)
}

/// Carries every message that `from` sent to `to`, on the channel that it went on: a whisper
/// to `to`, and each message to the group or the guild. Returns how many it carried.
pub fn deliver(from: &Player, to: &Player) -> usize {
    let sent = from.take_sent();
    let mut count = 0;
    for message in sent {
        if is_for(message.target.as_deref(), to) {
            to.hear_on(
                &message.event,
                &message.prefix,
                &message.text,
                &message.channel,
                &from.full_name(),
            );
            count += 1;
        }
    }
    count
}

/// Carries messages both ways until none is left, with a second of time between rounds, so
/// a message that waits for the rate limit goes too.
pub fn exchange(a: &Player, b: &Player) {
    for _ in 0..50 {
        let carried = deliver(a, b) + deliver(b, a);
        let waiting: usize = a.eval::<usize>("ns.TaskChannel.Waiting()")
            + b.eval::<usize>("ns.TaskChannel.Waiting()");
        if carried == 0 && waiting == 0 {
            return;
        }
        a.tick();
        b.tick();
    }
    panic!("the two players never stopped talking");
}

/// Ada and Corvin, in one guild, both online.
pub fn ada_and_corvin() -> (Player, Player) {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_guild_with(&[&corvin]);
    corvin.in_guild_with(&[&ada]);
    (ada, corvin)
}

/// The key of a task of Ada in the journal of the doer.
pub fn received_key(id: &str) -> String {
    format!("Ada-Stormrage/{id}")
}

/// Ada gives Corvin a task with these steps, as a Lua list, and Corvin accepts it.
/// Returns the two players and the id of the task.
pub fn accepted_task(steps: &str) -> (Player, Player, String) {
    let (ada, corvin) = ada_and_corvin();
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'Trouble at Agamand Mills', text = 'Put Gregor to rest.',
             reward = '5 gold', steps = {steps} }}, 'Corvin-Stormrage')"
    ));
    exchange(&ada, &corvin);
    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);
    (ada, corvin, id)
}

/// The story program and the addon of a reader read the marks of a peer's story alike: the
/// page shows `TaskNames.PeerView` of its texts, the title first, and the desktop keeps the
/// story that `TaskNames.MarkedStory` made, with the same words, `$N` read back as the
/// reader's name `name`, in any case, as `key_of` folds a name. The addon refuses a story
/// exactly when the desktop counts more than 32 other players in it. A story that breaks the
/// limits of a story is no case. Returns why they differ.
pub fn peer_marks_disagree(reader: &Player, name: &str, texts: &[Vec<u8>]) -> Option<String> {
    let lua = &reader.game.lua;
    // With no limit, the addon also gives the texts of a story that it refuses.
    let read: mlua::Function = reader.eval(
        "return function(texts)
             local refused = ns.TaskNames.MarkedStory(texts) == nil
             local limit = ns.TaskNames.MAX_PEER_MARKS
             ns.TaskNames.MAX_PEER_MARKS = math.huge
             local kept = ns.TaskNames.MarkedStory(texts)
             ns.TaskNames.MAX_PEER_MARKS = limit
             return (ns.TaskNames.PeerView(texts)), kept, refused
         end",
    );
    let lua_texts: Vec<mlua::LuaString> = texts
        .iter()
        .map(|text| lua.create_string(text).unwrap())
        .collect();
    let (shown, kept, refused): (Vec<mlua::LuaString>, Vec<mlua::LuaString>, bool) =
        read.call(lua_texts).unwrap();
    let text = |value: &mlua::LuaString| String::from_utf8_lossy(&value.as_bytes()).into_owned();
    let shown: Vec<String> = shown.iter().map(text).collect();
    let kept: Vec<String> = kept.iter().map(text).collect();
    let plain: Vec<String> = kept
        .iter()
        .map(|text| timeways_story::aliases::unmarked(text).text)
        .collect();
    let title = (!plain[0].is_empty()).then(|| plain[0].as_str());
    if !timeways_story::stories::is_good_story(title, &plain[1..]) {
        return None;
    }
    // The desktop drops no mark that the addon sent.
    for text in &kept {
        let cleaned = timeways_story::aliases::for_matching(text);
        if timeways_story::aliases::unlikely_marks_as_words(&cleaned) != cleaned {
            return Some(format!("the desktop drops a mark of {text:?}"));
        }
    }
    let others = timeways_story::aliases::other_players(&kept, name);
    if refused != (others > 32) {
        return Some(format!(
            "the addon refused: {refused}, with {others} players in {kept:?}"
        ));
    }
    if refused {
        return None;
    }
    let stored = match desktop_story(name, &kept) {
        Ok(stored) => stored,
        Err(error) => return Some(format!("the desktop refused {kept:?}: {error}")),
    };
    let desktop: Vec<String> = stored
        .iter()
        .map(|text| without_realms(&key_of(&text.replace("$N", name))))
        .collect();
    let page: Vec<String> = shown
        .iter()
        .map(|text| without_realms(&key_of(text)))
        .collect();
    (desktop != page)
        .then(|| format!("{page:?} on the page, {desktop:?} on the desktop, from {kept:?}"))
}

/// The story as the desktop of the character `name` keeps it, the title first: an empty
/// title for none.
fn desktop_story(name: &str, kept: &[String]) -> Result<Vec<String>, String> {
    use timeways_story::input::{Input, MessageId};
    use timeways_story::story::Output;
    // A fuzz target has no test folder of cargo.
    let folder = option_env!("CARGO_TARGET_TMPDIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let path = folder.join("peer-marks-pack.sqlite");
    if !path.exists() {
        timeways_story::pack::Pack::write(&path, &[]).unwrap();
    }
    let pack = timeways_story::pack::Pack::open(&path).unwrap();
    let mut story = timeways_story::story::Story::new(pack, timeways_story::store::Store::Memory);
    let entered = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: name.to_string(),
    };
    story.handle(entered).unwrap();
    let accepted = Input::StoryAccepted {
        at: hourglass::Tick(1),
        number: 1,
        title: (!kept[0].is_empty()).then(|| kept[0].clone()),
        paragraphs: kept[1..].to_vec(),
    };
    story.handle(accepted).map_err(|error| error.to_string())?;
    let asked = Input::JournalAsked {
        id: MessageId(1),
        page: 0,
    };
    let Output::Journal { page, .. } = story.handle(asked).unwrap().remove(0) else {
        return Err("no journal".to_string());
    };
    let told = &page.journal.stories[0];
    let mut texts = vec![told.title.clone().unwrap_or_default()];
    texts.extend(told.paragraphs.iter().cloned());
    Ok(texts)
}

/// The text with each "-Realm" after a word taken out. The desktop swaps a known name with
/// its realm, "Lysara-X", for one ID, so the journal shows "Lysara" where the page shows the
/// typed "Lysara-X": a word of the screen, no name.
fn without_realms(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let is_word = timeways_story::aliases::is_word_char;
    while let Some(c) = chars.next() {
        let after_a_word = out.chars().last().is_some_and(is_word);
        if c == '-' && after_a_word && chars.peek().is_some_and(|next| is_word(*next)) {
            while chars.peek().is_some_and(|next| is_word(*next)) {
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}
