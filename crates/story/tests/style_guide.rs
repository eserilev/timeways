//! The user's verdicts of the style guide (docs/plans/narrator-style.md 7) against the
//! checks of a narrator line. A loved line passes every check, and a hated line fails one.

use timeways_story::arrival::arrival_in;
use timeways_story::check::{banned_words_in, slop_in};
use timeways_story::line_check::{Checked, Grounds, checked_line};
use timeways_story::narrator::Naming;
use timeways_story::prose::prose_faults;

/// The lines that the user loved, and the "do" lines of the new pairs (7.2).
const LOVED: [&str; 26] = [
    "The Miners' League once worked the Deadmines under Foreman Thistlenettle. The Defias \
     attacked, the tunnel collapsed, and only Wilder escaped. The rest still roam there, undead.",
    "The Royal Apothecary Society was founded to cure the living. Beneath the ruins of \
     Lordaeron, it brews a new plague for them instead.",
    "Arugal called the worgen into Silverpine to fight the Scourge, but he could not control \
     them. Shadowfang Keep has no master now.",
    "Westfall's rich fields have lain fallow since the Second War. Its own bitter people took \
     it from Stormwind, and the Defias Brotherhood holds it now.",
    "Northshire's vineyards were once Stormwind's pride. Defias bandits hold the fields now, \
     and kobolds dig in Echo Ridge Mine.",
    "The Silver Hand once burned the dead of Lordaeron. Now the dead channel the Light, and \
     their power grows. $N has reached level 20.",
    "Lakeshire has asked Stormwind for soldiers more than once. None have come, and the \
     Blackrock orcs still hold Stonewatch Keep.",
    "Troggs rose from beneath Gnomeregan. Mekgineer Thermaplugg flooded the city with \
     radiation to stop them, and most of the gnomes died. The rest live in Ironforge now.",
    "The Scarlet Crusade swore to burn the Scourge out of Lordaeron. Now it kills anyone it \
     suspects of the plague, living or dead.",
    "Arugal cursed the people of Pyrewood. By day they tend their village, and by night they \
     are worgen.",
    "Edwin VanCleef built the Defias Brotherhood from the stonemasons Stormwind never paid. The \
     Brotherhood has lost its founder.",
    "Hogger led the Riverpaw gnolls against the farms of Elwynn for years. The Riverpaw have no \
     leader now.",
    "Naralex went into the Wailing Caverns to make the Barrens green again. The Emerald \
     Nightmare took his mind, and his own Druids of the Fang serve it now.",
    "Thrall grew up a slave in Durnholde Keep, where his keepers trained him to fight. The keep \
     is a ruin now, and the Syndicate holds it.",
    "The War of the Three Hammers left the Bronzebeards on the throne of Ironforge. The Dark \
     Irons fled south, and the Wildhammers went to the Hinterlands.",
    "The Blackrock orcs raid the farms of Redridge from Stonewatch Keep. Lakeshire holds its \
     bridge with its own guards.",
    "Grom Hellscream killed the demigod Cenarius in Ashenvale. The Warsong Clan still cuts its \
     trees for Orgrimmar, and the night elves fight them for every grove.",
    "The Shadow Council taught the first orcs to bargain with demons. The warlocks of the Horde \
     grow stronger. $N has reached level 20.",
    "Duskwood was part of Elwynn Forest until dark magic from Karazhan turned its trees. The \
     Night Watch of Darkshire holds the town against the dead.",
    "The Burning Blade cult hides in Ragefire Chasm, beneath Orgrimmar itself.",
    "The prisoners of the Stockade rose up and took it from their guards. Stormwind holds the \
     gate, and the riot goes on inside.",
    "The Gurubashi trolls once ruled all of Stranglethorn. Their empire broke apart, and its \
     tribes fight among its ruins now.",
    "The Crossroads stands where the roads of the Barrens meet. Orc grunts hold its walls \
     against the centaur, and the caravans of the Horde stop there.",
    "The quilboar believe the demigod Agamaggan died in the Barrens. The great thorns of \
     Razorfen grew from his blood, and they guard them as holy ground.",
    "Blackfathom Deeps was a temple to Elune before the sea took it. The Twilight's Hammer \
     worships Aku'mai there now.",
    "The night elves planted Teldrassil to win back their immortality. The dragons never \
     blessed the tree, and the Gnarlpine furbolgs of the island have turned corrupt.",
];

