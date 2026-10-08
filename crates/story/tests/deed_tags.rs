//! The tags of the review of 2026-10-07 (docs/plans/lore-names-and-now.md 3): each wrong
//! tag, each missed tag, and each class of fault, with the sentence of the wiki that showed
//! it. The pages around each sentence are invented.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::collections::BTreeMap;
use timeways_story::character::Character;
use timeways_story::input::GameQuestKind;
use timeways_story::name_match::Words;
use timeways_story::outcome_passages::{
    Chain, Npc, PageKind, Paragraph, Quest, Stance, Title, dependencies,
};
use timeways_story::pack::{Deed, Dependency, Link, Origin, Passage, SetupFor};
use timeways_story::setup_passages::{Found, Instances, setup_for, setup_sentences, window};
use timeways_story::spoiler::may_show;
use timeways_story::wikitext::{Cites, cites};

fn npc(name: &str, stance: Stance) -> PageKind {
    PageKind::Npc(Npc {
        name: name.to_string(),
        stance,
    })
}

fn quest(name: &str, chain: Chain, title: Title) -> PageKind {
    PageKind::Quest(Quest {
        name: name.to_string(),
        chain,
        title,
    })
}

/// The pages of the wiki that the sentences link to or cite.
fn kind_of(title: &str) -> Option<PageKind> {
    match title {
        "Edwin VanCleef" => Some(npc("Edwin VanCleef", Stance::Foe)),
        "Dagran Thaurissan" => Some(npc("Emperor Dagran Thaurissan", Stance::Foe)),
        "Moira Thaurissan" => Some(npc("Moira Thaurissan", Stance::Foe)),
        "Baron Rivendare" => Some(npc("Baron Titus Rivendare", Stance::Foe)),
        "Balnazzar" => Some(npc("Balnazzar", Stance::Foe)),
        "Hakkar" => Some(npc("Avatar of Hakkar", Stance::Foe)),
        "Targorr the Dread" => Some(npc("Targorr the Dread", Stance::Foe)),
        "Dextren Ward" => Some(npc("Dextren Ward", Stance::Foe)),
        "Raze Direhorn Post" => Some(quest("Raze Direhorn Post", Chain::End, Title::Own)),
        "Confirming the Suspicion" => {
            Some(quest("Confirming the Suspicion", Chain::End, Title::Own))
        }
        "What Comes Around..." => Some(quest("What Comes Around...", Chain::End, Title::Own)),
        "The Ancient Egg" => Some(quest("The Ancient Egg", Chain::Continues, Title::Own)),
        "Into The Temple of Atal'Hakkar" => Some(quest(
            "Into The Temple of Atal'Hakkar",
            Chain::End,
            Title::Own,
        )),
        "Pool of Tears" => Some(quest("Pool of Tears", Chain::Continues, Title::Own)),
        _ => None,
    }
}

/// The tags of one line of wikitext on `page`, with the known bosses and the foes of the
/// page in the list.
fn tags(line: &str, page: &str, bosses: &[&str], foes: &[&str]) -> Vec<Dependency> {
    let cited = cites(line);
    let text = timeways_story::wikitext::plain(line);
    let bosses: Vec<String> = bosses.iter().map(ToString::to_string).collect();
    let foes: Vec<String> = foes.iter().map(ToString::to_string).collect();
    let paragraph = Paragraph {
        text: &text,
        cites: &cited,
        page,
        bosses: &bosses,
        foes: &foes,
    };
    dependencies(&paragraph, kind_of)
}

fn foe(name: &str) -> Dependency {
    Dependency::Foe(name.to_string())
}

#[test]
fn a_hostage_after_and_retrieve_is_no_foe() {
    let text = "King Magni, upset that Moira was with an arch-enemy of his family, sent a team \
                to kill Emperor Thaurissan and retrieve the presumably ensorcelled Moira back \
                to Ironforge.";
    let found = Found {
        text,
        cites: &Cites::default(),
        page: "Moira Thaurissan",
        links: &[Link::Place("Blackrock Depths".to_string())],
    };
    let instances = Instances {
        names: vec!["Blackrock Depths".to_string()],
        bosses: BTreeMap::from([(
            "Moira Thaurissan".to_string(),
            vec!["Blackrock Depths".to_string()],
        )]),
    };

    assert_eq!(setup_for(&found, &instances, kind_of), None);
}

