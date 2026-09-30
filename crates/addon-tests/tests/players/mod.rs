//! Two players with Timeways in one test: each one a game of its own, with a hub that
//! carries the addon messages between them (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the harness"
)]

use crate::common::Game;

pub const REALM: &str = "Stormrage";

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
        self.run(&format!("wow.guild = {{ {} }}", members.join(", ")));
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

    /// The addon messages that this game sent since the last delivery, as
    /// (prefix, text, channel, target).
    pub fn take_sent(&self) -> Vec<(String, String, String, Option<String>)> {
        let sent: Vec<mlua::Table> = self.eval("wow.addonSent");
        self.run("wow.addonSent = {}");
        sent.into_iter()
            .map(|message| {
                (
                    message.get("prefix").unwrap(),
                    message.get("text").unwrap(),
                    message.get("channel").unwrap(),
                    message.get("target").unwrap(),
                )
            })
            .collect()
    }

    /// Fires `CHAT_MSG_ADDON` as the game does for a message of `sender`.
    pub fn hear(&self, prefix: &str, text: &str, channel: &str, sender: &str) {
        let fire: mlua::Function = self.eval(
            "return function(prefix, text, channel, sender)
                 wow.Fire('CHAT_MSG_ADDON', prefix, text, channel, sender, '', 0, 0, '', 0)
             end",
        );
        fire.call::<()>((prefix, text, channel, sender)).unwrap();
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

/// Carries every message that `from` sent to `to`: a whisper to `to`, and each message to the
/// group or the guild. Returns how many it carried.
pub fn deliver(from: &Player, to: &Player) -> usize {
    let sent = from.take_sent();
    let mut count = 0;
    for (prefix, text, channel, target) in sent {
        if is_for(target.as_deref(), to) {
            to.hear(&prefix, &text, &channel, &from.full_name());
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
