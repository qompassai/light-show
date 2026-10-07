//! The Warehouse: data catalog and pure economy logic. No Bevy systems
//! live here — the screen is `states::warehouse`.
//!
//! Two hosts run the place: Bianca on the test-bench side (manual facts)
//! and Tessa on the field side (use cases, war stories). The shelf is a
//! list of `ToolDef`s, so more tools slot in as data; v1 ships the Klein
//! Tools VDV Scout Pro 3, every fact taken from its instruction manual
//! (VDV501-851). The shop trades gear for cores (see `Cores`) — the
//! dead parts players haul back from jobs for core credit; the
//! gear's effects reach gameplay through the `Loadout` resource.
//!
//! Everything below is plain data and total functions, so the economy is
//! unit-testable without a running App. Save contents are untrusted
//! input: unknown item ids are ignored, never trusted or panicked on.

use crate::save::SaveData;
use bevy::prelude::Resource;

/// Which host speaks a line. Names live only in `HOSTS` (one-line
/// rename when the placeholders change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostId {
    Bianca,
    Tessa,
}

/// A Warehouse host: who she is, what she looks like (for the art
/// pipeline and the fallback panel), and how she greets the player.
#[derive(Debug)]
pub struct Host {
    pub id: HostId,
    pub name: &'static str,
    /// Asset path under `assets/`; the screen tolerates it missing.
    pub portrait: &'static str,
    pub look: &'static str,
    pub side: &'static str,
    pub greeting: &'static str,
}

pub const HOSTS: [Host; 2] = [
    Host {
        id: HostId::Bianca,
        name: "Bianca",
        portrait: "art/bianca/companions/bianca_portrait.jpg",
        look: "Tall. Long black hair in a blunt hime cut, grey eyes, beauty mark under \
               the left eye. Black tee: \"COME AT ME PRINCESS\".",
        side: "Test bench",
        greeting: "Welcome to the Warehouse. Bring me your dead parts — every core you \
                   haul in gets inspected, graded, and credited, no exceptions. Everything \
                   on this shelf comes with a manual, and yes, I have read all of them. \
                   Twice. Like a good isekai protagonist reads the system menu.",
    },
    Host {
        id: HostId::Tessa,
        name: "Tessa",
        portrait: "art/tessa/companions/tessa_portrait.jpg",
        look: "Short. Brown ponytail, cowgirl boots, jeans with more holes than a \
               punch-down block.",
        side: "Field",
        greeting: "Howdy, partner. Bianca'll tell you what the book says. I'll tell you when \
                   you actually reach for it, usually at the bottom of a crawlspace — which \
                   is also where I dig out half the dead parts you'll be trading us.",
    },
];

/// The host record for `id`. Total: every `HostId` has exactly one entry.
pub fn host(id: HostId) -> &'static Host {
    let host = match id {
        HostId::Bianca => &HOSTS[0],
        HostId::Tessa => &HOSTS[1],
    };
    assert_eq!(host.id, id, "HOSTS order must match HostId");
    host
}

/// The Warehouse backdrop pool: generated interiors with Bianca and
/// Tessa IN the artwork — the hosts are never composited portrait
/// cards. The scene rotates through the pool with a shuffle bag
/// (`BackdropBag`): a different backdrop each visit, no repeat until
/// the pool cycles. Paths are relative to the game asset root; the
/// index into this table is the rotation's currency (save fields,
/// `BackdropBag`, the scene), so entries are never reordered, only
/// appended.
pub const BACKDROPS: [&str; 5] = [
    "art/warehouse_backdrop_1.jpg",
    "art/warehouse_backdrop_2.jpg",
    "art/warehouse_backdrop_3.jpg",
    "art/warehouse_backdrop_4.jpg",
    "art/warehouse_backdrop_5.jpg",
];

/// Shuffle-bag rotation over `BACKDROPS`. A Bevy resource on the
/// screen side: seeded from the save at startup, advanced once per
/// Warehouse entry, and mirrored back into the save by the screen so
/// the rotation survives restarts.
///
/// Invariants: `remaining` holds each pool index at most once, all
/// `< BACKDROPS.len()`; `current` is always a valid index. Every
/// block of `BACKDROPS.len()` consecutive draws is a permutation of
/// the pool, and two consecutive draws never match — not even across
/// a reshuffle, because a refill's first draw is swapped away from
/// the outgoing `current`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Resource)]
pub struct BackdropBag {
    /// Indices not yet shown this cycle; drawn from the end.
    remaining: Vec<u8>,
    /// Backdrop on screen now (index into `BACKDROPS`).
    current: u8,
}

impl BackdropBag {
    /// From persisted fields. The save is untrusted input: unknown
    /// indices are dropped, duplicates collapse, and an out-of-range
    /// current resets to the first backdrop.
    pub fn from_save(save: &SaveData) -> Self {
        let count = BACKDROPS.len() as u8;
        let mut seen = std::collections::HashSet::new();
        let remaining: Vec<u8> = save
            .warehouse_backdrop_bag
            .iter()
            .copied()
            .filter(|&idx| idx < count && seen.insert(idx))
            .collect();
        let current = if save.warehouse_backdrop_current < count {
            save.warehouse_backdrop_current
        } else {
            0
        };
        Self { remaining, current }
    }

    /// Persisted form: (bag in bagged order, current index).
    pub fn to_save(&self) -> (Vec<u8>, u8) {
        (self.remaining.clone(), self.current)
    }

    /// Backdrop on screen now (index into `BACKDROPS`).
    pub fn current(&self) -> u8 {
        self.current
    }

