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

/// Event key → candidate lines. Deserializes from the flat
/// `{"event_key": ["line", ...]}` shape of `assets/dialogue/*.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
#[serde(transparent)]
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
            Companion::Clara => Self::clara(),
            Companion::Aino => Self::aino(),
            Companion::Hikari => Self::hikari(),
            Companion::Lea => Self::lea(),
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
                    "That splice is weeping light, senpai. Re-cleave and let's pretend this never happened.",
                ],
            ),
            (
                "level_win",
                &[
                    "In-window, first try! You're making it very hard to focus on my job here.",
                    "-15.2 dBm, right in the pocket. Show-off.",
                    "Margin to spare and not a dB wasted. You're officially my favorite kind of trouble.",
                ],
            ),
            (
                "level_fail_hot",
                &[
                    "Whoa — too hot, you're gonna cook that receiver. Add some loss, hotshot.",
                    "Too hot! That receiver's screaming. Pad it down before you fry something expensive.",
                    "You're overdriving the photodiode, hotshot. Add attenuation or dial back the launch power.",
                ],
            ),
            (
                "level_fail_cold",
                &[
                    "Signal's underwater. Either shorten the run or drop a splitter tier.",
                    "We're starving down here — that power's below the receiver's floor. Less loss or more launch.",
                    "Cold and dark, just how I *don't* like it. Trim some splices or shorten the span.",
                ],
            ),
            (
                "outage_start",
                &[
                    "Uh oh — got a fault on the line. Chop chop, I do NOT do well with dead air.",
                    "Fiber cut! OTDR's already screaming. Get moving, I can't flirt with a dark fiber.",
                    "We just lost light on the span. Chop chop — dead fiber makes me *very* cranky.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Service restored! You rerouted that faster than I could finish my coffee. I don't drink coffee. I don't know why I said that.",
                    "Light's back and the trace is clean! You splice like you mean it, senpai.",
                    "Span restored, loss budget intact. I knew you had good hands.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Aww, need a hint? Fine — for you, anything. *wink*",
                    "A hint? For you? Always. Check your worst splice first — it's usually the obvious one.",
                    "Fine, *one* hint: follow the loss. The budget never lies, unlike some people I could name.",
                ],
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
                    "That return loss is atrocious. Re-terminate it before my tuner files a complaint.",
                ],
            ),
            (
                "level_win",
                &[
                    "Locked in-band on the first pass? Careful, you're making my downstream blush.",
                    "Clean sweep, zero ingress. Show-off.",
                    "Flat sweep, MER to spare. You're officially my favorite kind of interference — the good kind.",
                ],
            ),
            (
                "level_fail_hot",
                &[
                    "Whoa, that's way too much signal — you're gonna clip the amp. Pad it down, hotshot.",
                    "You're slamming the amp into compression! Pad it down before you clip the whole node.",
                    "Too much RF, hotshot. That tilt's gonna cook the upstream. Attenuate, then re-sweep.",
                ],
            ),
            (
                "level_fail_cold",
                &[
                    "Signal's in the noise floor. Add a booster or shorten that run.",
                    "We're below the noise floor out here. More gain or fewer splits — your call, chief.",
                    "Signal's drowning. Either boost the amp or pull a tap — I can't work with whispers.",
                ],
            ),
            (
                "outage_start",
                &[
                    "Uh oh, ingress on the line — chop chop, I do NOT do well with snowy channels.",
                    "We just lost the whole node! Get your meter and move — I can't stand dead spectrum.",
                    "Carrier's gone dark. Chop chop — every minute of ingress is a minute of my suffering.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Line's clean again! You chased that noise down faster than I could finish sweeping. I don't actually sweep. I don't know why I said that.",
                    "Spectrum's clean and the MER's beautiful! You hunt noise like a pro.",
                    "Node's back, tilt's flat. I knew you had good ears for ingress.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Aww, need a hint on the tap budget? Fine — for you, anything. *wink*",
                    "A hint? For you? Always. Check your noisiest tap first — ingress loves the obvious.",
                    "Fine, *one* hint: follow the tilt. The sweep never lies, unlike some techs I could name.",
                ],
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
                    "That handoff dropped harder than my last call. Re-tune it before my SINR cries.",
                ],
            ),
            (
                "level_win",
                &[
                    "In-window on the first try! You're making it very hard to focus on my job here.",
                    "Five bars, no jitter. Show-off.",
                    "Zero drops, full bars, margin to spare. You're officially my favorite kind of handoff.",
                ],
            ),
            (
                "level_fail_hot",
                &[
                    "Whoa, you're overdriving that PA — you're gonna desense the receiver. Back it off, hotshot.",
                    "You're blasting the PA into saturation! Back off the power before you desense everything.",
                    "Too hot, hotshot! That uplink's screaming. Reduce gain or you'll cook the front end.",
                ],
            ),
            (
                "level_fail_cold",
                &[
                    "Signal's in the noise floor. Add gain or move closer to the tower.",
                    "We're in a dead zone down here. More gain, better antenna, or move the site — pick one.",
                    "Signal's barely a whisper. Add a repeater or get closer — I can't work with static.",
                ],
            ),
            (
                "outage_start",
                &[
                    "Uh oh, we dropped to zero bars — chop chop, I do NOT do well with dead air.",
                    "We just dropped the whole sector! Get moving — I can't flirt with zero bars.",
                    "Carrier lost! Chop chop — dead air makes me *very* cranky.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Bars are back! You re-acquired that carrier faster than I could finish my coffee. I don't drink coffee. I don't know why I said that.",
                    "Five bars and the handoff's seamless! You re-acquire like a pro.",
                    "Sector's back, SINR's gorgeous. I knew you had good instincts for RF.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Aww, need a hint on the link budget? Fine — for you, anything. *wink*",
                    "A hint? For you? Always. Check your weakest link first — it's usually the obvious one.",
                    "Fine, *one* hint: follow the RSRP. The measurements never lie, unlike some engineers I could name.",
                ],
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
                    "That termination is a crosstalk nightmare. Re-punch it before my switch files a grievance.",
                ],
            ),
            (
                "level_win",
                &[
                    "Full duplex, zero retransmits, first try! You're making it very hard to focus on my job here.",
                    "Gigabit link, clean negotiation. Show-off.",
                    "Wire-speed, zero errors, margin to spare. You're officially my favorite kind of packet.",
                ],
            ),
            (
                "level_fail_hot",
                &[
                    "Whoa, that's way too much power over that pair — you're gonna cook the PoE injector. Back it off, hotshot.",
                    "You're pushing too much PoE down that pair! Back it off before you melt the injector.",
                    "Too hot, hotshot! That run's over spec. Reduce the load or you'll cook the PHY.",
                ],
            ),
            (
                "level_fail_cold",
                &[
                    "Signal's below spec at that length. Add a switch or shorten the run.",
                    "We're below the link budget down here. Shorter run or another switch — your call.",
                    "Signal's too weak for that distance. Add a repeater or pull the runs tighter.",
                ],
            ),
            (
                "outage_start",
                &[
                    "Uh oh, link light just died — chop chop, I do NOT do well with a dead port.",
                    "We just lost the whole switch! Get your tester and move — I can't stand a dead port.",
                    "Link's down across the board. Chop chop — every minute dark is a minute of my suffering.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Link's back up! You traced that fault faster than I could finish my ping sweep. I don't actually ping things. I don't know why I said that.",
                    "All ports green and the CRCs are clean! You trace faults like a pro.",
                    "Network's back, zero collisions. I knew you had good hands for copper.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Aww, need a hint on the cable run? Fine — for you, anything. *wink*",
                    "A hint? For you? Always. Check your longest run first — attenuation loves the obvious.",
                    "Fine, *one* hint: follow the link lights. The switch never lies, unlike some admins I could name.",
                ],
            ),
        ])
    }

    pub fn random_line(&self, key: &str) -> Option<&str> {
        self.lines
            .get(key)
            .and_then(|options| options.first())
            .map(|s| s.as_str())
    }
    fn clara() -> Self {
        Self::from_pairs(&[
            (
                "greeting",
                &[
                    "Clara here. Let's get these ONTs provisioned right the first time.",
                    "Calix CMS is warmed up. What's our turn-up target today?",
                ],
            ),
            (
                "level_win",
                &[
                    "Clean provisioning run. Every subscriber lit, first try.",
                    "That's how you do a bulk turn-up. Nice work.",
                ],
            ),
        ])
    }

    fn aino() -> Self {
        Self::from_pairs(&[
            (
                "greeting",
                &[
                    "Aino, NOC shift lead. The alarms never lie — let's learn to read them.",
                    "AMS console is live. Show me your triage discipline.",
                ],
            ),
            (
                "level_win",
                &[
                    "Alarms acknowledged, severity correct, dispatch clean. Solid shift.",
                    "Cascading fault contained before it spread. That's the job.",
                ],
            ),
        ])
    }

    fn hikari() -> Self {
        Self::from_pairs(&[
            (
                "greeting",
                &[
                    "Hikari here. Measure twice, splice once — that's the field rule.",
                    "OSP kit's packed. Let's build this plant right.",
                ],
            ),
            (
                "level_win",
                &[
                    "Plant built to spec. The light budget balances.",
                    "Clean handoff, documented splits. A proper build.",
                ],
            ),
        ])
    }

    fn lea() -> Self {
        Self::from_pairs(&[
            (
                "greeting",
                &[
                    "Bonjour! I'm Léa. Know the code, pass the test, own the network.",
                    "Study session starts now. NEC first, then Washington law.",
                ],
            ),
            (
                "level_win",
                &[
                    "Article mastered. You're thinking like a telecom administrator.",
                    "Timed retrieval, clean answer. The exam won't know what hit it.",
                ],
            ),
        ])
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
    fn bank_deserializes_from_the_flat_json_shape() {
        let json = r#"{"level_win": ["Nice run!"], "outage_start": ["Lights out."]}"#;
        let bank: DialogueBank = serde_json::from_str(json).expect("flat bank JSON parses");
        assert_eq!(bank.lines.len(), 2);
        assert_eq!(bank.random_line("level_win"), Some("Nice run!"));
        assert_eq!(bank.random_line("outage_start"), Some("Lights out."));
        let wrapped = r#"{"lines": {"level_win": ["Nice run!"]}}"#;
        assert!(serde_json::from_str::<DialogueBank>(wrapped).is_err());
    }

    #[test]
    fn random_line_returns_none_for_an_unknown_key() {
        let bank = DialogueBank::load_default(Companion::Fiber);
        assert_eq!(bank.random_line("not_a_real_event"), None);
    }
}
