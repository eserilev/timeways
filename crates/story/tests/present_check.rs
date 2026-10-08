//! The present check of a narrator history (docs/plans/lore-names-and-now.md 2.3 C), on
//! the answers of the Claude review run of 2026-10-07 and their lore.

use timeways_story::present_check::{
    PresentFault, PresentGrounds, has_present_sentence, unsourced_present_in,
};

fn faults(history: &str, lore: &str, place: &str) -> Vec<PresentFault> {
    faults_after(history, lore, place, &[])
}

fn faults_after(history: &str, lore: &str, place: &str, defeated: &[&str]) -> Vec<PresentFault> {
    let names = vec![place.to_string()];
    let defeated: Vec<String> = defeated.iter().map(ToString::to_string).collect();
    let grounds = PresentGrounds {
        lore,
        names: &names,
        defeated: &defeated,
    };
    unsourced_present_in(history, &grounds)
}

const VANCLEEF_QUOTE: &str = "\"The House of Nobles - no, the entire kingdom of Stormwind will \
    pay for their crimes against the Stonemasons.\"";

const SMITE: &str = "Mr. Smite was an elite tauren who could be found in the Deadmines. Mr. \
    Smite was the first mate under Captain Greenskin.";

const AGAMAGGAN: &str = "Ten thousand years ago - during the War of the Ancients, the mighty \
    demigod, Agamaggan, came forth to battle the Burning Legion. Though the colossal boar fell \
    in combat, his actions helped save Azeroth from ruin. Yet over time, in the areas where his \
    blood fell, massive thorn-ridden vines sprouted from the earth.";

#[test]
fn an_invented_present_is_refused() {
    let invented = [
        (
            "The Stonemasons rebuilt Stormwind, and the House of Nobles cheated them of their \
             pay. The masons swore that the whole kingdom would answer for that crime, and they \
             hold the Deadmines now.",
            VANCLEEF_QUOTE,
            "The Deadmines",
        ),
        (
            "Edwin VanCleef swore that the House of Nobles and all of Stormwind would pay for \
             their crimes against the Stonemasons, and that grievance against the kingdom \
             stands.",
            VANCLEEF_QUOTE,
            "Edwin VanCleef",
        ),
        (
            "Mr. Smite, a tauren, became first mate under Captain Greenskin, and he still holds \
             that post in the Deadmines now.",
            SMITE,
            "The Deadmines",
        ),
    ];

    for (history, lore, place) in invented {
        assert!(
            matches!(
                faults(history, lore, place).as_slice(),
                [PresentFault::Unsourced(_)]
            ),
            "{history}"
        );
    }
}

#[test]
fn a_weak_present_with_only_a_past_lore_is_refused() {
    let weak = [
        "The demigod Agamaggan fell fighting the Burning Legion in the War of the Ancients, and \
         the thorn vines that rose from his blood form Razorfen Kraul now.",
        "Agamaggan, the great boar demigod, fell fighting the Burning Legion in the War of the \
         Ancients ten thousand years ago. Where his blood fell, massive thorned vines sprouted \
         from the earth, and they still rise over Razorfen Kraul.",
    ];

    for history in weak {
        assert_eq!(
            faults(history, AGAMAGGAN, "Razorfen Kraul").len(),
            1,
            "{history}"
        );
    }
}