    /// Asset path of the backdrop on screen now.
    pub fn current_path(&self) -> &'static str {
        BACKDROPS[self.current as usize]
    }

    /// Move to the next backdrop and return its index, refilling and
    /// reshuffling the bag when it runs dry. `seed` only stirs the
    /// shuffle; the no-repeat invariants hold for every seed.
    pub fn advance(&mut self, seed: u64) -> u8 {
        if self.remaining.is_empty() {
            self.remaining = (0..BACKDROPS.len() as u8).collect();
            shuffle_backdrops(&mut self.remaining, seed);
            // Never open a fresh bag with the outgoing backdrop: a
            // different scene every visit, cycle boundary or not.
            if self.remaining.len() > 1 && self.remaining.last() == Some(&self.current) {
                let last = self.remaining.len() - 1;
                self.remaining.swap(0, last);
            }
        }
        self.current = self.remaining.pop().expect("bag refilled above");
        assert!((self.current as usize) < BACKDROPS.len());
        self.current
    }
}

/// Fisher–Yates stirred by a xorshift64* PRNG: tiny, deterministic
/// per seed (the rotation tests pin behavior per seed), and more
/// than enough entropy for dealing backdrops.
fn shuffle_backdrops(indices: &mut [u8], seed: u64) {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    if state == 0 {
        state = 0x2545_F491_4F6C_DD1D;
    }
    for i in (1..indices.len()).rev() {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let roll = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let j = (roll % (i as u64 + 1)) as usize;
        indices.swap(i, j);
    }
}

/// One multiple-choice question with both host reactions. `source` is
/// the manual section that validates the answer.
#[derive(Debug)]
pub struct QuizQuestion {
    pub question: &'static str,
    pub choices: [&'static str; 4],
    /// Index into `choices`; always < 4 (checked by test).
    pub correct_idx: usize,
    pub source: &'static str,
    /// Bianca, on a right answer: the manual fact.
    pub explain_correct: &'static str,
    /// Tessa, on a wrong answer: the field consequence, then the answer.
    pub explain_wrong: &'static str,
}

/// A tool on the shelf: what it is, what it does, when you reach for it,
/// and the quiz that proves you were listening.
#[derive(Debug)]
pub struct ToolDef {
    /// Stable id persisted in `SaveData::warehouse_quiz_passed`.
    pub id: &'static str,
    pub name: &'static str,
    pub maker: &'static str,
    pub tagline: &'static str,
    pub summary: &'static str,
    pub functions: &'static [&'static str],
    pub use_cases: &'static [&'static str],
    pub quiz: &'static [QuizQuestion],
}

/// The shelf. Order is display order.
pub const TOOLS: &[ToolDef] = &[SCOUT_PRO_3];

const SCOUT_PRO_3: ToolDef = ToolDef {
    id: "scout_pro_3",
    name: "VDV Scout Pro 3",
    maker: "Klein Tools (VDV501-851)",
    tagline: "Voice, data, and video cable tester.",
    summary: "Portable VDV cable tester for RJ11, RJ12, RJ45 and F-connector terminated \
              cables. Measures cable length, tests for shield, performs hub blink, and \
              traces up to 19 locations (5 with the included remotes). Length method: \
              capacitance, 1.5 ft to 1,999 ft (0.5-610 m) at 15 pF/ft, accurate to \
              +/-5%. A 9V battery gives about 50 hours active or 4 years standby. Auto \
              power-off after 5 minutes (60 minutes in Tone mode).",
    functions: &[
        "Wiremap tests on voice (RJ11/RJ12) and data (RJ45) cables.",
        "Continuity test on F-terminated coax.",
        "Detects short faults, open faults, reversals, miswires, crossover wiring, \
         and split pairs.",
        "Cable ID and location mapping with Test + Map remotes. The self-storing \
         remote is always ID #1.",
        "Length measurement by capacitance, with an editable length constant \
         (10-40 pF/ft; defaults: voice 17.0, data 15.0, video 15.0 pF/ft).",
        "Tone generator: solid 800/1000/1200/1400/1500 Hz plus alternating pairs, \
         for tracing with an analog probe.",
        "Hub Blink (long-press Tone) blinks the switch port LED so you can find the port.",
        "PoE test (long-press Length/PoE) detects PoE and shows voltage and pair \
         configuration.",
        "Voltage safety check before every test: if voltage is found, no test runs \
         and the lightning-bolt warning shows.",
        "SAFETY: built for UNENERGIZED cabling only. Live AC can damage it and endanger \
         you. Never use Hub Blink on an active PoE port.",
    ],
    use_cases: &[
        "New Cat6 drop won't link? Wiremap tells you if it's an open, a short, a \
         miswire or split pairs before you go re-terminating blind.",
        "Wall of unlabeled patch-panel ports? The ID remotes tell you which room each \
         port lands in, up to 19 locations.",
        "\"Enough cable left on this spool? How far to the break?\" Length mode measures \
         the run, and an open pair's length tells you roughly where the fault is.",
        "One cable in a bundle of fifty: tone it and wave the probe. The toned cable \
         is the loudest one, plain as a dinner bell.",
        "Dead switch port and nobody labeled the closet? Hub Blink flashes the port LED.",
        "Before you plug a camera into a PoE run, the PoE test confirms power is there \
         and which pairs carry it.",
    ],
    quiz: SCOUT_PRO_3_QUIZ,
};

