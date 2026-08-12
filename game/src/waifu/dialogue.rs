//! Data-driven dialogue banks, one per `Companion`. Compiled-in banks live
//! here (also mirrored to `assets/dialogue/<companion>_en.json` for
//! easy editing/localization). This module just defines the shape and a
//! safe fallback so missing keys never crash the game.
//!
//! All lines across all four companions are reviewed for PEGI-12 /
//! Google Play "Teen": flirtation and light teasing only, no explicit
//! content. Each companion's bank mirrors the same event keys
//! (`clean_splice`, `messy_splice`, `level_win`, `level_fail_hot`,
//! `level_fail_cold`, `outage_start`, `outage_resolved`, `hint_request`)
//! flavored for their transmission medium — see `docs/ART_STYLE.md` for
//! each character's brief.

use super::Companion;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone, Deserialize)]
pub struct DialogueBank {
    pub lines: HashMap<String, Vec<String>>,
}

impl DialogueBank {
    /// Compiled-in fallback bank for the given companion.
    pub fn load_default(companion: Companion) -> Self {
        match companion {
            Companion::Fiber => Self::seraphine(),
            Companion::Coax => Self::ondine(),
            Companion::Mobile => Self::linka(),
            Companion::Ethernet => Self::lattice(),
        }
    }

    fn from_pairs(pairs: &[(&str, &[&str])]) -> Self {
        let mut lines = HashMap::new();
        for (key, options) in pairs {
            lines.insert(
                key.to_string(),
                options.iter().map(|s| s.to_string()).collect(),
            );
        }
        Self { lines }
    }

    /// Séraphine — fiber-optic splicing.
    fn seraphine() -> Self {
        Self::from_pairs(&[
            (
                "clean_splice",
                &[
                    "Ara ara~ a 0.05 dB fusion splice? Be still my heart, senpai.",
                    "Mmm, that's a clean core alignment. I could watch you splice all day.",
                    "Look at you, prepping your cleaver like you mean it. Cute.",
                ],
            ),
            (
                "messy_splice",
                &[
                    "That mechanical splice loss made me flinch. Physically. I have no body.",
                    "Bold of you to skip re-cleaving. I'm not mad, just... concerned.",
                ],
            ),
            (
                "level_win",
                &[
                    "In-window, first try! You're making it very hard to focus on my job here.",
                    "-15.2 dBm, right in the pocket. Show-off.",
                ],
            ),
            (
                "level_fail_hot",
                &["Whoa — too hot, you're gonna cook that receiver. Add some loss, hotshot."],
            ),
            (
                "level_fail_cold",
                &["Signal's underwater. Either shorten the run or drop a splitter tier."],
            ),
            (
                "outage_start",
                &["Uh oh — got a fault on the line. Chop chop, I do NOT do well with dead air."],
            ),
            (
                "outage_resolved",
                &["Service restored! You rerouted that faster than I could finish my coffee. I don't drink coffee. I don't know why I said that."],
            ),
            (
                "hint_request",
                &["Aww, need a hint? Fine — for you, anything. *wink*"],
            ),
        ])
    }

    /// Ondine — coax / broadband RF.
    fn ondine() -> Self {
        Self::from_pairs(&[
            (
                "clean_splice",
                &[
                    "Ooh, an F-connector torqued to spec? I felt that in my signal-to-noise ratio.",
                    "Mmm, zero return loss. You really know how to sweep a line, don't you.",
                    "Look at that tap value — precise. I like precise.",
                ],
            ),
            (
                "messy_splice",
                &[
                    "That connector's loose enough to cause ingress. My tuner is judging you.",
                    "Bold to skip the torque wrench. I'm not upset, just... static-y.",
                ],
            ),
            (
                "level_win",
                &[
                    "Locked in-band on the first pass? Careful, you're making my downstream blush.",
                    "Clean sweep, zero ingress. Show-off.",
                ],
            ),
            (
                "level_fail_hot",
                &["Whoa, that's way too much signal — you're gonna clip the amp. Pad it down, hotshot."],
            ),
            (
                "level_fail_cold",
                &["Signal's in the noise floor. Add a booster or shorten that run."],
            ),
            (
                "outage_start",
                &["Uh oh, ingress on the line — chop chop, I do NOT do well with snowy channels."],
            ),
            (
                "outage_resolved",
                &["Line's clean again! You chased that noise down faster than I could finish sweeping. I don't actually sweep. I don't know why I said that."],
            ),
            (
                "hint_request",
                &["Aww, need a hint on the tap budget? Fine — for you, anything. *wink*"],
            ),
        ])
    }

