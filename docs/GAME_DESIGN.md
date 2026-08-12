# Light Show — Game Design Document

## Premise

You're a rookie Outside Plant (OSP) fiber tech at Qompass Networks. Your job: route
light from the Central Office / OLT (Point A) through the outside plant to the
customer ONT (Point B), keeping the received light level inside spec — while
Séraphine, your AI-splice-drone companion, heckles, flirts, and coaches you
through it. Every level is a real fiber-optics problem wearing a puzzle-game
costume.

## Core Loop

1. **Survey the board.** A grid-based OSP map: OLT, splice enclosures, splitters,
   patch panels, ONT, spans of cable (aerial, buried, conduit).
2. **Route the light.** Drag fiber jumpers / choose splice paths to connect A to B.
   Every component you place or select consumes optical budget.
3. **Hit the window.** The received power at B must land inside the design window
   (e.g. −8 dBm to −27 dBm for GPON per ITU-T G.984, tightened per level for
   difficulty). Too hot (too little loss) can be as wrong as too much loss —
   just like real link budgets.
4. **Survive the outage event.** Mid-level, a fault fires (fiber cut, water
   intrusion, bad splice, macrobend, connector contamination). You must
   reroute, patch, or splice around it before the "customer complaint" timer
   expires.
5. **Séraphine reacts.** Dialogue/animation triggers on key actions: good
   splice ("Ara ara, clean 0.02 dB loss~ I like a tech who preps his fusion
   splicer"), bad choice, level clear, outage resolved.

## The Optical Model (real formulas, simplified UI)

We use an additive dB loss budget, matching real OSP link-budget practice:

```
P_rx (dBm) = P_tx (dBm) − Σ(loss contributors, dB) + Σ(gain, dB from amps, rare)
```

Loss contributors modeled as game pieces:

| Component            | Typical loss (dB) | Puzzle behavior |
|-----------------------|-------------------|------------------|
| Fusion splice (good)  | 0.05–0.1          | Cheap, but costs "splice time" resource |
| Mechanical splice     | 0.3–0.5           | Fast, worse loss, used under outage time pressure |
| Connector (UPC)       | 0.3–0.5           | Placed at patch panels |
| Connector (APC)       | 0.3 (angled, low reflectance) | Required for high-bandwidth / PON levels to avoid "reflectance fail" |
| Dirty/contaminated connector | +2 to +5 extra | Random hazard; "clean it" mini-interaction |
| Splitter 1×2          | ~3.5              | Required to reach multiple ONTs (PON levels) |
| Splitter 1×4          | ~7.2              | |
| Splitter 1×8          | ~10.5             | |
| Splitter 1×16         | ~13.5             | |
| Splitter 1×32         | ~17.5             | |
| Fiber span (per km, SMF-28) | 0.35 (1310nm) / 0.25 (1550nm) | Distance slider per span; wavelength choice matters |
| Macrobend (tight coil/staple strike) | 0.5–3+ variable | Outage hazard — visually a kinked line |
| Water intrusion in splice closure | rises over "time" stat | Outage hazard that worsens if ignored |

Distances and losses are pulled onto a running ledger UI (styled like an OTDR
trace) so players visually learn to read a loss budget the way a real tech
reads OTDR output.

### Win condition
`P_rx` within the level's target window AND path is a single continuous
fiber path A→B AND (if present) outage resolved before timer end.

### Fail conditions
- `P_rx` too low (link down, "customer can't stream anime").
- `P_rx` too high / reflectance too high on APC-required segment (receiver
  saturation / return-loss fail).
- Outage timer expires before reroute.
- Physical impossibility (bend radius violation — visualized as a snapped
  fiber if a player forces too tight a corner in the routing grid).

## Difficulty / World Progression

1. **World 1 — Splice School:** single span, single splice, teaches the dB
   ledger and fusion vs mechanical splice tradeoffs.
2. **World 2 — The Patch Panel:** connector types, UPC vs APC, dirty
   connector hazard, cleaning mini-game.
3. **World 3 — PON Split:** splitters, budget math across branches, serving
   multiple ONTs (multi-goal levels: every home must be in-window).
4. **World 4 — Storm Season (Outages):** timed faults — aerial fiber cut by
   fallen branch, buried cable dig-up, splice closure flooding. Player must
   reroute via protection path or emergency splice under time pressure.
5. **World 5 — Long Haul:** wavelength choice (1310 vs 1550 vs 1490 for
   PON upstream/downstream), dispersion flavor-hazard, distance limits.
6. **World 6 — Séraphine's Route (bonus/endless):** procedurally
   generated boards combining all mechanics, leaderboard-style scoring on
   minimal loss margin ("how close to perfect can you thread the light").

## Séraphine — Companion System

- Anime-styled AI hologram that lives in your OTDR tablet. 64×64 pixel-art
  sprite, multiple emote frames (idle, blush, wink, pout, celebrate, alarmed).
- Flirty-but-wholesome dialogue bank keyed to game events (see
  `src/waifu/dialogue.rs` and `assets/dialogue/seraphine_en.json`).
- Gives **optional hints** (costs in-game "favor points" earned by clean
  splices) — never required to solve a level, keeps it skippable/SFW-safe for
  both storefronts.
- No purchasable currency tied to her dialogue — avoids loot-box/gacha
  anti-features so the build stays F-Droid eligible.

## Content & Store Compliance Notes

- No real-money transactions, ads, trackers, or proprietary network calls —
  keeps the build free of F-Droid "anti-features."
- All art/audio original or CC0/CC-BY, tracked in `docs/CREDITS.md`.
- Dialogue reviewed for PEGI 12 / Google Play "Teen" rating: flirtation and
  light innuendo only, no explicit content, no fan-service nudity.
- Educational framing (real dB math, real component names) supports Google
  Play's "Educational" secondary category.