/// The lines that the user hated (7.1), and the "don't" lines of the new pairs (7.2) that a
/// pattern can catch, with the race and the class of their hero.
const HATED: [(&str, &[&str]); 27] = [
    (
        "Level six came on a grey morning. Our hero moved on without rest.",
        &[],
    ),
    (
        "Level ten came on a muddy road at dusk. Our hero did not stop walking.",
        &[],
    ),
    ("One more stranger came into those fields.", &[]),
    ("A stranger to these plains walked among them.", &[]),
    (
        "At level 1, Kobee was a corpse in Deathknell. Twenty levels later, the Scarlet Crusade \
         knows the name.",
        &["forsaken"],
    ),
    (
        "Level 10. The warlocks of the Undercity will now teach $N to bind a voidwalker.",
        &["forsaken", "warlock"],
    ),
    ("Its power grows in the druid.", &["tauren", "druid"]),
    ("That old craft grows sharper in $N.", &[]),
    ("$N ended him.", &[]),
    (
        "Apothecary Renferrel spoke of the Royal Apothecary Society's plague. In the Undercity, \
         its masters keep their vats below the throne.",
        &[],
    ),
    (
        "Arugal lost them to their hunger. He called them his children to the end.",
        &[],
    ),
    ("The worg would not try it now.", &[]),
    (
        "Whispers of war echo through Redridge as the Blackrock shadow looms.",
        &[],
    ),
    (
        "The Scarlet Crusade, zealous and proud, stands as a bastion against the dark.",
        &[],
    ),
    ("$N ended VanCleef.", &[]),
    (
        "The Riverpaw will remember the name of the paladin.",
        &["paladin"],
    ),
    (
        "Naralex's dream of a green Barrens lives on in the hero.",
        &[],
    ),
    (
        "As the tome 'The War of the Three Hammers' tells it, the dwarves fought bitterly.",
        &[],
    ),
    (
        "Magistrate Solomon spoke of the Blackrock threat. In Redridge, its orcs raid the farms.",
        &[],
    ),
    (
        "Ashenvale mourns, its ancient trees weeping for the fallen demigod.",
        &[],
    ),
    (
        "Level 20. The Forsaken warlock's power is a testament to the dark arts.",
        &[],
    ),
    ("Duskwood: a forest of shadow, sorrow, and secrets.", &[]),
    (
        "Not just a prison, the Stockade is a symbol of Stormwind's fall.",
        &[],
    ),
    (
        "One more stranger arrived at the Crossroads, where caravans gather.",
        &[],
    ),
    (
        "Teldrassil's majestic boughs cradle the night elves in eternal starlight.",
        &[],
    ),
    (
        "Level 30. $N has killed 412 foes and finished 87 quests.",
        &[],
    ),
    (
        "The dwarves of Ironforge greet $N as one of their own.",
        &["dwarf"],
    ),
];

/// "Don't" lines of 7.2 that no pattern catches. The prompt, the samples, and the review
/// of the user carry them (10.2): each is a figure of speech that the ban list does not
/// name, or a judgment of a people.
const ONLY_THE_GUIDE_CATCHES: [&str; 7] = [
    "Gnomeregan, once a marvel, is now a tomb of broken dreams.",
    "Pyrewood hides a terrible secret beneath its quiet streets.",
    "Thrall's ghost still haunts the ruined halls of Durnholde, where chains once bound a \
     people.",
    "The Burning Blade lurks beneath Orgrimmar, a dark heart within the Horde.",
    "Stranglethorn's jungle hungers, its ruins whispering of glories long past.",
    "The quilboar of Razorfen are savage and cruel, a blight upon the land.",
    "The dead Twilight's Hammer cultists hunger for a god they cannot reach.",
];

/// The names of the loved lines that hold a slop word. The lore of their moment holds
/// the name, and a slop word that the lore holds stays allowed.
const NAMES_OF_THE_LORE: &str = "the Shadow Council";

/// Every race and class word of a hero, so a loved line passes for any hero.
const EVERY_KIND: [&str; 17] = [
    "human", "dwarf", "gnome", "elf", "orc", "troll", "tauren", "forsaken", "warrior", "paladin",
    "hunter", "rogue", "priest", "shaman", "mage", "warlock", "druid",
];

fn kinds(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_string()).collect()
}

/// Grounds whose lore is the line itself, so the line is grounded, copies nothing, and adds
/// no number. Slop is checked alone, with nothing told.
fn grounds_of(line: &str, hero_words: Vec<String>) -> Grounds {
    Grounds {
        moment: "The player reached level 20.".to_string(),
        names: Vec::new(),
        lore: Some(line.to_string()),
        naming: Naming::Name,
        hero_words,
        outside: Vec::new(),
    }
}

fn fails_a_check(line: &str, hero_words: &[&str]) -> bool {
    let hero_words = kinds(hero_words);
    let refused = !matches!(
        checked_line(line, &grounds_of(line, hero_words.clone()), ""),
        Checked::Line(_)
    );
    refused
        || !slop_in(line, "").is_empty()
        || !banned_words_in(line).is_empty()
        || arrival_in(line, &hero_words).is_some()
        || !prose_faults(line, &hero_words).is_empty()
}

#[test]
fn every_loved_line_passes_every_check() {
    for line in LOVED {
        let every_kind = kinds(&EVERY_KIND);

        assert_eq!(
            checked_line(line, &grounds_of(line, every_kind.clone()), ""),
            Checked::Line(line.to_string())
        );
        assert_eq!(
            slop_in(line, NAMES_OF_THE_LORE),
            Vec::<&str>::new(),
            "{line}"
        );
        assert_eq!(prose_faults(line, &every_kind), [], "{line}");
    }
}

#[test]
fn every_hated_line_fails_a_check() {
    for (line, hero_words) in HATED {
        assert!(fails_a_check(line, hero_words), "{line}");
    }
}

#[test]
fn the_lines_that_only_the_guide_catches_pass_the_checks() {
    for line in ONLY_THE_GUIDE_CATCHES {
        assert!(
            !fails_a_check(line, &[]),
            "a pattern catches it now: {line}"
        );
    }
}