const SCOUT_PRO_3_QUIZ: &[QuizQuestion] = &[
    QuizQuestion {
        question: "The Scout Pro 3 measures cable length using which property?",
        choices: [
            "Resistance",
            "Capacitance",
            "Time-domain reflectometry",
            "Optical loss",
        ],
        correct_idx: 1,
        source: "General Specifications: Length Measurement Method",
        explain_correct: "Correct. General Specifications: \"Length Measurement Method: \
                          Capacitance.\" That's why the length constant is in pF per foot.",
        explain_wrong: "That'd be a fancier box, hon. This one reads capacitance. Wrong \
                        pF/ft setting and your 'two hundred foot' run is a fairy tale. \
                        The answer's capacitance.",
    },
    QuizQuestion {
        question: "A cable tests with pairs landed in the right order per-pin, but the \
                   pairs themselves aren't kept twisted together. The tester shows...",
        choices: ["Open", "Short", "Split pairs", "Reversal"],
        correct_idx: 2,
        source: "Display: Cable Faults (\"Split\")",
        explain_correct: "Correct. Display, Cable Faults: \"Split\" means the wire pairs \
                          are not maintained as pairs. Pin order alone proves nothing.",
        explain_wrong: "Split pairs pass a pin-checker and still fail at gigabit. That's \
                        why this tester exists. The answer's split pairs.",
    },
    QuizQuestion {
        question: "What is the default length constant for data cable?",
        choices: ["17.0 pF/ft", "10.0 pF/ft", "40.0 pF/ft", "15.0 pF/ft"],
        correct_idx: 3,
        source: "Length Constant: defaults",
        explain_correct: "Correct. Length Constant defaults: voice 17.0, data 15.0, \
                          video 15.0 pF/ft. Editable from 10 to 40.",
        explain_wrong: "Leave the wrong constant in and every length you quote is off, \
                        and the boss quotes the customer off your number. Data's 15.0 \
                        pF/ft.",
    },
    QuizQuestion {
        question: "Before every test the Scout checks for voltage. If voltage is found...",
        choices: [
            "No test runs; disconnect immediately",
            "The test proceeds at reduced accuracy",
            "It switches to PoE mode",
            "It beeps once and continues",
        ],
        correct_idx: 0,
        source: "Display: Voltage Check",
        explain_correct: "Correct. Display, Voltage Check: with voltage present no test \
                          runs and the lightning bolt shows. Disconnect. Do not argue \
                          with the bolt.",
        explain_wrong: "That little lightning bolt ain't decoration, partner. Voltage \
                        found means no test runs. Unplug it right now.",
    },
    QuizQuestion {
        question: "Hub Blink is forbidden in which situation?",
        choices: [
            "On shielded cable",
            "On runs over 100 ft",
            "When connected to an active PoE port",
            "During tone mode",
        ],
        correct_idx: 2,
        source: "Keypad: Tone/Hub Blink",
        explain_correct: "Correct. Keypad, Tone/Hub Blink: \"DO NOT attempt to use Hub \
                          Blink when connected to a PoE active port.\" Capital letters \
                          theirs, not mine.",
        explain_wrong: "Blink a hot PoE port and you're filling out a tool-loss form. \
                        The rule is never on an active PoE port.",
    },
    QuizQuestion {
        question: "Tone frequencies on the Scout Pro 3 include which solid tone?",
        choices: ["440 Hz", "1000 Hz", "60 Hz", "2000 Hz"],
        correct_idx: 1,
        source: "Tone Generation: solid tones",
        explain_correct: "Correct. Tone Generation: solid tones at 800, 1000, 1200, 1400 \
                          and 1500 Hz, plus alternating pairs.",
        explain_wrong: "Pick a tone the probe can't pull out of the hum and you'll be \
                        waving it at that bundle till sundown. 1000 Hz is one of the \
                        solids.",
    },
    QuizQuestion {
        question: "The self-storing Test + Map remote always shows as which ID?",
        choices: [
            "Remote ID #0",
            "Remote ID #1",
            "Remote ID #19",
            "It has no ID",
        ],
        correct_idx: 1,
        source: "Self-Storing Test + Map ID Remote",
        explain_correct: "Correct. The self-storing Test + Map remote is always ID #1. \
                          The protagonist is always number one.",
        explain_wrong: "Mix up which remote's which and you'll label half a patch panel \
                        to the wrong rooms. The self-storing one is always #1.",
    },
    QuizQuestion {
        question: "Is it safe to connect the Scout Pro 3 to a live AC-powered cable run?",
        choices: [
            "Yes, under 55V",
            "Yes, with the backlight off",
            "Yes, in ID mode",
            "Never: it's designed for unenergized cabling only",
        ],
        correct_idx: 3,
        source: "Warnings",
        explain_correct: "Correct. Warnings: designed for unenergized cabling only. Live \
                          AC may damage the tester and endanger the user.",
        explain_wrong: "Live AC'll cook the tester, and maybe you with it. Never. It's \
                        built for unenergized cabling only.",
    },
];

/// The shelf tool with `id`, if it exists.
pub fn tool(id: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.id == id)
}

/// Who presents a Learn page and under which heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnLine {
    pub host: HostId,
    pub heading: &'static str,
    pub text: &'static str,
}

/// The Learn walkthrough for `tool` as host dialogue: Bianca presents what
/// it is and what it does (manual facts), Tessa presents when you reach
/// for it. Length is `2 + functions + use_cases` (summary + tagline page
/// first, then one page per entry).
pub fn learn_lines(tool: &ToolDef) -> Vec<LearnLine> {
    let mut lines = Vec::with_capacity(2 + tool.functions.len() + tool.use_cases.len());
    lines.push(LearnLine {
        host: HostId::Bianca,
        heading: "WHAT IT IS",
        text: tool.tagline,
    });
    lines.push(LearnLine {
        host: HostId::Bianca,
        heading: "WHAT IT IS",
        text: tool.summary,
    });
    for text in tool.functions {
        lines.push(LearnLine {
            host: HostId::Bianca,
            heading: "WHAT IT DOES",
            text,
        });
    }
    for text in tool.use_cases {
        lines.push(LearnLine {
            host: HostId::Tessa,
            heading: "WHEN YOU REACH FOR IT",
            text,
        });
    }
    lines
}

// ---------------------------------------------------------------------------
// Shop
// ---------------------------------------------------------------------------

/// Permanent gear is bought once; consumables stack and are used up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Permanent,
    Consumable,
}

/// One shop listing. `host` pitches it: Bianca for bench gear, Tessa for
/// field gear.
#[derive(Debug)]
pub struct ShopItem {
    /// Stable id persisted in `SaveData::owned_gear` / `consumables`.
    pub id: &'static str,
    pub name: &'static str,
    pub price: u32,
    pub kind: ItemKind,
    /// 1 = always for sale; 2 = locked until `TIER2_UNLOCK_GEAR_COUNT`
    /// permanents are owned.
    pub tier: u8,
    pub blurb: &'static str,
    pub host: HostId,
    pub pitch: &'static str,
}

