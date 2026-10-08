# TA Study & Quiz — Léa (NEC / Washington 09)

Léa's track is the classroom half of the trade: preparation for Washington's 09 telecommunications administrator study, built from the National Electrical Code and Washington's own law and administrative code. There is no plant to build here — the board is unused by design. Each level is a question set on one domain, answered in order, with Léa's explanation shown after every answer and the governing article reference kept with the question. The track walks the Code in order (general requirements, grounding, wiring methods, hazardous locations, special conditions, communications systems), detours through theory, then through Washington law (RCW 19.28) and administrative code (WAC 296-46B), and ends in a twenty-question mock exam.

Scoring on this track: the quiz UI owns the win condition (`quiz_passed` in `game/src/level.rs`) — the board win check returns false by design whenever a quiz block is present, so no empty board can ever pass a study level. Every level here sets the same bar: **70% correct** (the exam standard the game's quiz default encodes), untimed, with per-domain scoring feeding the results screen's study recommendations. Empty question sets fail closed. Wrong answers are teaching moments, not failures — only the final fraction decides.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>lea1 — Study Hall Warm-Up</strong></summary>

- **Level ID:** `lea1` · **World (data):** 8
- **Objective:** Léa's study hall is open. Ten questions on NEC Articles 90, 100, and 110 — the general requirements every telecom tech needs cold. Score 70% to move on.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: GeneralRequirements. Empty question sets can never pass (fail-closed).
- **Teaches:** NEC general requirements (Articles 90, 100, 110): the Code's purpose, scope, definitions, and installation requirements — the vocabulary every later article depends on.

</details>

<details>
<summary><strong>lea2 — Grounded: Article 250</strong></summary>

- **Level ID:** `lea2` · **World (data):** 8
- **Objective:** Grounding and bonding — NEC Article 250. The difference between a clean install and a callback. Ten questions, 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: GroundingBonding. Empty question sets can never pass (fail-closed).
- **Teaches:** Grounding and bonding (Article 250): why systems are grounded, what the equipment grounding conductor must do, and what objectionable current means.

</details>

<details>
<summary><strong>lea3 — Wiring Methods: Articles 300-398</strong></summary>

- **Level ID:** `lea3` · **World (data):** 8
- **Objective:** How it all goes in the walls — NEC Articles 300 through 398. Ten questions on wiring methods. 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: WiringMethods. Empty question sets can never pass (fail-closed).
- **Teaches:** Wiring methods (Articles 300–398): conductors in the same raceway, protection where cables pass through framing, and burial cover depths.

</details>

<details>
<summary><strong>lea4 — Hazardous Locations: Articles 500-516</strong></summary>

- **Level ID:** `lea4` · **World (data):** 8
- **Objective:** Classified locations — NEC Articles 500 through 516. Where the wrong fitting isn't just a fail, it's a hazard. Ten questions, 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: HazardousLocations. Empty question sets can never pass (fail-closed).
- **Teaches:** Hazardous (classified) locations (Articles 500–516): classification documentation and Division distinctions — where installation errors become ignition risks.

</details>

<details>
<summary><strong>lea5 — Special Conditions: Articles 705-780</strong></summary>

- **Level ID:** `lea5` · **World (data):** 8
- **Objective:** Special occupancies and conditions — NEC Articles 705 through 780. Ten questions, 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: SpecialConditions. Empty question sets can never pass (fail-closed).
- **Teaches:** Special conditions (Articles 705–780): interconnected power sources and their disconnecting means — the rules for systems that can energize a building from two directions.

</details>

<details>
<summary><strong>lea6 — Comms Systems Boss: Articles 800-830</strong></summary>

- **Level ID:** `lea6` · **World (data):** 8
- **Objective:** Léa's home turf — NEC Articles 800 through 830, communications systems. Twelve questions, the boss of the study track. 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **12 questions**, pass at **70%** (≥ 9 correct), untimed. Domains in data: CommsSystems. Empty question sets can never pass (fail-closed).
- **Teaches:** Communications systems (Articles 800–830): Chapter 8's relationship to Chapters 1–7, abandoned-cable removal, and plenum/air-handling space rules — the telecom admin's home articles.

</details>

<details>
<summary><strong>lea7 — Theory Workshop</strong></summary>

- **Level ID:** `lea7` · **World (data):** 8
- **Objective:** The math behind the plant — Ohm's law, power, dB calculations. Ten theory questions. 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: Theory. Empty question sets can never pass (fail-closed).
- **Notes:** Data note: the theory questions carry `article_ref` values in the level data that point at calculation mnemonics (for example 'NEC 120' on an Ohm's-law item) rather than literal Code articles — read them as worked-example labels, not citations.
- **Teaches:** Electrical theory: Ohm's law, power, and decibel math — the calculations behind every budget in the other seven tracks.

</details>

<details>
<summary><strong>lea8 — Washington Law: RCW 19.28</strong></summary>

- **Level ID:** `lea8` · **World (data):** 8
- **Objective:** Washington State electrical law — RCW 19.28. Separately scored on the real exam, so it gets its own level here. Ten questions, 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: WashingtonLaw. Empty question sets can never pass (fail-closed).
- **Teaches:** Washington law (RCW 19.28): the statutory definitions and classes of electrical work that bound what a telecom administrator may legally do.

</details>

<details>
<summary><strong>lea9 — Washington Admin Code: WAC 296-46B</strong></summary>

- **Level ID:** `lea9` · **World (data):** 8
- **Objective:** The administrative companion to RCW 19.28 — WAC 296-46B. Ten questions on Washington's electrical rules. 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **10 questions**, pass at **70%** (≥ 7 correct), untimed. Domains in data: WashingtonLaw. Empty question sets can never pass (fail-closed).
- **Teaches:** Washington administrative code (WAC 296-46B): which NEC edition and annexes Washington adopts, and the state-specific administrative definitions.

</details>

<details>
<summary><strong>lea10 — Mock Exam: Final Boss</strong></summary>

- **Level ID:** `lea10` · **World (data):** 8
- **Objective:** The full dress rehearsal — twenty questions drawn from both pretests, covering every domain. This is the exam before the exam. 70% to pass.
- **Mechanic / evaluator:** Quiz UI owns the level (`states/quiz.rs`); the board is unused and the board win check returns false by design when a quiz block is present. Questions are answered in presentation order with per-question explanations after each answer.
- **Pass thresholds / scoring:** `quiz_passed`: correct / total ≥ pass_pct. This level: **20 questions**, pass at **70%** (≥ 14 correct), untimed. Domains in data: GeneralRequirements. Empty question sets can never pass (fail-closed).
- **Notes:** Data note: all twenty questions are tagged `GeneralRequirements` in the level data even though the briefing describes a mixed-domain rehearsal drawn from both pretests — per-domain results-screen breakdowns on this level therefore report a single domain. The questions themselves do range across the earlier domains' material.
- **Teaches:** Exam simulation: twenty mixed questions at the same 70% bar as the real study standard — retrieval under breadth, across every domain at once.

</details>
