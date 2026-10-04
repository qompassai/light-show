# Léa Quiz Data Import Plan

Maps `PhaedrusFlow/tds` `ta/quizzes/data/*.json` to Léa's in-game level content.

## Source files

| File | Questions | Maps to |
|---|---|---|
| `nec-800-830-communications.json` | Comms systems | **Comms Systems Boss** (8 Qs on exam) |
| `nec-250-grounding-bonding.json` | Grounding | Grounding mini-boss |
| `nec-300-398-wiring-methods.json` | Wiring methods | Wiring Methods mini-boss |
| `nec-500-516-hazardous-locations.json` | Hazloc | Hazloc mini-boss |
| `nec-705-780-special-conditions.json` | Special conditions | Special Conditions mini-boss |
| `nec-90-100-110-general.json` | General requirements | Tutorial / warm-up |
| `nec-mixed-02.json` | Mixed | Lookup Sprint pool |
| `general-theory.json` | Theory/calculations | **Theory Workshop** |
| `rcw-19-28.json` | WA law | **WA Law track** (separately scored!) |
| `wa-wac-296-46b.json` | WA admin code | **WA Law track** |
| `pretest-a.json` / `pretest-b.json` | Full pretests | **Mock Exam** (final boss) |

## JSON format (source)

```json
{
  "id": "nec-800-830-communications",
  "title": "...",
  "description": "...",
  "questions": [
    {
      "id": 1,
      "question": "...",
      "choices": ["...", "...", "...", "..."],
      "answer": "...",
      "explanation": "..."
    }
  ]
}
```

## Target format (game)

Léa's levels use a new `QuizLevel` type (to be defined in `game/src/level.rs`):

```rust
pub struct QuizQuestion {
    pub id: String,           // e.g. "nec800-001"
    pub prompt: String,       // the question text
    pub choices: [String; 4], // shuffled at runtime
    pub correct_idx: usize,   // index into choices (pre-shuffle)
    pub explanation: String,  // shown after answer (Léa's teaching moment)
    pub article_ref: Option<String>, // e.g. "NEC 800.3" for lookup sprints
    pub domain: QuizDomain,   // which boss/track this belongs to
}

pub enum QuizDomain {
    CommsSystems,      // NEC 800-830 (8 exam Qs)
    GroundingBonding,  // NEC 250 (3 exam Qs)
    WiringMethods,     // NEC 300-398 (3 exam Qs)
    SpecialConditions, // NEC 705-780 (4 exam Qs)
    GeneralRequirements, // NEC 90/100/110
    Theory,            // calculations (6 exam Qs)
    WashingtonLaw,     // RCW + WAC (17 exam Qs, separately scored)
}
```

## Import steps

1. **Copy** `ta/quizzes/data/*.json` into `game/assets/quiz/` (new dir)
2. **Transform** at build time (or runtime load):
   - `answer` (string) → find matching `choices` index → `correct_idx`
   - Derive `article_ref` from explanation text (regex for `NEC \d+\.\d+` / `Article \d+`)
   - Assign `domain` from filename
3. **Shuffle** choices at level load (Fisher-Yates, seeded for reproducibility)
4. **Explanations** become Léa's dialogue — shown after each answer with her
   encouraging framing ("Here's the lookup path...")

## Boss mapping (exam weights)

| Boss | Source file(s) | Exam Qs | Clear condition |
|---|---|---|---|
| Comms Systems | `nec-800-830-communications.json` | 8 | 70%+ × 3 sessions |
| Theory | `general-theory.json` | 6 | 70%+ × 3 sessions |
| Special Conditions | `nec-705-780-special-conditions.json` | 4 | 70%+ × 3 sessions |
| Grounding | `nec-250-grounding-bonding.json` | 3 | 70%+ × 3 sessions |
| Wiring Methods | `nec-300-398-wiring-methods.json` | 3 | 70%+ × 3 sessions |
| WA Law | `rcw-19-28.json` + `wa-wac-296-46b.json` | 17 | 70%+ × 3 sessions |
| **Final: Mock Exam** | `pretest-a.json` / `pretest-b.json` | 47 | 70%+ both sections |

## Lookup Sprint data

`nec-mixed-02.json` + all files → pool for timed sprints.
Each sprint picks 10 random questions, 60-second timer.
Scoring: speed bonus for fast correct answers.

## Spaced repetition

Track per-question: `seen_count`, `correct_count`, `last_seen`.
Priority queue: lowest `correct_count / seen_count` first.
Missed questions resurface after 1d, 3d, 7d (Anki-style intervals).

## Notes

- The `ta` repo also has Anki `.apkg` files in `flash_cards/` — these
  are for the player's own study, not imported into the game.
- `wa-09-telecom-admin-study-guide.pdf` is the human-readable companion.
  Léa's in-game "study guide" UI can link to it.
- PSI bulletin: confirm NEC edition before test day (2020 vs 2023).
  Léa should show the edition her questions target.