pub const HEADLAMP: &str = "headlamp";
pub const SPARE_BATTERY: &str = "spare_battery";
pub const GOLDEN_CRIMPER: &str = "golden_crimper";
pub const FIELD_COFFEE: &str = "field_coffee";
pub const SPARE_REMOTES: &str = "spare_remotes";

/// The stock, in display order.
pub const SHOP: &[ShopItem] = &[
    ShopItem {
        id: HEADLAMP,
        name: "Headlamp",
        price: 30,
        kind: ItemKind::Permanent,
        tier: 1,
        blurb: "Hints cost no cores.",
        host: HostId::Tessa,
        pitch: "Can't fix what you can't see. Strap this on and the answers come to you \
                free of charge.",
    },
    ShopItem {
        id: SPARE_BATTERY,
        name: "Spare 9V Battery",
        price: 40,
        kind: ItemKind::Permanent,
        tier: 1,
        blurb: "Outage repair timer x1.25.",
        host: HostId::Bianca,
        pitch: "Fifty hours active, four years standby, per the manual. A fresh cell \
                buys you time when the network goes down.",
    },
    ShopItem {
        id: GOLDEN_CRIMPER,
        name: "Golden Crimper",
        price: 80,
        kind: ItemKind::Permanent,
        tier: 2,
        blurb: "Cores from wins x1.5 (rounded up).",
        host: HostId::Bianca,
        pitch: "Legendary-rarity gear. Every termination it makes is a clean one, and \
                clean work gets noticed.",
    },
    ShopItem {
        id: FIELD_COFFEE,
        name: "Field Coffee",
        price: 5,
        kind: ItemKind::Consumable,
        tier: 1,
        blurb: "Next outage timer +15 s. Used up when an outage starts.",
        host: HostId::Tessa,
        pitch: "Gas-station coffee, black as a dark fiber. Fifteen extra seconds of \
                pure hustle.",
    },
    ShopItem {
        id: SPARE_REMOTES,
        name: "Spare Remotes",
        price: 8,
        kind: ItemKind::Consumable,
        tier: 1,
        blurb: "One wrong quiz answer is forgiven. Used up on use.",
        host: HostId::Bianca,
        pitch: "A second set of ID remotes. If you mislabel one, I'll pretend I didn't \
                see it. Once.",
    },
];

/// Permanents owned before tier-2 stock unlocks.
pub const TIER2_UNLOCK_GEAR_COUNT: usize = 2;

/// Most of one consumable a player can carry. Bounds save growth.
pub const CONSUMABLE_STACK_MAX: usize = 99;

/// The shop listing with `id`, if it exists.
pub fn item(id: &str) -> Option<&'static ShopItem> {
    SHOP.iter().find(|i| i.id == id)
}

/// True when the player can pay `price` from `balance`.
pub fn can_afford(balance: u32, price: u32) -> bool {
    balance >= price
}

/// Count of distinct known permanents in `owned_gear`. Duplicates and
/// unknown ids (a hand-edited save) do not count toward unlocks.
pub fn owned_gear_count(owned_gear: &[String]) -> usize {
    SHOP.iter()
        .filter(|i| i.kind == ItemKind::Permanent && owns(owned_gear, i.id))
        .count()
}

/// True when tier-2 stock is for sale: at least
/// `TIER2_UNLOCK_GEAR_COUNT` distinct permanents owned.
pub fn tier2_unlocked(owned_gear: &[String]) -> bool {
    owned_gear_count(owned_gear) >= TIER2_UNLOCK_GEAR_COUNT
}

/// True when `id` is in `list`.
pub fn owns(list: &[String], id: &str) -> bool {
    list.iter().any(|x| x == id)
}

/// How many of consumable `id` are in `consumables`.
pub fn count(consumables: &[String], id: &str) -> usize {
    consumables.iter().filter(|x| *x == id).count()
}

/// Remove one `id` from `consumables`. Returns false (and changes
/// nothing) when there is none to use.
pub fn consume_one(consumables: &mut Vec<String>, id: &str) -> bool {
    match consumables.iter().position(|x| x == id) {
        Some(at) => {
            consumables.remove(at);
            true
        }
        None => false,
    }
}

/// Why a purchase was refused. Refusal leaves all state untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseError {
    UnknownItem,
    AlreadyOwned,
    Locked,
    CannotAfford { price: u32, balance: u32 },
    StackFull,
}

impl std::fmt::Display for PurchaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PurchaseError::UnknownItem => write!(f, "That isn't on the shelf."),
            PurchaseError::AlreadyOwned => write!(f, "You already own that."),
            PurchaseError::Locked => write!(
                f,
                "Locked: own {TIER2_UNLOCK_GEAR_COUNT} pieces of gear first."
            ),
            PurchaseError::CannotAfford { price, balance } => {
                write!(f, "Costs {price} cores; you have {balance}.")
            }
            PurchaseError::StackFull => {
                write!(f, "Your pack is full ({CONSUMABLE_STACK_MAX} max).")
            }
        }
    }
}

/// The state after a successful purchase. The caller commits all three
/// fields together (to `Cores` and `SaveData`) and persists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    pub balance: u32,
    pub owned_gear: Vec<String>,
    pub consumables: Vec<String>,
}