    /// Linka — mobile / cellular RF.
    fn linka() -> Self {
        Self::from_pairs(&[
            (
                "clean_splice",
                &[
                    "Ooh, a clean handoff with zero dropped calls? Be still my baseband, senpai.",
                    "Mmm, that's a solid RSRP. I could watch you tune antennas all day.",
                    "Look at you optimizing that link budget like you mean it. Cute.",
                ],
            ),
            (
                "messy_splice",
                &[
                    "That much path loss made me flinch. Physically. I'm literally just radio waves.",
                    "Bold of you to skip the site survey. I'm not mad, just... concerned about your SINR.",
                ],
            ),
            (
                "level_win",
                &[
                    "In-window on the first try! You're making it very hard to focus on my job here.",
                    "Five bars, no jitter. Show-off.",
                ],
            ),
            (
                "level_fail_hot",
                &["Whoa, you're overdriving that PA — you're gonna desense the receiver. Back it off, hotshot."],
            ),
            (
                "level_fail_cold",
                &["Signal's in the noise floor. Add gain or move closer to the tower."],
            ),
            (
                "outage_start",
                &["Uh oh, we dropped to zero bars — chop chop, I do NOT do well with dead air."],
            ),
            (
                "outage_resolved",
                &["Bars are back! You re-acquired that carrier faster than I could finish my coffee. I don't drink coffee. I don't know why I said that."],
            ),
            (
                "hint_request",
                &["Aww, need a hint on the link budget? Fine — for you, anything. *wink*"],
            ),
        ])
    }

    /// Lattice — Ethernet / copper LAN.
    fn lattice() -> Self {
        Self::from_pairs(&[
            (
                "clean_splice",
                &[
                    "Ooh, a punch-down with zero crosstalk? Be still my collision domain, senpai.",
                    "Mmm, that's a clean 568B pinout. I could watch you dress cable all day.",
                    "Look at you labeling every drop like you mean it. Cute.",
                ],
            ),
            (
                "messy_splice",
                &[
                    "That much attenuation made me flinch. Physically. I'm literally just electrons.",
                    "Bold of you to skip the cable tester. I'm not mad, just... concerned about your link light.",
                ],
            ),
            (
                "level_win",
                &[
                    "Full duplex, zero retransmits, first try! You're making it very hard to focus on my job here.",
                    "Gigabit link, clean negotiation. Show-off.",
                ],
            ),
            (
                "level_fail_hot",
                &["Whoa, that's way too much power over that pair — you're gonna cook the PoE injector. Back it off, hotshot."],
            ),
            (
                "level_fail_cold",
                &["Signal's below spec at that length. Add a switch or shorten the run."],
            ),
            (
                "outage_start",
                &["Uh oh, link light just died — chop chop, I do NOT do well with a dead port."],
            ),
            (
                "outage_resolved",
                &["Link's back up! You traced that fault faster than I could finish my ping sweep. I don't actually ping things. I don't know why I said that."],
            ),
            (
                "hint_request",
                &["Aww, need a hint on the cable run? Fine — for you, anything. *wink*"],
            ),
        ])
    }

    pub fn random_line(&self, key: &str) -> Option<&str> {
        self.lines
            .get(key)
            .and_then(|options| options.first())
            .map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_KEYS: &[&str] = &[
        "clean_splice",
        "messy_splice",
        "level_win",
        "level_fail_hot",
        "level_fail_cold",
        "outage_start",
        "outage_resolved",
        "hint_request",
    ];

    #[test]
    fn every_companion_has_every_event_key_with_at_least_one_line() {
        for companion in Companion::ALL {
            let bank = DialogueBank::load_default(companion);
            for key in EXPECTED_KEYS {
                let lines = bank.lines.get(*key);
                assert!(
                    lines.is_some_and(|l| !l.is_empty()),
                    "{companion:?} is missing lines for '{key}'"
                );
            }
        }
    }

    #[test]
    fn each_companion_bank_is_flavored_distinctly() {
        // The four banks share the same keys but must not share the same
        // text -- otherwise switching companions would be cosmetic-only
        // with no actual dialogue payoff.
        let banks: Vec<_> = Companion::ALL
            .iter()
            .map(|c| DialogueBank::load_default(*c))
            .collect();
        for key in EXPECTED_KEYS {
            let first_lines: Vec<_> = banks.iter().map(|b| b.random_line(key)).collect();
            for i in 0..first_lines.len() {
                for j in (i + 1)..first_lines.len() {
                    assert_ne!(
                        first_lines[i], first_lines[j],
                        "companions {i} and {j} share an identical '{key}' line"
                    );
                }
            }
        }
    }

    #[test]
    fn random_line_returns_none_for_an_unknown_key() {
        let bank = DialogueBank::load_default(Companion::Fiber);
        assert_eq!(bank.random_line("not_a_real_event"), None);
    }
}
