//! What the fuzz targets share: a small pack.

use std::path::PathBuf;
use std::sync::OnceLock;
use timeways_story::pack::{Link, Origin, Pack, Passage};

/// One pack for each fuzz process, written once.
pub fn pack() -> Pack {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    let path = PATH.get_or_init(|| {
        let path =
            std::env::temp_dir().join(format!("timeways-fuzz-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let passage = |text: &str, place: &str| Passage {
            text: text.to_string(),
            source: "https://example.test".to_string(),
            links: vec![Link::Place(place.to_string())],
            origin: Origin::Pack,
            about: Some(place.to_string()),
            depends_on: Vec::new(),
            setup_for: None,
        };
        let passages = [
            passage("The tower fell.", "Testvale"),
            passage("The inn is old.", "Mockshire"),
        ];
        Pack::write(&path, &passages).unwrap();
        path
    });
    Pack::open(path).unwrap()
}