/// Validate and prepare buying `item_id`. Pure: on `Ok` returns the new
/// balance and inventories; on `Err` nothing has changed. Checks run in
/// the order a player would want explained: exists, owned, locked,
/// affordable, carry limit.
pub fn purchase(
    item_id: &str,
    balance: u32,
    owned_gear: &[String],
    consumables: &[String],
) -> Result<Purchase, PurchaseError> {
    let item = item(item_id).ok_or(PurchaseError::UnknownItem)?;
    if item.kind == ItemKind::Permanent && owns(owned_gear, item.id) {
        return Err(PurchaseError::AlreadyOwned);
    }
    if item.tier >= 2 && !tier2_unlocked(owned_gear) {
        return Err(PurchaseError::Locked);
    }
    if !can_afford(balance, item.price) {
        return Err(PurchaseError::CannotAfford {
            price: item.price,
            balance,
        });
    }
    if item.kind == ItemKind::Consumable && count(consumables, item.id) >= CONSUMABLE_STACK_MAX {
        return Err(PurchaseError::StackFull);
    }
    let mut purchase = Purchase {
        balance: balance - item.price,
        owned_gear: owned_gear.to_vec(),
        consumables: consumables.to_vec(),
    };
    match item.kind {
        ItemKind::Permanent => purchase.owned_gear.push(item.id.to_owned()),
        ItemKind::Consumable => purchase.consumables.push(item.id.to_owned()),
    }
    Ok(purchase)
}

// ---------------------------------------------------------------------------
// Core payouts
// ---------------------------------------------------------------------------

/// Pass threshold for a tool quiz: correct/total >= 7/8.
pub const QUIZ_PASS_NUMERATOR: usize = 7;
pub const QUIZ_PASS_DENOMINATOR: usize = 8;
/// Cores for passing a tool's quiz the first time.
pub const QUIZ_FIRST_PASS_CORES: u32 = 15;
/// Cores for a perfect score on a tool already passed.
pub const QUIZ_PERFECT_REPLAY_CORES: u32 = 3;
/// Extra cores the first time a level is cleared.
pub const FIRST_CLEAR_CORES: u32 = 10;

/// True when `correct` of `total` meets the 7/8 bar. An empty quiz
/// never passes (an authoring bug, not a win).
pub fn quiz_passed(correct: usize, total: usize) -> bool {
    total > 0 && correct <= total && correct * QUIZ_PASS_DENOMINATOR >= total * QUIZ_PASS_NUMERATOR
}

/// What finishing a tool quiz pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuizReward {
    pub cores: u32,
    /// True when this run should record the tool as passed.
    pub first_pass: bool,
}

/// Payout for finishing a tool quiz: first pass +15; a perfect replay of
/// an already-passed tool +3; anything else nothing.
pub fn quiz_payout(correct: usize, total: usize, already_passed: bool) -> QuizReward {
    let passed = quiz_passed(correct, total);
    let cores = match (passed, already_passed) {
        (true, false) => QUIZ_FIRST_PASS_CORES,
        (true, true) if correct == total => QUIZ_PERFECT_REPLAY_CORES,
        _ => 0,
    };
    QuizReward {
        cores,
        first_pass: passed && !already_passed,
    }
}

/// `base` scaled by `mult`, rounded up. A non-finite or sub-1.0
/// multiplier is treated as 1.0: gear can only ever add cores.
pub fn scale_cores(base: u32, mult: f32) -> u32 {
    if !mult.is_finite() || mult <= 1.0 {
        return base;
    }
    // `as u32` saturates on overflow, so a huge product cannot wrap.
    (base as f32 * mult).ceil() as u32
}

/// Cores for a level win: `base_per_win`, plus `FIRST_CLEAR_CORES` on a
/// first clear, all scaled by `cores_mult`.
pub fn win_cores(base_per_win: u32, first_clear: bool, cores_mult: f32) -> u32 {
    let bonus = if first_clear { FIRST_CLEAR_CORES } else { 0 };
    scale_cores(base_per_win.saturating_add(bonus), cores_mult)
}

// ---------------------------------------------------------------------------
// Loadout: gear effects as gameplay sees them
// ---------------------------------------------------------------------------

/// Spare battery: outage repair timer multiplier.
pub const SPARE_BATTERY_TIMER_MULT: f32 = 1.25;
/// Field coffee: seconds added to the next outage timer.
pub const FIELD_COFFEE_BONUS_SECS: f32 = 15.0;
/// Golden crimper: core multiplier on wins.
pub const GOLDEN_CRIMPER_CORES_MULT: f32 = 1.5;

/// Gear effects derived from the save. Recomputed (never edited by hand)
/// whenever the save's gear changes: on purchase, at level start, and at
/// outage start (`states::outage`, which also consumes field coffee).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Loadout {
    /// Multiplies the outage kind's base repair timer (>= 1.0).
    pub outage_timer_mult: f32,
    /// Seconds added to the next outage timer (from field coffee).
    pub outage_timer_bonus_secs: f32,
    /// Hints cost no cores. No hint purchase path exists in the game yet
    /// (`Cores` is only ever earned), so this is recorded for the
    /// future hint system and read by nothing today.
    pub hint_free: bool,
    /// Multiplies cores from level wins (>= 1.0).
    pub cores_mult: f32,
}

impl Default for Loadout {
    fn default() -> Self {
        Self {
            outage_timer_mult: 1.0,
            outage_timer_bonus_secs: 0.0,
            hint_free: false,
            cores_mult: 1.0,
        }
    }
}

impl Loadout {
    /// Effects of `owned_gear` and `consumables`. Unknown ids are ignored.
    pub fn from_gear(owned_gear: &[String], consumables: &[String]) -> Self {
        let mut loadout = Self::default();
        if owns(owned_gear, HEADLAMP) {
            loadout.hint_free = true;
        }
        if owns(owned_gear, SPARE_BATTERY) {
            loadout.outage_timer_mult = SPARE_BATTERY_TIMER_MULT;
        }
        if owns(owned_gear, GOLDEN_CRIMPER) {
            loadout.cores_mult = GOLDEN_CRIMPER_CORES_MULT;
        }
        // One coffee per outage, however many are in the pack.
        if owns(consumables, FIELD_COFFEE) {
            loadout.outage_timer_bonus_secs = FIELD_COFFEE_BONUS_SECS;
        }
        loadout
    }

    /// Effects of the gear in `save`.
    pub fn from_save(save: &SaveData) -> Self {
        Self::from_gear(&save.owned_gear, &save.consumables)
    }