#[test]
fn the_same_sentence_on_the_page_of_the_emperor_sets_up_his_defeat() {
    let text = "King Magni Bronzebeard sent a team to kill Emperor Thaurissan, and retrieve the \
                previously thought to be ensorcelled Moira back to Ironforge.";
    let found = Found {
        text,
        cites: &Cites::default(),
        page: "Dagran Thaurissan",
        links: &[Link::Place("Blackrock Depths".to_string())],
    };
    let instances = Instances {
        names: vec!["Blackrock Depths".to_string()],
        bosses: BTreeMap::from([(
            "Dagran Thaurissan".to_string(),
            vec!["Blackrock Depths".to_string()],
        )]),
    };

    let setup = setup_for(&found, &instances, kind_of).unwrap();

    assert_eq!(
        setup.deed,
        Deed::Foe("Emperor Dagran Thaurissan".to_string())
    );
}

#[test]
fn the_start_of_a_name_skips_a_title_of_another_name() {
    let words = Words::of("They sent a team to kill Emperor Thaurissan and free Moira Thaurissan.");

    assert_eq!(words.name_at(0..words.len(), "Moira Thaurissan"), Some(10));
    assert_eq!(
        words.name_at(0..words.len(), "Emperor Dagran Thaurissan"),
        Some(6)
    );
}

#[test]
fn another_given_name_before_a_surname_names_someone_else() {
    let words = Words::of("Following Alexandros Mograine's death, some paladins left.");

    assert!(!words.names(0..words.len(), "Renault Mograine"));
}

#[test]
fn a_wish_of_a_foe_toward_his_own_people_is_no_hook() {
    let text = "The Emperor kidnapped Princess Moira, as Thaurissan wanted to free his people \
                from the Firelord.";

    assert!(setup_sentences(text).is_empty());
}

#[test]
fn a_commission_takes_the_quest_of_its_own_sentence() {
    let line = "Thrall's own people had seen the pool.<ref>[[Pool of Tears]]</ref> Warchief \
                Thrall sent a group of orcs to investigate the strange happenings at the \
                Temple.<ref>[[Into The Temple of Atal'Hakkar]]</ref>";
    let text = timeways_story::wikitext::plain(line);
    let cited = cites(line);
    let found = Found {
        text: &text,
        cites: &cited,
        page: "Temple of Atal'Hakkar",
        links: &[Link::Place("The Temple of Atal'Hakkar".to_string())],
    };
    let instances = Instances {
        names: vec!["The Temple of Atal'Hakkar".to_string()],
        bosses: BTreeMap::new(),
    };

    let setup = setup_for(&found, &instances, kind_of).unwrap();

    assert_eq!(
        setup.deed,
        Deed::Quest("Into The Temple of Atal'Hakkar".to_string())
    );
}

#[test]
fn leaders_who_succumbed_end_the_setup_before_them() {
    let text = "It is known that Varimathras was determined to eliminate Sally Whitemane. \
                Although in time the other leaders succumbed, Sally survived.";

    let shown = window(text, &Deed::Foe("Sally Whitemane".to_string()));

    assert!(shown.is_some_and(|shown| !shown.contains("succumbed")));
}

#[test]
fn a_deed_takes_the_references_of_its_own_sentence() {
    let line = "The Grimtotem killed the family of Hyal.<ref>[[Confirming the Suspicion]]</ref> \
                An [[adventurer]] later discovered this treachery and brought the Grimtotem to \
                justice.<ref>[[Raze Direhorn Post]]</ref>";

    assert_eq!(
        tags(line, "Theramore Isle", &[], &[]),
        [Dependency::Quest("Raze Direhorn Post".to_string())]
    );
}

#[test]
fn a_deed_with_no_foe_in_its_sentence_never_takes_a_foe_of_another_sentence() {
    let line = "The city was also the site of the Ancient Egg, a relic of [[Hakkar]] that was \
                secured by hapless adventurers.<ref>[[The Ancient Egg]]</ref> The altar was \
                used to summon the demigod.";

    assert_eq!(
        tags(line, "Jintha'Alor", &[], &[]),
        [Dependency::Unresolved]
    );
}

#[test]
fn a_plot_that_an_adventurer_foiled_is_unresolved_when_its_quest_continues() {
    let line = "The Buccaneers also planned an attack on Booty Bay, but an adventurer \
                discovered their plot and warned the leadership of Booty Bay.\
                <ref>[[The Ancient Egg]]</ref>";

    assert_eq!(tags(line, "Booty Bay", &[], &[]), [Dependency::Unresolved]);
}

#[test]
fn a_clue_is_no_outcome() {
    let line = "Some time later, Horde adventurers in Blackrock Spire discovered Blackrock \
                documents signed by Warchief Rend.";

    assert!(tags(line, "Blackrock clan", &[], &[]).is_empty());
}

