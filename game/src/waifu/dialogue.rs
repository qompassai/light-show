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
                    "0.05 dB on the fusion splice. That is exactly how a splice is supposed to read.",
                    "Clean core alignment, loss right at nominal. The OTDR trace will have nothing to gossip about.",
                    "Cleaver prepped like a pro — angle true, face clean. Splice it.",
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
                    "In-window, first try, budget balanced to the tenth. Textbook.",
                    "-15.2 dBm, right in the pocket. Show-off.",
                    "Margin to spare and not a dB wasted. That's the whole job, done right.",
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
                    "Fiber cut! OTDR's already screaming. Get moving — dark fiber helps no one.",
                    "We just lost light on the span. Chop chop — dead fiber makes me *very* cranky.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Service restored! You rerouted that faster than I could finish my coffee. I don't drink coffee. I don't know why I said that.",
                    "Light's back and the trace is clean! You splice like you mean it, senpai.",
                    "Span restored, loss budget intact. That OTDR trace is showing off.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Need a hint? Start with your worst splice — it's usually the obvious one.",
                    "One hint: follow the loss. The budget never lies.",
                    "Check the longest span first. Attenuation compounds quietly.",
                ],
            ),
            (
                "first_placement",
                &[
                    "First component down. Now we find out what the budget thinks of your plan.",
                    "Placed. Remember: every tenth of a dB is a decision.",
                ],
            ),
            (
                "segment_complete",
                &[
                    "Route's connected end to end. Now the only question is the window.",
                    "Full path. Hold your breath for the budget check.",
                ],
            ),
            (
                "too_hot",
                &[
                    "Too hot! That part's cooked — bag it. That's a core for the Warehouse now.",
                    "Overdriven and fried. The Warehouse pays a core for dead parts like that.",
                ],
            ),
            (
                "too_low",
                &[
                    "Signal's fading under the window. We're spending dB we don't have — check your worst loss.",
                    "Too low. Something on this route is eating light; find the hungriest splice.",
                ],
            ),
            (
                "level_complete",
                &[
                    "In window and clean. That's how a route is supposed to land.",
                    "Service restored, budget balanced. Textbook work.",
                ],
            ),
            (
                "level_failed",
                &[
                    "Out of window — the budget doesn't bend, but routes can be rebuilt.",
                    "Missed the window this time. The loss is in there somewhere; we'll find it.",
                ],
            ),
            (
                "idle_nudge",
                &[
                    "Still thinking it over? The window isn't moving — trust your worst splice.",
                    "Take your time. The light's patient, even if the outage clock isn't.",
                ],
            ),
            (
                "identification_wrong",
                &[
                    "That tone reads on the wrong fiber. Label discipline exists so the next tech doesn't inherit our guesses.",
                ],
            ),
            (
                "service_fail",
                &[
                    "Light's in window and the service still fails — read the budget again. The failure is in the plant, not the launch.",
                ],
            ),
            (
                "survey_point_fail",
                &[
                    "A survey point failed its requirement. One dark corner is enough — coverage is a promise per point, not an average.",
                ],
            ),
            (
                "diagnosis_wrong",
                &[
                    "Wrong diagnosis. Compare the old reading to the new one; the quantity that changed is the culprit.",
                ],
            ),
            (
                "workmanship_defect",
                &[
                    "The inspection found a defect in the finished end. Precision is the job — rebuild it to the card, not to memory.",
                ],
            ),
            (
                "config_mismatch",
                &[
                    "Apply rejected. A configuration is a budget like any other: every field accounted, or it doesn't close.",
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
                    "F-connector torqued to spec, return loss clean. That's how you terminate.",
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
                    "Locked in-band on the first pass, tilt flat. That's a sweep worth framing.",
                    "Clean sweep, zero ingress. Show-off.",
                    "Flat sweep, MER to spare, zero ingress. The node approves.",
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
                    "Need a hint on the tap budget? Check your noisiest tap first — ingress loves the obvious.",
                    "One hint: follow the tilt. The sweep never lies.",
                    "Start at the farthest tap and work back. Losses stack in order.",
                ],
            ),
            (
                "first_placement",
                &[
                    "First piece in the cascade. Everything after this is gain staging.",
                    "Placed. Watch the tilt as you build — the high end falls first.",
                ],
            ),
            (
                "segment_complete",
                &[
                    "Cascade's continuous to the tap. The sweep says it's all one plant now.",
                    "End to end. Now we see where the level lands at the drop.",
                ],
            ),
            (
                "too_hot",
                &[
                    "Too hot — that stage is clipping! Cooked part... bag it, that's a core now.",
                    "Overdriven into distortion. Fried is fried: one more core for the bin.",
                ],
            ),
            (
                "too_low",
                &[
                    "Level's sinking toward the noise floor. More gain or fewer splits — pick one.",
                    "Too low at the tap. Something's eating your dBmV; check the longest run.",
                ],
            ),
            (
                "level_complete",
                &[
                    "Flat sweep, in window, MER to spare. That's a clean cascade.",
                    "Unity gain, locked in-band. Beautiful balance.",
                ],
            ),
            (
                "level_failed",
                &[
                    "Out of window. The cascade keeps every receipt — rebalance it stage by stage.",
                    "Missed the window. Tilt or level — one of them is lying to you.",
                ],
            ),
            (
                "idle_nudge",
                &[
                    "Meter's waiting on you. Ingress loves an idle line, you know.",
                    "Still mapping it out? Start from the tap and work back.",
                ],
            ),
            (
                "identification_wrong",
                &[
                    "That mapper reading doesn't match the work order. Wrong run — test them, don't trust handwriting.",
                ],
            ),
            (
                "service_fail",
                &[
                    "Continuity's fine, the level's fine — and service is still drowning. That's not a gain problem, that's a bad piece of plant. Find it and swap it.",
                ],
            ),
            (
                "survey_point_fail",
                &[
                    "One point's under its line. Out here that's the far end of the drop — passing at the distribution point isn't passing at the modem.",
                ],
            ),
            (
                "diagnosis_wrong",
                &[
                    "Wrong call. Look at what the tones said then and what they say now — the delta names the fault.",
                ],
            ),
            (
                "workmanship_defect",
                &[
                    "Inspection caught what your eyes waved through. A defect you ship is a callback you schedule — rebuild the end and test it like you mean it.",
                ],
            ),
            (
                "config_mismatch",
                &[
                    "Rejected at apply. Out here the worksheet is the tone sheet — the value on paper is the value that ships, not the one that looks close.",
                ],
            ),
            (
                "tutorial_workbench",
                &[
                    "The bench is a state machine, not a vibe: strip to the card, fold the braid back, seat it flush, compress once. A mistake the card catches costs a connector. A mistake it can't — shielding, seating, the short test — ships. Work the card, in order, every end.",
                ],
            ),
            (
                "tutorial_intermittent",
                &[
                    "A drop that blinks when the cabinet's disturbed isn't a signal problem — it's a connection moving. Don't chase gain. Reproduce it, inspect the run, tighten the loose one to the card, then disturb it again and make it prove it holds. Rerouting around a loose fitting just schedules the callback.",
                ],
            ),
            (
                "tutorial_identification",
                &[
                    "New bench rule: identify before you amplify. Attach the remote at the drop, test every candidate in the closet, and let the mapper — not a handwritten label — tell you which run is ours.",
                ],
            ),
        ])
    }

    /// Linka — wireless / Wi-Fi RF. Her lines are discipline-first per
    /// Matt's direction: RSSI readouts and resolving Wi-Fi interference
    /// (co-channel networks, noisy neighbors, channel choice, antenna
    /// geometry) — in her voice, never the flirt pool.
    fn linka() -> Self {
        Self::from_pairs(&[
            (
                "clean_splice",
                &[
                    "Clean association, zero retries — that's how you join a network.",
                    "Solid RSSI on a clean channel. You pick your spots well.",
                    "Textbook placement. The spectrum noticed, trust me.",
                ],
            ),
            (
                "messy_splice",
                &[
                    "That RSSI made me flinch. Something's interfering — or the geometry's wrong.",
                    "Bold of you to skip the site survey. Co-channel interference loves confidence like that.",
                    "That link's dropping packets. Re-aim it before the noise floor files a complaint.",
                ],
            ),
            (
                "level_win",
                &[
                    "RSSI in the sweet spot, channel clean. That's a link I'd sign.",
                    "Strong signal, zero interference complaints. Show-off.",
                    "In window with margin to spare. The airwaves approve.",
                ],
            ),
            (
                "level_fail_hot",
                &[
                    "Too hot — you're desensing the receiver! Back the power off before you cook the front end.",
                    "That RSSI is suspiciously strong and totally unusable: saturation. Dial it down.",
                    "Overdriving helps no one. Less power, cleaner channel.",
                ],
            ),
            (
                "level_fail_cold",
                &[
                    "RSSI's under the floor. Shorten the path or find a cleaner channel.",
                    "We're in a dead zone. Better geometry or a repeater — pick one.",
                    "Signal's a whisper in a loud room. Move closer or clear the interference.",
                ],
            ),
            (
                "outage_start",
                &[
                    "Uh oh — RSSI just fell off a cliff. Chop chop, dead air helps no one.",
                    "We lost the link entirely! Get moving — interference doesn't hunt itself.",
                    "Carrier's gone. Every minute dark is a minute the noise wins.",
                ],
            ),
            (
                "outage_resolved",
                &[
                    "Link's back and the RSSI's honest again. Fast recovery.",
                    "Clean channel, stable signal. You hunt interference like a pro.",
                    "Back in window. Whatever was shouting out there, you routed around it.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Need a hint on the interference hunt? Check your weakest RSSI first.",
                    "One hint: follow the RSSI trend. The measurements never lie.",
                    "Hint: find what's sharing your channel before you touch the power.",
                ],
            ),
            (
                "first_placement",
                &[
                    "First radio down. Now we chase RSSI — stronger isn't always cleaner.",
                    "Placed. Read the RSSI before you trust it; interference hides in good numbers.",
                ],
            ),
            (
                "segment_complete",
                &[
                    "Link's closed end to end. Now — is the RSSI honest?",
                    "Full path. If the RSSI holds, we're golden.",
                ],
            ),
            (
                "too_hot",
                &[
                    "Too hot — that front end's saturated! Fried radio... bag it, that's a core.",
                    "Blasted past the top of the window. Cooked part, one core — Warehouse rules.",
                ],
            ),
            (
                "too_low",
                &[
                    "RSSI's in the basement. Either the path's too long or something's shouting over us — find the interference.",
                    "Too weak. Fix the geometry or clear the channel; gain alone won't fix a noisy floor.",
                ],
            ),
            (
                "level_complete",
                &[
                    "RSSI strong, channel clean, zero retries. That's a link.",
                    "In window with margin. Whatever was interfering, you routed around it.",
                ],
            ),
            (
                "level_failed",
                &[
                    "Out of window. Don't just add power — hunt the interference first.",
                    "Missed it. Check what's sharing your channel before you blame the distance.",
                ],
            ),
            (
                "idle_nudge",
                &[
                    "Still scanning? Trust the RSSI trend, not the first pretty number.",
                    "Quiet air's deceptive. Pick your channel and commit.",
                ],
            ),
            (
                "identification_wrong",
                &[
                    "Wrong run selected. In my world that's associating to the neighbor's AP — same lesson: verify the ID before you trust the link.",
                ],
            ),
            (
                "service_fail",
                &[
                    "RSSI healthy, SNR failing — loud is not clear. Something is raising your floor, and more power won't fix it.",
                ],
            ),
            (
                "survey_point_fail",
                &[
                    "That point is below its line — the design passed at the desk and failed at the shelf. Survey the room you're serving, not the room you wish you had.",
                ],
            ),
            (
                "diagnosis_wrong",
                &[
                    "Wrong cause. The history is right there — what actually moved between then and now? Chase evidence, not hunches.",
                ],
            ),
            (
                "workmanship_defect",
                &[
                    "A defect went live in that termination. In RF that's a return-loss story — rebuild it and re-measure; hope isn't a test set.",
                ],
            ),
            (
                "config_mismatch",
                &[
                    "Rejected. Read the verdict, not your hopes: one field disagrees with the worksheet. Availability comes from the inventory — the DHCP pool is a rumor, not a rule.",
                ],
            ),
            (
                "tutorial_static_ipv4",
                &[
                    "Static IPv4, worksheet discipline: the assigned address is the one in the inventory under the printer's name. In the DHCP pool doesn't mean available, and out of the pool doesn't mean free — the inventory is the only truth. Gateway and DNS come off the same sheet.",
                ],
            ),
            (
                "tutorial_static_ipv6",
                &[
                    "IPv6 adds one trap: a link-local gateway, fe80::, is only meaningful scoped to its interface. Right address, wrong interface, no route — the apply will tell you exactly that, in exactly those words.",
                ],
            ),
            (
                "tutorial_history_diagnosis",
                &[
                    "Read the history like a tech, not a fortune-teller: same RSSI then and now, SNR down — the signal didn't move, the noise did. That's interference, every time.",
                ],
            ),
            (
                "tutorial_survey",
                &[
                    "A design that passes at one point is a hypothesis. Survey every named point against its own line — the shelf in the back doesn't care about your average.",
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
                    "Clean 568B pinout, pairs untwisted the minimum. That certifies.",
                    "Every drop labeled, every run dressed. The next tech thanks you.",
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
                    "Full duplex, zero retransmits, first try. Every constraint green.",
                    "Gigabit link, clean negotiation. Show-off.",
                    "Wire-speed, zero errors, margin to spare. Cleanest path on the board.",
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
                    "Network's back, zero collisions. Your terminations held under pressure.",
                ],
            ),
            (
                "hint_request",
                &[
                    "Need a hint on the cable run? Check your longest run first — the 100 m wall is real.",
                    "One hint: follow the link lights. A dark port is a confession.",
                    "Hint: meter the run before you blame the switch. Copper keeps receipts.",
                ],
            ),
            (
                "first_placement",
                &[
                    "First drop punched down. Mind the untwist — crosstalk starts there.",
                    "Placed. Every run from here counts against the 100 m wall.",
                ],
            ),
            (
                "segment_complete",
                &[
                    "Path's continuous end to end. Now the constraints decide: length, PoE, bandwidth.",
                    "Full run. Check it against the checklist before you celebrate.",
                ],
            ),
            (
                "too_hot",
                &[
                    "Too hot — that PoE load cooked the part! Bag it: one core for the Warehouse.",
                    "Over the PoE budget and fried. Dead part, one core — that's the trade.",
                ],
            ),
            (
                "too_low",
                &[
                    "Below PHY spec at that length. Shorten the run or add a switch — physics won't negotiate.",
                    "Signal's under spec. Something on this run is too long or too lossy.",
                ],
            ),
            (
                "level_complete",
                &[
                    "Wire-speed, zero errors, every constraint green. That's a certified run.",
                    "Full duplex and in spec. The checklist has nothing left to say.",
                ],
            ),
            (
                "level_failed",
                &[
                    "Out of spec. The constraints keep receipts — measure the longest run first.",
                    "Missed it. Length, PoE, or bandwidth: one of the three is lying.",
                ],
            ),
            (
                "idle_nudge",
                &[
                    "Still measuring? Start with the longest run — it's the usual suspect.",
                    "Take your time. The link lights aren't going anywhere.",
                ],
            ),
            (
                "identification_wrong",
                &[
                    "Wrong drop. A mislabeled run is exactly how a neighbor's port gets pulled — verify before you disconnect anything.",
                ],
            ),
            (
                "service_fail",
                &[
                    "Link light's green and the service still fails. Continuity is not acceptance — test the service, not just the wire.",
                ],
            ),
            (
                "survey_point_fail",
                &[
                    "That point's below spec. A jack that links in the office and dies at the desk isn't done — check the point, not the switch.",
                ],
            ),
            (
                "diagnosis_wrong",
                &[
                    "Wrong cause. Diff the history against today — one number moved. Follow the one that moved.",
                ],
            ),
            (
                "workmanship_defect",
                &[
                    "That termination failed inspection. A bad crimp passes a tug and fails a certifier — rebuild it and certify it.",
                ],
            ),
            (
                "config_mismatch",
                &[
                    "Rejected at apply. Check the inventory before the pool, and the worksheet before both — that's the order the network believes.",
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

    const REACTION_KEYS: &[&str] = &[
        "first_placement",
        "segment_complete",
        "too_hot",
        "too_low",
        "level_complete",
        "level_failed",
        "idle_nudge",
    ];

    #[test]
    fn every_main_companion_has_every_reaction_key() {
        for companion in Companion::ALL {
            let bank = DialogueBank::load_default(companion);
            for key in REACTION_KEYS {
                assert!(
                    bank.lines.get(*key).is_some_and(|l| !l.is_empty()),
                    "{companion:?} is missing reaction lines for '{key}'"
                );
            }
        }
    }

    #[test]
    fn results_and_reaction_pools_carry_no_arousal_framing() {
        // Playtest round 2 (Matt): the aroused/flirt pool is out of
        // results and reaction text. Professional-playful is fine;
        // these substrings are the arousal framings that shipped once
        // and must never come back on these surfaces.
        const BANNED: &[&str] = &[
            "hard to focus",
            "be still my heart",
            "favorite kind of",
            "good hands",
            "flirt",
            "*wink*",
            "ara ara",
            "i could watch you",
        ];
        const SURFACES: &[&str] = &[
            "level_win",
            "level_complete",
            "level_failed",
            "outage_resolved",
            "first_placement",
            "segment_complete",
            "too_hot",
            "too_low",
            "idle_nudge",
            "hint_request",
        ];
        for companion in Companion::ALL {
            let bank = DialogueBank::load_default(companion);
            for key in SURFACES {
                for line in bank.lines.get(*key).into_iter().flatten() {
                    let lower = line.to_lowercase();
                    for banned in BANNED {
                        assert!(
                            !lower.contains(banned),
                            "{companion:?} '{key}' line carries banned framing '{banned}': {line}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn json_mirrors_match_the_compiled_banks() {
        // The JSON files under assets/dialogue are doc/localization
        // mirrors of the compiled banks (not runtime-loaded). They
        // must not drift: a stale mirror once kept shipping the old
        // flirt lines after the bank was rewritten.
        for companion in Companion::ALL {
            let stem = companion.picker_stem();
            let path = format!("{}/assets/dialogue/{stem}_en.json", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("mirror {path} unreadable: {e}"));
            let mirror: DialogueBank = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("mirror {path} does not parse: {e}"));
            let bank = DialogueBank::load_default(companion);
            assert_eq!(mirror.lines, bank.lines, "mirror {stem}_en.json drifted from the compiled bank");
        }
    }

    #[test]
    fn random_line_returns_none_for_an_unknown_key() {
        let bank = DialogueBank::load_default(Companion::Fiber);
        assert_eq!(bank.random_line("not_a_real_event"), None);
    }
}