    /// Seconds to add to an outage whose base timer is `base_timer_secs`:
    /// the multiplier's share plus the flat bonus. Never negative.
    pub fn outage_extra_secs(&self, base_timer_secs: f64) -> f64 {
        let mult_share = base_timer_secs * (f64::from(self.outage_timer_mult) - 1.0);
        let extra = mult_share + f64::from(self.outage_timer_bonus_secs);
        if extra.is_finite() {
            extra.max(0.0)
        } else {
            0.0
        }
    }
}


/// Warehouse-enter reaction lines for a visit (spec: hosts react to
/// the haul the player brings in). Pure: `haul` is the pending salvage
/// haul claimed on entry, `None` on an ordinary visit. Bianca inspects
/// and names the parts; Tessa's tease scales with the haul size.
pub fn enter_reaction(haul: Option<&crate::salvage::PendingHaul>) -> Vec<(HostId, String)> {
    let Some(haul) = haul.filter(|h| h.count > 0) else {
        return vec![(
            HostId::Bianca,
            "Warehouse is open. Shelf's stocked, bench is warm.".to_string(),
        )];
    };
    let n = haul.count as usize;
    let tease = match n {
        1 => "One core. A start. Try not to cook the rest of the plant, partner.",
        2..=3 => "Now that's a haul. Somebody's route ran hot out there.",
        _ => "Well now. That route didn't run hot, it ran a furnace. Respect.",
    };
    vec![
        (
            HostId::Bianca,
            format!(
                "Let me see the bag... {}. Graded: {n} core{} credited. Dead parts in, good gear out.",
                haul.summary,
                if n == 1 { "" } else { "s" }
            ),
        ),
        (HostId::Tessa, tease.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    // ---- Catalog integrity ----

    #[test]
    fn scout_quiz_has_the_eight_spec_questions_with_valid_answers() {
        let scout = tool("scout_pro_3").expect("Scout Pro 3 is on the shelf");
        assert_eq!(scout.quiz.len(), 8);
        let answers: Vec<&str> = scout
            .quiz
            .iter()
            .map(|q| q.choices[q.correct_idx])
            .collect();
        assert_eq!(
            answers,
            [
                "Capacitance",
                "Split pairs",
                "15.0 pF/ft",
                "No test runs; disconnect immediately",
                "When connected to an active PoE port",
                "1000 Hz",
                "Remote ID #1",
                "Never: it's designed for unenergized cabling only",
            ]
        );
        for q in scout.quiz {
            assert!(q.correct_idx < 4);
            assert!(!q.source.is_empty() && !q.explain_correct.is_empty());
            assert!(!q.explain_wrong.is_empty());
        }
    }

    #[test]
    fn correct_answers_are_not_all_in_the_same_slot() {
        // A quiz where "always pick B" wins teaches nothing.
        let scout = tool("scout_pro_3").unwrap();
        let first = scout.quiz[0].correct_idx;
        assert!(scout.quiz.iter().any(|q| q.correct_idx != first));
    }

    #[test]
    fn ids_are_unique_and_lookups_reject_unknowns() {
        for (i, a) in SHOP.iter().enumerate() {
            assert!(SHOP[i + 1..].iter().all(|b| b.id != a.id));
        }
        assert!(item("headlamp").is_some());
        assert!(item("Headlamp").is_none());
        assert!(item("").is_none());
        assert!(tool("scout_pro_4").is_none());
    }

    #[test]
    fn shop_matches_the_spec_prices_kinds_and_tiers() {
        let spec = [
            (HEADLAMP, 30, ItemKind::Permanent, 1),
            (SPARE_BATTERY, 40, ItemKind::Permanent, 1),
            (GOLDEN_CRIMPER, 80, ItemKind::Permanent, 2),
            (FIELD_COFFEE, 5, ItemKind::Consumable, 1),
            (SPARE_REMOTES, 8, ItemKind::Consumable, 1),
        ];
        assert_eq!(SHOP.len(), spec.len());
        for (id, price, kind, tier) in spec {
            let it = item(id).unwrap();
            assert_eq!((it.price, it.kind, it.tier), (price, kind, tier), "{id}");
        }
    }

    #[test]
    fn host_lookup_matches_ids() {
        assert_eq!(host(HostId::Bianca).name, "Bianca");
        assert_eq!(host(HostId::Tessa).name, "Tessa");
    }

    #[test]
    fn learn_lines_cover_every_section_with_the_right_host() {
        let scout = tool("scout_pro_3").unwrap();
        let lines = learn_lines(scout);
        assert_eq!(
            lines.len(),
            2 + scout.functions.len() + scout.use_cases.len()
        );
        assert_eq!(lines[0].host, HostId::Bianca);
        assert!(lines
            .iter()
            .filter(|l| l.heading == "WHEN YOU REACH FOR IT")
            .all(|l| l.host == HostId::Tessa));
        assert_eq!(lines.last().unwrap().host, HostId::Tessa);
    }

    // ---- Purchases ----

    #[test]
    fn can_afford_is_inclusive() {
        assert!(can_afford(30, 30));
        assert!(!can_afford(29, 30));
        assert!(can_afford(0, 0));
    }

    #[test]
    fn buying_a_permanent_debits_and_records_it() {
        let p = purchase(HEADLAMP, 50, &[], &[]).unwrap();
        assert_eq!(p.balance, 20);
        assert_eq!(p.owned_gear, ids(&[HEADLAMP]));
        assert!(p.consumables.is_empty());
    }

    #[test]
    fn consumables_stack() {
        let p = purchase(FIELD_COFFEE, 20, &[], &ids(&[FIELD_COFFEE])).unwrap();
        assert_eq!(p.balance, 15);
        assert_eq!(count(&p.consumables, FIELD_COFFEE), 2);
    }

    #[test]
    fn rebuying_a_permanent_is_refused() {
        let owned = ids(&[HEADLAMP]);
        assert_eq!(
            purchase(HEADLAMP, 999, &owned, &[]),
            Err(PurchaseError::AlreadyOwned)
        );
    }

    #[test]
    fn unaffordable_is_refused_without_going_negative() {
        assert_eq!(
            purchase(SPARE_BATTERY, 39, &[], &[]),
            Err(PurchaseError::CannotAfford {
                price: 40,
                balance: 39
            })
        );
        assert!(purchase(FIELD_COFFEE, 0, &[], &[]).is_err());
    }

    #[test]
    fn unknown_or_hostile_ids_are_refused() {
        for id in [
            "",
            "HEADLAMP",
            "headlamp ",
            "../save.json",
            "golden_crimper\0",
        ] {
            assert_eq!(
                purchase(id, u32::MAX, &[], &[]),
                Err(PurchaseError::UnknownItem),
                "{id:?}"
            );
        }
    }

    #[test]
    fn tier2_unlocks_at_two_distinct_known_permanents() {
        assert!(!tier2_unlocked(&ids(&[HEADLAMP])));
        assert!(tier2_unlocked(&ids(&[HEADLAMP, SPARE_BATTERY])));
        // Duplicates, unknown ids, and consumables in the gear list (a
        // hand-edited save) must not unlock tier 2.
        assert!(!tier2_unlocked(&ids(&[HEADLAMP, HEADLAMP])));
        assert!(!tier2_unlocked(&ids(&[HEADLAMP, "laser_sword"])));
        assert!(!tier2_unlocked(&ids(&[HEADLAMP, FIELD_COFFEE])));
    }

    #[test]
    fn locked_tier2_is_refused_even_when_rich() {
        assert_eq!(
            purchase(GOLDEN_CRIMPER, 1000, &ids(&[HEADLAMP]), &[]),
            Err(PurchaseError::Locked)
        );
        let owned = ids(&[HEADLAMP, SPARE_BATTERY]);
        let p = purchase(GOLDEN_CRIMPER, 80, &owned, &[]).unwrap();
        assert_eq!(p.balance, 0);
        assert!(owns(&p.owned_gear, GOLDEN_CRIMPER));
    }

    #[test]
    fn consumable_stack_is_bounded() {
        let full = vec![FIELD_COFFEE.to_owned(); CONSUMABLE_STACK_MAX];
        assert_eq!(
            purchase(FIELD_COFFEE, 1000, &[], &full),
            Err(PurchaseError::StackFull)
        );
    }

    #[test]
    fn consume_one_removes_exactly_one_and_refuses_when_empty() {
        let mut pack = ids(&[FIELD_COFFEE, SPARE_REMOTES, FIELD_COFFEE]);
        assert!(consume_one(&mut pack, FIELD_COFFEE));
        assert_eq!(count(&pack, FIELD_COFFEE), 1);
        assert_eq!(count(&pack, SPARE_REMOTES), 1);
        assert!(consume_one(&mut pack, FIELD_COFFEE));
        assert!(!consume_one(&mut pack, FIELD_COFFEE));
        assert_eq!(pack, ids(&[SPARE_REMOTES]));
    }

    // ---- Payouts ----

    #[test]
    fn quiz_pass_bar_is_seven_of_eight() {
        assert!(quiz_passed(8, 8));
        assert!(quiz_passed(7, 8));
        assert!(!quiz_passed(6, 8));
        assert!(!quiz_passed(0, 0));
        // More correct than asked is a corrupt tally, not a pass.
        assert!(!quiz_passed(9, 8));
    }

    #[test]
    fn quiz_payout_first_pass_then_perfect_replays_only() {
        assert_eq!(
            quiz_payout(7, 8, false),
            QuizReward {
                cores: 15,
                first_pass: true
            }
        );
        assert_eq!(quiz_payout(8, 8, false).cores, 15);
        assert_eq!(
            quiz_payout(8, 8, true),
            QuizReward {
                cores: 3,
                first_pass: false
            }
        );
        assert_eq!(quiz_payout(7, 8, true).cores, 0);
        assert_eq!(
            quiz_payout(6, 8, false),
            QuizReward {
                cores: 0,
                first_pass: false
            }
        );
    }

    #[test]
    fn win_cores_adds_first_clear_and_rounds_the_mult_up() {
        assert_eq!(win_cores(10, false, 1.0), 10);
        assert_eq!(win_cores(10, true, 1.0), 20);
        assert_eq!(win_cores(10, false, 1.5), 15);
        assert_eq!(win_cores(10, true, 1.5), 30);
        assert_eq!(scale_cores(7, 1.5), 11); // 10.5 rounds up
    }

    #[test]
    fn hostile_multipliers_never_reduce_or_explode_cores() {
        for mult in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY, -2.0, 0.0, 0.5] {
            assert_eq!(scale_cores(10, mult), 10, "{mult}");
        }
        assert_eq!(scale_cores(u32::MAX, 1.5), u32::MAX);
        assert_eq!(win_cores(u32::MAX, true, 1.0), u32::MAX);
    }

    // ---- Loadout ----

    #[test]
    fn empty_loadout_changes_nothing() {
        let l = Loadout::from_gear(&[], &[]);
        assert_eq!(l, Loadout::default());
        assert_eq!(l.outage_extra_secs(60.0), 0.0);
    }

    #[test]
    fn loadout_reflects_each_item() {
        let l = Loadout::from_gear(
            &ids(&[HEADLAMP, SPARE_BATTERY, GOLDEN_CRIMPER]),
            &ids(&[FIELD_COFFEE, FIELD_COFFEE]),
        );
        assert!(l.hint_free);
        assert_eq!(l.outage_timer_mult, 1.25);
        assert_eq!(l.cores_mult, 1.5);
        // Two coffees still only add one coffee's worth per outage.
        assert_eq!(l.outage_timer_bonus_secs, 15.0);
        // 60 s base: +15 from the battery, +15 from the coffee.
        assert_eq!(l.outage_extra_secs(60.0), 30.0);
    }

    #[test]
    fn headlamp_alone_sets_only_hint_free() {
        let l = Loadout::from_gear(&ids(&[HEADLAMP]), &[]);
        assert_eq!(
            l,
            Loadout {
                hint_free: true,
                ..Loadout::default()
            }
        );
    }

    #[test]
    fn loadout_ignores_unknown_and_misplaced_ids() {
        // Coffee listed as gear and a battery listed as a consumable (a
        // hand-edited save) grant nothing.
        let l = Loadout::from_gear(&ids(&[FIELD_COFFEE, "x"]), &ids(&[SPARE_BATTERY]));
        assert_eq!(l, Loadout::default());
    }

    #[test]
    fn outage_extra_secs_rejects_non_finite_base() {
        let l = Loadout::from_gear(&ids(&[SPARE_BATTERY]), &[]);
        assert_eq!(l.outage_extra_secs(f64::NAN), 0.0);
        assert_eq!(l.outage_extra_secs(f64::INFINITY), 0.0);
        assert_eq!(l.outage_extra_secs(-100.0), 0.0);
    }
}

#[cfg(test)]
mod enter_reaction_tests {
    use super::*;
    use crate::salvage::PendingHaul;

    fn haul_of(n: u32) -> PendingHaul {
        PendingHaul {
            count: n,
            summary: "Fusion Splice ×1".to_string(),
        }
    }

    #[test]
    fn no_haul_is_one_short_bianca_line() {
        let lines = enter_reaction(None);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, HostId::Bianca);
    }

    #[test]
    fn haul_lines_name_the_parts_and_count() {
        let h = haul_of(2);
        let lines = enter_reaction(Some(&h));
        assert_eq!(lines.len(), 2);
        assert!(lines[0].1.contains("Fusion Splice"));
        assert!(lines[0].1.contains("2 cores credited"));
        assert_eq!(lines[1].0, HostId::Tessa);
    }

    #[test]
    fn tessa_tease_tiers_scale_with_haul_size() {
        let one = enter_reaction(Some(&haul_of(1)));
        let mid = enter_reaction(Some(&haul_of(3)));
        let big = enter_reaction(Some(&haul_of(5)));
        assert_ne!(one[1].1, mid[1].1);
        assert_ne!(mid[1].1, big[1].1);
    }
}