#[test]
fn a_passage_with_two_deeds_gets_both_tags() {
    let line = "Targorr was held in the Stockade, until he was killed by an adventurer sent by \
                Guard Berton.<ref>[[What Comes Around...]]</ref> The nobles stopped the \
                execution of the grave robber [[Dextren Ward]].";

    assert_eq!(
        tags(line, "Stormwind Stockade", &[], &[]),
        [
            Dependency::Quest("What Comes Around...".to_string()),
            foe("Dextren Ward")
        ]
    );
}

#[test]
fn champions_who_accomplished_a_task_asked_before_depend_on_its_foe() {
    let line = "Adventurers sent the Head of [[Balnazzar]] to the Duke. The Duke later ordered \
                adventurers to slay [[Baron Rivendare]]. The champions who accomplished such a \
                task were rewarded.";

    assert_eq!(
        tags(line, "Argent Dawn", &[], &[]),
        [foe("Balnazzar"), foe("Baron Titus Rivendare")]
    );
}

#[test]
fn heroes_who_succeeded_after_a_call_to_destroy_edwin_depend_on_vancleef() {
    let line = "Stoutmantle called upon the heroes of the Alliance to destroy the Brotherhood \
                and Edwin at their head. With support from Stoutmantle's soldiers, the heroes \
                succeeded and Westfall was safe once again.";

    assert_eq!(
        tags(line, "Westfall", &["Edwin VanCleef"], &[]),
        [foe("Edwin VanCleef")]
    );
}

#[test]
fn a_deed_of_a_group_with_no_foe_is_the_defeat_of_the_first_foe_of_its_page() {
    let line = "However, they were defeated by adventurers sent by Gryan Stoutmantle before \
                they could accomplish their goals.";

    assert_eq!(
        tags(line, "Defias Brotherhood", &[], &["Edwin VanCleef"]),
        [foe("Edwin VanCleef")]
    );
}

#[test]
fn adventurers_who_killed_the_guard_of_a_crypt_depend_on_him() {
    let line = "The sword was sealed away in the crypt and protected by Galen Trollbane until \
                adventurers of the Horde killed him.";

    assert_eq!(
        tags(line, "Stromgarde Keep", &[], &["Galen Trollbane"]),
        [foe("Galen Trollbane")]
    );
}

#[test]
fn the_mysterious_death_of_the_commander_waits_for_his_kill() {
    let line = "However, after Commander Mograine's mysterious death, he was eventually \
                replaced by High Commander Goodchilde.";

    assert_eq!(
        tags(line, "Scarlet Crusade", &[], &["Renault Mograine"]),
        [foe("Renault Mograine")]
    );
}

#[test]
fn a_shared_surname_alone_names_no_boss() {
    let line = "Ever since the Dark Iron dwarves' capital, Thaurissan, was destroyed by the \
                summoning of Ragnaros, they have searched for a landmass.";

    assert!(tags(line, "Searing Gorge", &["Dagran Thaurissan"], &[]).is_empty());
}

fn two_deeds() -> Passage {
    Passage {
        text: "Targorr was killed by an adventurer, and Dextren Ward was executed.".to_string(),
        source: "the wiki page \"Stormwind Stockade\"".to_string(),
        links: vec![Link::Place("The Stockade".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: vec![
            Dependency::Quest("What Comes Around...".to_string()),
            foe("Dextren Ward"),
        ],
        setup_for: None::<SetupFor>,
    }
}

#[test]
fn a_passage_with_two_deeds_shows_only_after_both() {
    let mut character = Character::new();
    character.enter_zone(Tick(1), "The Stockade", None).unwrap();
    character
        .finish_game_quest(Tick(2), "What Comes Around...", GameQuestKind::Normal)
        .unwrap();
    let after_one = may_show(&character, &two_deeds());

    character.defeat_npc(Tick(3), "Dextren Ward").unwrap();

    assert!(!after_one);
    assert!(may_show(&character, &two_deeds()));
}

#[test]
fn a_boss_beheaded_by_another_on_his_own_page_waits_for_his_kill() {
    let line = "As promised, Renault became the Scarlet Commander of the Scarlet Monastery, \
                however, he was later beheaded by Alexandros' vengeful spirit.";

    assert_eq!(
        tags(line, "Dagran Thaurissan", &[], &[]),
        [foe("Emperor Dagran Thaurissan")]
    );
    assert!(tags(line, "Westfall", &[], &[]).is_empty());
}
