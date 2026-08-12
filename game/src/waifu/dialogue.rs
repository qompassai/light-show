//! Data-driven dialogue bank. Lines live in
//! `assets/dialogue/seraphine_en.json` so localization/tone edits never
//! require a recompile. This module just defines the shape and a safe
//! fallback so missing keys never crash the game.

use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone, Deserialize)]
pub struct DialogueBank {
    pub lines: HashMap<String, Vec<String>>,
}

impl DialogueBank {
    /// Compiled-in fallback bank (also mirrored to
    /// `assets/dialogue/seraphine_en.json` for easy editing/localization).
    /// All lines reviewed for PEGI-12 / Google Play "Teen": flirtation and
    /// light teasing only, no explicit content.
    pub fn load_default() -> Self {
        let mut lines = HashMap::new();
        lines.insert(
            "clean_splice".into(),
            vec![
                "Ara ara~ a 0.05 dB fusion splice? Be still my heart, senpai.".into(),
                "Mmm, that's a clean core alignment. I could watch you splice all day.".into(),
                "Look at you, prepping your cleaver like you mean it. Cute.".into(),
            ],
        );
        lines.insert(
            "messy_splice".into(),
            vec![
                "That mechanical splice loss made me flinch. Physically. I have no body.".into(),
                "Bold of you to skip re-cleaving. I'm not mad, just... concerned.".into(),
            ],
        );
        lines.insert(
            "level_win".into(),
            vec![
                "In-window, first try! You're making it very hard to focus on my job here.".into(),
                "-15.2 dBm, right in the pocket. Show-off.".into(),
            ],
        );
        lines.insert(
            "level_fail_hot".into(),
            vec!["Whoa — too hot, you're gonna cook that receiver. Add some loss, hotshot.".into()],
        );
        lines.insert(
            "level_fail_cold".into(),
            vec!["Signal's underwater. Either shorten the run or drop a splitter tier.".into()],
        );
        lines.insert(
            "outage_start".into(),
            vec![
                "Uh oh — got a fault on the line. Chop chop, I do NOT do well with dead air."
                    .into(),
            ],
        );
        lines.insert(
            "outage_resolved".into(),
            vec![
                "Service restored! You rerouted that faster than I could finish my coffee. I don't drink coffee. I don't know why I said that.".into(),
            ],
        );
        lines.insert(
            "hint_request".into(),
            vec!["Aww, need a hint? Fine — for you, anything. *wink*".into()],
        );
        Self { lines }
    }

    pub fn random_line(&self, key: &str) -> Option<&str> {
        self.lines
            .get(key)
            .and_then(|options| options.first())
            .map(|s| s.as_str())
    }
}