#[cfg(test)]
mod backdrop_tests {
    use super::*;

    fn sorted(mut draws: Vec<u8>) -> Vec<u8> {
        draws.sort_unstable();
        draws
    }

    // ---- Rotation: validation ----

    #[test]
    fn every_cycle_is_a_full_permutation_for_many_seeds() {
        // The shuffle-bag contract (Matt's direction): a different
        // backdrop each visit, no repeats until the pool cycles.
        for seed in [0_u64, 1, 7, 42, 1_000_000, u64::MAX] {
            let mut bag = BackdropBag::default();
            for cycle in 0..3_u64 {
                let draws: Vec<u8> = (0..BACKDROPS.len())
                    .map(|_| bag.advance(seed ^ cycle))
                    .collect();
                assert_eq!(
                    sorted(draws),
                    (0..BACKDROPS.len() as u8).collect::<Vec<u8>>(),
                    "seed {seed}: each cycle must deal every backdrop exactly once"
                );
            }
        }
    }

    #[test]
    fn consecutive_visits_never_repeat_even_across_reshuffles() {
        let mut bag = BackdropBag::default();
        let mut previous = bag.advance(3);
        for visit in 0..200_u64 {
            let next = bag.advance(visit.wrapping_mul(31).wrapping_add(5));
            assert_ne!(next, previous, "visit {visit} repeated a backdrop");
            previous = next;
        }
    }