/// The grounded endings of the run, each with its lore and its place.
const GROUNDED: [(&str, &str, &str); 8] = [
    (
        "Westfall was rich farmland on the border of Stormwind, but it has lain fallow since \
         the Second War. Its own people took it from the Alliance, and the Defias Brotherhood \
         holds it now.",
        "Westfall borders the Kingdom of Stormwind and is mostly populated by humans not under \
         the Alliance\u{2019}s complete control. The region was stolen right under the \
         Alliance\u{2019}s nose by its own bitter people. This rich land has lain fallow since \
         the Second War, but it is now held by the Defias Brotherhood.",
        "Westfall",
    ),
    (
        "The Wailing Caverns form a network of tunnels beneath the heart of the Barrens. Their \
         underground rivers still run for miles under the savannah and rise to the surface as \
         small oases across the Barrens.",
        "The Wailing Caverns are a network of underground caverns within the heart of the \
         Barrens. Its waterways carry underground rivers for miles beneath the savannah, and \
         naturally bubbles to the surface as small oases throughout the Barrens.",
        "Wailing Caverns",
    ),
    (
        "Watery caverns run beneath northwestern Ashenvale and descend to a temple once \
         devoted to the Old Gods. Blackfathom Deeps lies partly underwater now, reached only by \
         a stair-lined shaft and a submerged entrance.",
        "Blackfathom Deeps (also known as: BFD) is a partially underwater dungeon in \
         northwestern Ashenvale. It is accessed by a stair-lined shaft that requires one to \
         swim through an underwater entrance. The deeps are comprised of a series of watery \
         caverns leading deep to a temple devoted to the Old Gods.",
        "Blackfathom Deeps",
    ),
    (
        "Stormwind built the Stockade beneath the canal district of its capital to hold \
         crooks, insurgents, and murderers. Warden Thelwater presides over it now, and the \
         most dangerous criminals in the land are held there.",
        "Stormwind Stockade, aka the Stormwind Stockades and The Stockade, is a high-security \
         prison complex, hidden beneath the canal district of Stormwind City. Presided over by \
         Warden Thelwater, the Stockade is home to petty crooks, political insurgents, \
         murderers, and a score of the most dangerous criminals in the land. The stockade \
         allowed visitors for the prisoners.",
        "The Stockade",
    ),
    (
        "The Old Monastery in the northeast of Tirisfal Glades was once a cathedral of the \
         Church of the Holy Light. After the Third War, the Scarlet Crusade took it and holds \
         its fortified halls now.",
        "The Scarlet Monastery, once known as The Old Monastery, is a fortified monastery \
         located in the northeast corner of the blighted Tirisfal Glades, it was once a \
         cathedral of the Church of the Holy Light, which was taken over by the Scarlet \
         Crusade after the end of the Third War.",
        "Scarlet Monastery",
    ),
    (
        "The titan keepers built Uldaman deep beneath the mountains of Khaz Modan, and \
         Ironforge dwarves partly dug it out before the Third War. Troggs and Dark Iron \
         dwarves hold its halls now and dig there for riches.",
        "Uldaman is a huge and ancient, massive vault created by the titan keepers, buried \
         deep within mountains of Khaz Modan. It is located in the Badlands, west of what was \
         once the entrance to Loch Modan. Prior to the Third War, it was partially excavated \
         by the dwarves from Ironforge, but has since fallen into the hands of the troggs and \
         Dark Irons, who find the place ideal to dig for earthly riches.",
        "Uldaman",
    ),
    (
        "Foreman Thistlenettle led a crew of the Miners' League in the Deadmines until the \
         Defias Brotherhood struck and brought the tunnel down. Wilder alone escaped, and his \
         slain comrades wander the shafts as restless undead now.",
        "For a time, the Miners' League was partly in charge of the Deadmines under the \
         direction of Foreman Thistlenettle and Wilder Thistlenettle. However, one day the \
         Defias Brotherhood attacked them during their work and the mine tunnel collapsed. \
         Only Wilder managed to escape, while the rest of the company died and began to roam \
         as restless undead.",
        "The Deadmines",
    ),
    (
        "VanCleef led the Stonemasons out of Stormwind and ran the farmers of Westfall off to \
         seize its gold mines, and his outlaw stonemasons still hold the Deadmines.",
        "After he led the remnants of the Stonemasons out of Stormwind, he took advantage of \
         the relatively unprotected state of Westfall, and used his considerable manpower to \
         run many of the farmers off, and take over the handful of gold mines. Taking \
         advantage of the resources at his disposal, VanCleef hatched a plan of retribution \
         against the government of Stormwind.",
        "The Deadmines",
    ),
];

#[test]
fn a_present_from_the_lore_passes() {
    for (history, lore, place) in GROUNDED {
        assert_eq!(faults(history, lore, place), [], "{history}");
    }
}

#[test]
fn a_defeated_foe_is_never_alive_in_a_line() {
    let lore = "Edwin VanCleef leads the Defias Brotherhood from the Deadmines.";
    let history = "The Stonemasons were never paid, and VanCleef still holds the Deadmines.";

    let before = faults_after(history, lore, "The Deadmines", &[]);
    let after = faults_after(history, lore, "The Deadmines", &["Edwin VanCleef"]);

    assert_eq!(before, []);
    assert_eq!(
        after,
        [PresentFault::Defeated(
            "and VanCleef still holds the Deadmines.".to_string(),
            "Edwin VanCleef".to_string()
        )]
    );
}

#[test]
fn a_past_history_needs_no_source() {
    let history = "Mathias Shaw was a childhood friend of Edwin, and he trained Edwin as a rogue.";

    assert_eq!(faults(history, SMITE, "The Deadmines"), []);
}

#[test]
fn a_sentence_inside_quote_marks_is_no_source() {
    let lore = "\"The Brotherhood holds the Deadmines.\"";

    let found = faults("The Brotherhood holds the Deadmines now.", lore, "Westfall");

    assert_eq!(found.len(), 1);
}

#[test]
fn still_before_a_past_verb_is_no_present() {
    assert!(!has_present_sentence(
        "Years after that war, Landen Stilwell still remained within its walls."
    ));
    assert!(has_present_sentence("The rest still roam there, undead."));
}

#[test]
fn a_plural_after_a_name_is_no_verb() {
    assert!(!has_present_sentence(
        "The Gurubashi trolls once ruled all of Stranglethorn."
    ));
    assert!(has_present_sentence("Mutanus appears to disrupt them."));
}
