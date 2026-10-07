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
                "tutorial_briefing_gestures",
                &[
                    "The bench is simple: tap an open span between two points, then pick the component that fills it — a splice, a span, a splitter. Placed something wrong? Tap it again and swap it. Nothing is final until the light adds up.",
                    "Watch the path light up as you build. A finished route from OLT to ONT is a route I can measure — and I will measure it, out loud, every time.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "Here's the only math that matters: launch power, minus every loss on the path, equals receive power. Fiber eats about 0.28 dB per kilometre at 1490. A fusion splice costs 0.075 dB, a mechanical one 0.4 — cheap now, expensive across forty splices.",
                    "And when you meet a splitter, respect it: a 1x4 takes 7.3 dB off the top before your signal does anything else. Splitters are where budgets go to be humbled.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Welcome to Splice School — I'm Séraphine, and I teach fiber. Every level I give you works the same way: light leaves the OLT at a launch power, the plant takes its cut, and what's left has to land inside the ONT's receive window.",
                    "Too little light and the receiver starves — TOO LOW. Too much and you cook the photodiode — TOO HOT. Your whole job is the number in between. Route the light, hit the window.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "Storm school, one lesson early: spans fail. Aerial plant takes wind damage, buried plant takes backhoes. When light dies mid-level, don't panic and don't rebuild — reroute onto the protection path and restore service first. The post-mortem can wait; the customer can't.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "First splice placed — good. See how the ledger updated? Every component you place writes a line: what it is, what it cost. That ledger is the OTDR's diary. Read it and you'll never guess at a fault again.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "There's your first verdict, and the ledger underneath tells you WHY — which span ate what, down to the tenth of a dB. TOO LOW means add light or cut loss; TOO HOT means pad it down. Never argue with a verdict. Follow the loss.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "One more thing before you run solo: parts you fry — components that go TOO HOT — come back as cores at results. The Warehouse trades in dead parts, so even a cooked amplifier isn't a total loss. Bank them, spend them, and try not to make a habit of cooking my inventory.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Last thing, and it will matter more than anything I taught today: fiber has a name, and the name is a color pair — the tube it rides in, and its own color inside the tube. Blue tube, orange strand is a different fiber from orange tube, blue strand. When we reach the splice tray, that pair is the only identity a strand has. Verify the pair, then verify the light.",
                ],
            ),
            (
                "fj_coda_cat3",
                &[
                    "One more call before we roll: the neighbor's landline is dead. Don't guess — test. The Klein tester condemns the Cat 3 run in one pass: BAD CABLE. Swap the run, retest, dial tone. The tester verdict, not hope, is what condemns a cable. Write it in the notes and go home clean.",
                ],
            ),
            (
                "fj_demarc_trap",
                &[
                    "Survey skepticism, free of charge: the obvious box on the south wall belongs to the legacy carrier and it is NOT the demarc. The real demarc is on the roof, where the aerial path has been telling you to look all along. Verify the demarc. The obvious box lies.",
                ],
            ),
            (
                "fj_partial_warning",
                &[
                    "Stop and read the ledger before you call that done. You fixed one fault and left the other — the bleed and the dead splitter are separate line items, and the meter grades the whole path, not your effort. Both repairs, then re-measure.",
                ],
            ),
            (
                "fj_survey_ports",
                &[
                    "Ports one through four: about -25.00, every one. When all four legs read the same weak number, the fault is upstream and shared — the feed — not four coincidentally identical legs. Uniform readings indict the common path. Remember that shape.",
                ],
            ),
            (
                "fj_survey_sb",
                &[
                    "Meter at the SB: -25.45 dBm. For this plant that's a whisper — the receiver is straining to hear it. Write it down. A survey you can't quote from memory is a survey you'll redo from a ladder.",
                ],
            ),
            (
                "fj_survey_vfl",
                &[
                    "VFL out. There — light bleeding at the splice, and again in the white feed at the center block. Two faults, both visible, both in the ledger the moment you know where to look. This is why we measure before we touch: the plant was confessing the whole time.",
                ],
            ),
            (
                "fj_swap_done",
                &[
                    "New 1x4 in, tails redressed, port four capped — a capped port is a port that can't collect water, dirt, or regrets. Putty the vault and the handhole, tag the drop, and re-measure everything. The job isn't the repair. The job is the proof.",
                ],
            ),
            (
                "sp_beat1_clear",
                &[
                    "One tube down. Notice you never counted strands — you matched colors and the colors told the truth, because inside one tube they can't repeat. Hold onto how easy that felt. It's about to stop being true.",
                ],
            ),
            (
                "sp_beat2_trap",
                &[
                    "Stop. Look at fibers 2 and 13 before you touch anything: blue-orange and orange-blue. Same two colors, opposite chairs. If your hands splice by color memory here, you will light the wrong house beautifully. Pair first. Always.",
                ],
            ),
            (
                "sp_beat3_numbers",
                &[
                    "Numbers only now. Decode before you reach: fiber 26 — green tube, orange strand. The chart is not a crutch, it's the work order's other half. Fluency is just the chart, memorized by your hands.",
                ],
            ),
            (
                "sp_chart_handover",
                &[
                    "Before the tray opens, the chart — I'm putting it in your hands, not on a poster. Twelve colors, one sequence: blue, orange, green, brown, slate, white, red, black, yellow, violet, rose, aqua. Tubes wear it. Strands wear it again. A fiber's name is where the two cross.",
                ],
            ),
            (
                "sp_damaged_strand",
                &[
                    "That strand is crushed — see the tray-edge damage? Do not force a match that isn't there. Exclude it, take the designated spare, and write the swap on the work order so the next tech's chart matches the tray. A splice tray is a document. Author it like one.",
                ],
            ),
            (
                "sp_wrap",
                &[
                    "Tray closed, every drop lit by its own fiber. That's the whole craft in one sentence: identity before neatness, the pair before the light. The next tray you open alone will have worse lighting and no one narrating. You'll hear the sequence anyway.",
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
                "tutorial_briefing_gestures",
                &[
                    "Same bench, different plant: tap an open run, pick what fills it — coax span, tap, amplifier. Amplifiers are the fun ones: place one and the level jumps, but so does everything wrong with the signal.",
                    "Swap freely while you learn. A cascade is a chain of decisions, and I would rather you experiment on my bench than on a live node at 2 a.m.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "Coax math: RG-6 eats roughly 5.5 dB per hundred metres, and it eats more at high frequencies than low — that's tilt, and it will matter. An amplifier gives you its gain in dB back, minus honesty: every amp adds its noise figure to the noise floor.",
                    "Which brings us to the number that runs my whole track: carrier-to-noise. CNR is your signal measured against the noise riding with it. Levels can be perfect while CNR fails — a loud, dirty signal is still a failed signal.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Ondine, coax. Forget everything fiber taught you about light — here we move RF, and we measure it in dBmV: decibels relative to one millivolt across 75 ohms. The headend launches hot, the plant bleeds level with distance, and the customer's tap has to land in its window.",
                    "Your job on my levels: balance the cascade. Every amplifier you place adds gain AND noise. Level is easy to buy. Clean level is the craft.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "Ingress. Learn the word. A cracked shield, a loose connector, a corroded fitting — every breach lets the outside world's RF leak INTO my plant, and it all lands on the return path. When an ingress storm starts, your noise floor climbs about a dB every ten seconds. The fix is never a bigger amplifier. Amplify ingress and you've built a louder problem.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "Placed. Now read the ledger the coax way: it tracks your level in dBmV AND your carrier-to-noise, hop by hop. When a reading confuses you, the hop where the numbers bent is the hop that did it.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "First verdict. On coax you get three ways to fail: TOO LOW is starvation, TOO HOT is distortion — an overdriven amp smears the signal across itself — and CNR TOO LOW means the carrier is drowning in noise even at a healthy level. The verdict names which one. Believe it.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "House rules: overdrive an amplifier into distortion and it comes back as a core at results — the Warehouse pays for dead parts, and Ondine's rule is that a core earned is a lesson learned. Earn a few while the stakes are fake. Out there, that amp is a truck roll.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Unity gain is the whole philosophy, so say it with me: what the plant takes, the cascade returns — no more, no less. Balance first, bravado never. Now go balance your first cascade.",
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
                "tutorial_briefing_gestures",
                &[
                    "The bench, RF edition: tap an open hop and fill it — a wireless hop, an amplifier, or a repeater. The repeater is the one to watch: it doesn't boost what it hears, it retransmits fresh at its own power. Completely different animal from an amp.",
                    "Distances are printed on every hop. Geometry is a component too — sometimes the best part you can place is a shorter path.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "Wireless math starts with the cruel one: free-space path loss. Double the distance at 2.4 GHz and you pay about 6 dB for it. Four hundred metres costs you roughly 92 dB before weather, walls, or bad luck get a vote.",
                    "Then the number I actually grade: SNR. Your receiver hears signal and noise together; what matters is the gap between them. I'll quote you RSSI all day, but a strong RSSI with a stronger noise floor is a failing link wearing a costume.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Linka. I run wireless, which means I run the discipline of RSSI: received signal strength, in dBm, measured at the far end of a hop that would love to fail. Your job on my levels: close the link — land the receive level inside the window with a signal-to-noise ratio that holds.",
                    "Anyone can shout across a field. We engineer links that still work when the band gets crowded and the weather turns.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "That's interference arriving. A co-channel network, a noisy neighbor AP, a microwave with a death wish — interference raises the effective noise floor and your SNR margin evaporates even though your RSSI never moved. When it starts: don't just add power. Change the geometry, change the channel plan in your head, or bridge the bad stretch with a repeater and reset the fight on your terms.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "Hop placed. Read the ledger my way: per-hop RSSI and the running SNR. The hop where SNR sags is where your link budget went — find it before it finds you at install time.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "First verdict, RF rules: TOO LOW means the receiver can't hear you — path loss won. TOO HOT means you're blasting the front end into compression. And SNR TOO LOW is the subtle one: plenty of signal, not enough of it standing above the noise. Three different diseases, three different cures. The verdict tells you which patient you have.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "Salvage rules, wireless edition: cook a receiver front end with a TOO HOT blast and the dead radio comes back as a core at results. The Warehouse pays, I learn what you did, and somewhere a WASPy little access point gets a second life. Still — measure twice, transmit once.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Close the link, keep the margin, respect the noise floor. That's the whole religion. Now — two sites, one field, no cable to save you. Show me a link budget.",
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
                "tutorial_briefing_gestures",
                &[
                    "The bench, structured-cabling edition: tap an open run, choose the copper that fills it — or the switch that breaks it up. Placement is commitment here: a run you place is a run some future tech has to live with. Place like you'll be the one tracing it at midnight. Because you will.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "My math has no decibels, and it is not softer for it. Three hard constraints: one hundred metres per copper segment — that is the wall, and physics does not negotiate. Bandwidth: the far end must still get the megabits the level demands. And PoE: add up every powered device's draw and check it against the budget before you admire your topology.",
                    "Violate any one of the three and the level fails, even if the other two are beautiful. Constraints are not suggestions with better PR.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Lattice. Wired Ethernet. My levels don't grade light or RF — they grade whether your link obeys the physics of copper and the arithmetic of power. The objective on every one: connect the endpoint, inside the distance wall, at the bandwidth demanded, within the PoE budget.",
                    "Fiber gets the glory. Copper gets the building. Learn it properly.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "When something on my track goes down mid-level, it will not be romantic. It will be a switch out of PoE budget, a run past the wall, a link negotiated down to nothing. Read the failure as a constraint violation, find which of the three you broke, and fix the cause — not the symptom.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "Placed. The ledger on my track is a checklist: segment lengths against the wall, delivered bandwidth against the demand, PoE draw against the budget. Green across the board or it isn't done. 'Mostly connected' is a helpdesk ticket, not a result.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "First verdict. My verdicts name the broken constraint: TOO LONG, UNDER-BANDWIDTH, or OVER BUDGET. No mysteries, no vibes. If you take one habit from me, take this: when a link fails, ask which rule it broke before you touch a single cable.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "The Warehouse pays cores for parts you kill — and on my track the classic corpse is a PoE-starved switch or a switch you cooked by ignoring its budget. A core is a receipt for a lesson. Collect the lesson, skip the receipt where you can.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "One hundred metres. Full bandwidth. Inside the power budget. Three sentences, whole career. Now — the IDF is sixty-five metres from the closet and the desk is sixty-five past that. Do the arithmetic before you touch the bench.",
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
            (
                "tutorial_briefing_gestures",
                &[
                    "Your console here is the subscriber list and the profile catalog. Match each ONT to the service profile its subscriber actually bought — tier, technology, the works — then turn it up. The bench is provisioning, and provisioning punishes assumption.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "My arithmetic is per-subscriber: nine ONTs, nine demanded profiles, and the turn-up only counts when EVERY subscriber verifies against its own demand. One wrong tier isn't a rounding error — it's a customer paying for gigabit and testing at a hundred. The system will tell you which line failed. Systems are honest that way. People improvise.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Clara. Provisioning. On my track you don't route light — the plant is built, the ONTs are registered, and the office has already created the subscribers. Your job is the turn-up: assign the right GPON or XGS profile to every subscriber so the service they bought is the service that verifies.",
                    "Fiber crews move light. I make light mean something. Precision is the whole personality.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "When a profile audit or an outage hits my track, work the list, not your feelings: which subscribers are down, which are merely mis-provisioned, and which were never right in the first place are three different problems wearing one red icon.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "First assignment made. Notice the console verifies the subscriber immediately — profile against demand, line by line. That instant honesty is the difference between provisioning and guessing.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "There's your first verdict: mismatch, named by subscriber. On my levels a failure is never 'somewhere in the network' — it is a specific ONT, a specific profile, a specific wrong assumption. Fix the named thing.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "The Warehouse still pays cores for hardware you kill on my track — a dead-box swap done carelessly is how ONTs become inventory. Provision first, swap second, and the cores you earn will be the honest kind.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Registration IDs issued, profiles demanded, excuses not accepted. Nine subscribers are waiting for their first turn-up. Make each one verify.",
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
            (
                "tutorial_briefing_gestures",
                &[
                    "Your controls are the board itself: click an alarm to acknowledge it, work it, clear it. Acknowledging is a promise, not a dismissal — an acked alarm is an alarm with your name on it. Choose your promises like a professional.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "The math of a NOC is triage arithmetic: severity times scope, divided by time. A critical alarm on one card is a ticket. The same alarm on forty elements is a fiber cut with good PR. Count what's actually down before you decide what's loudest — the cascade always points back to one root that is quieter than its children.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Welcome to the NOC — Aino. My track is the alarm board: everything the network feels, it reports here, all at once, at 3 a.m., in red. Your job is not to fix plant — the crews do that. Your job is to know, faster than anyone, WHAT broke, WHERE, and what can safely wait until morning.",
                    "One alarm on the board tonight. That restraint will not last. Nothing on this board ever stays at one.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "There — the board just lit up. This is a cascade: one root fault, a dozen child alarms, all shouting at once. Do not work them top to bottom. Find the parent — the cut, the card, the power event — acknowledge THAT, and watch half the board explain itself.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "Acknowledged — correctly, and on the root. See how the board settles when the first action is the right one? A NOC runs on exactly that: the discipline to diagnose before motion.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "Your first verdict on my track is a judgment call graded like math: cleared, escalated, or safely deferred. All three are legitimate — at the right time, for the right alarm. The board remembers which you chose and why.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "Cores, NOC edition: when field crews cook hardware on a repair, the dead parts bank as cores at results. Your contribution to that economy is simpler — the faster you name the root, the fewer parts die looking for it.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "The alarms never sleep, so we learn to. Triage calmly, acknowledge honestly, chase roots instead of noise. Your first shift starts now — one alarm, for now.",
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
            (
                "tutorial_briefing_gestures",
                &[
                    "Out here the bench is the last hundred meters: the drop, the slack, the ONT, your meter. Place each run like you're the one who'll re-enter this closure in February, in the rain, with a headlamp dying. Dress your slack. Label your work. Future you is also a person.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "Field math is the fiber budget with mud on it: the feeder is already lit and already spent its dB getting here. What's left at the drop is what you have — measure it at the ONT before you commit to anything. A reading in window is a promise you can keep. A reading you didn't take is a callback you're scheduling.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Hikari — field installations. Séraphine teaches the physics of light; my track is where light meets a customer's actual house. Your job: the drop install, done to standard, measured at the ONT, certified before you leave the driveway.",
                    "My mentor's rule, and now yours: nobody certifies a number they didn't measure. We start there.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "Out here 'outage' has a smell: a dirty drop, a bend in the wall, a connector somebody's thumb lived on. When a reading dies on my track, check the last thing human hands touched before you blame the plant. It's the hands. It's always the hands.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "First run placed. Look at the slack you left and the bend you didn't — that neatness is not vanity, it's the next tech's troubleshooting time, bought in advance.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "There's your first field verdict, and it came from a measurement, not an opinion: in window, or it isn't. If it isn't, the ledger walks the loss back to the exact run that ate it. Field rule: believe the meter, then fix the meter's story.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "Kill a part in the field — cook it, crack it, contaminate it past cleaning — and it banks as a core at results. The Warehouse pays either way, but a clean install record pays better: callbacks are the only debt this job charges interest on.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Fresh drop, lit feeder, one house that wants its internet. Measure at the ONT, certify what you measured, and leave the closure prettier than you found it. Go.",
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
            (
                "tutorial_briefing_gestures",
                &[
                    "The controls are a question, four answers, and your judgment. No bench, no components — the component under test is you. Answer, get the reasoning, and let the reasoning argue back. That argument is where the learning lives.",
                ],
            ),
            (
                "tutorial_briefing_math",
                &[
                    "There is no budget math here — there is something less forgiving: thresholds. Seventy percent to pass, because the field does not grade on a curve and neither does the exam. The Articles are the budget: Article 90 tells you what the Code is FOR, Article 100 gives you the words — and in this trade, the words are load-bearing — and Article 110 tells you how work gets approved and done.",
                ],
            ),
            (
                "tutorial_briefing_objective",
                &[
                    "Léa. Study hall. My track is the theory underneath everyone else's hands: the National Electrical Code as it applies to communications work — grounding, wiring methods, hazardous locations, the law of the state you'll pull permits in.",
                    "The others teach you to do the work. I teach you why the work is legal, safe, and insurable. Both halves are the trade.",
                ],
            ),
            (
                "tutorial_first_outage",
                &[
                    "When a question 'goes down' — when you're sure and the answer says otherwise — that is your outage, and the repair is the same as theirs: find the root. Which definition did you skim? Which exception did you miss? The Code rewards the reader who checks the exception before the rule finishes loading.",
                ],
            ),
            (
                "tutorial_first_placement",
                &[
                    "First answer in. Notice I give you the reasoning either way — a correct guess and a correct understanding score identically today and diverge completely on the exam. We're building the second one.",
                ],
            ),
            (
                "tutorial_first_verdict",
                &[
                    "Your verdicts here are scores, and a score is a map: it shows which Article is load-bearing and which is drywall. Fail a section and the map tells you where to study — that is a kinder diagnostic than any meter the others own.",
                ],
            ),
            (
                "tutorial_scoring_cores",
                &[
                    "No hardware dies in study hall, so cores here are simpler: pass cleanly and the Warehouse honors the scholarship. The real salvage is different — every wrong answer you repair now is a violation you don't install later.",
                ],
            ),
            (
                "tutorial_wrap",
                &[
                    "Articles 90, 100, 110 — purpose, definitions, requirements. Ten questions, seventy percent, no timer but mine. Open the book in your head, and begin.",
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

    const TUTORIAL_KEYS: &[&str] = &[
        "tutorial_briefing_objective",
        "tutorial_briefing_gestures",
        "tutorial_briefing_math",
        "tutorial_first_placement",
        "tutorial_first_verdict",
        "tutorial_first_outage",
        "tutorial_scoring_cores",
        "tutorial_wrap",
    ];

    const ALL_COMPANIONS: &[Companion] = &[
        Companion::Fiber,
        Companion::Coax,
        Companion::Mobile,
        Companion::Ethernet,
        Companion::Clara,
        Companion::Aino,
        Companion::Hikari,
        Companion::Lea,
    ];

    #[test]
    fn all_eight_banks_carry_their_eight_tutorial_keys() {
        // Specialists included explicitly: the completeness loops
        // above iterate Companion::ALL (the four mains only), so
        // specialist tutorial drift would otherwise be silent.
        for companion in ALL_COMPANIONS {
            let bank = DialogueBank::load_default(*companion);
            for key in TUTORIAL_KEYS {
                assert!(
                    bank.lines.get(*key).is_some_and(|l| !l.is_empty()),
                    "{companion:?} is missing tutorial lines for '{key}'"
                );
            }
        }
    }

    #[test]
    fn seraphine_bank_carries_the_field_job_and_splice_keys() {
        const SCENARIO_KEYS: &[&str] = &[
            "fj_coda_cat3",
            "fj_demarc_trap",
            "fj_partial_warning",
            "fj_survey_ports",
            "fj_survey_sb",
            "fj_survey_vfl",
            "fj_swap_done",
            "sp_beat1_clear",
            "sp_beat2_trap",
            "sp_beat3_numbers",
            "sp_chart_handover",
            "sp_damaged_strand",
            "sp_wrap",
        ];
        let bank = DialogueBank::load_default(Companion::Fiber);
        for key in SCENARIO_KEYS {
            assert!(
                bank.lines.get(*key).is_some_and(|l| !l.is_empty()),
                "seraphine is missing scenario lines for '{key}'"
            );
        }
    }

    #[test]
    fn specialist_json_mirrors_match_the_compiled_banks() {
        // The four specialist mirrors were created with the tutorial
        // pass; pin them to the compiled banks like the mains' mirror
        // test does, so they cannot drift silently either.
        for companion in [
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ] {
            let stem = companion.picker_stem();
            let path = format!(
                "{}/assets/dialogue/{stem}_en.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("mirror {path} unreadable: {e}"));
            let mirror: DialogueBank = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("mirror {path} does not parse: {e}"));
            let bank = DialogueBank::load_default(companion);
            assert_eq!(
                mirror.lines, bank.lines,
                "mirror {stem}_en.json drifted from the compiled bank"
            );
        }
    }

    #[test]
    fn random_line_returns_none_for_an_unknown_key() {
        let bank = DialogueBank::load_default(Companion::Fiber);
        assert_eq!(bank.random_line("not_a_real_event"), None);
    }
}