    #[test]
    fn save_round_trip_preserves_the_rotation() {
        let mut bag = BackdropBag::default();
        bag.advance(11);
        bag.advance(12);
        let (remaining, current) = bag.to_save();
        let save = SaveData {
            warehouse_backdrop_bag: remaining,
            warehouse_backdrop_current: current,
            ..SaveData::default()
        };
        let restored = BackdropBag::from_save(&save);
        assert_eq!(restored, bag);
        // The restored bag continues the same cycle, draw for draw.
        let mut original = bag;
        let mut restored = restored;
        for step in 0..BACKDROPS.len() {
            assert_eq!(
                original.advance(step as u64),
                restored.advance(step as u64),
                "draw {step} diverged after a save round-trip"
            );
        }
    }

    // ---- Rotation: adversarial ----

    #[test]
    fn hostile_save_fields_are_sanitized_on_load() {
        let save = SaveData {
            warehouse_backdrop_bag: vec![9, 2, 2, 255, 0, 9],
            warehouse_backdrop_current: 99,
            ..SaveData::default()
        };
        let bag = BackdropBag::from_save(&save);
        assert_eq!(bag.current(), 0, "out-of-range current resets");
        let (remaining, _) = bag.to_save();
        assert_eq!(remaining, vec![2, 0], "unknowns dropped, dupes collapse");
        // Even starting from the sanitized bag, the cycle contract holds.
        let mut bag = bag;
        let mut draws: Vec<u8> = (0..3).map(|_| bag.advance(5)).collect();
        draws.dedup();
        assert_eq!(draws.len(), 3, "sanitized bag still deals without repeats");
    }

    #[test]
    fn nearly_empty_bag_refills_without_repeating_current() {
        // One index left in the bag and it IS the current backdrop's
        // only neighbor scenario: after it is drawn, the refill must
        // not hand the same scene straight back.
        let save = SaveData {
            warehouse_backdrop_bag: vec![4],
            warehouse_backdrop_current: 2,
            ..SaveData::default()
        };
        let mut bag = BackdropBag::from_save(&save);
        assert_eq!(bag.advance(1), 4, "the bagged index is dealt first");
        let after_refill = bag.advance(2);
        assert_ne!(after_refill, 4, "no immediate repeat across the refill");
    }
}
