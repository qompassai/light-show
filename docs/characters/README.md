# Characters

Biographies and asset inventories for every named character in Light Show:
the four companion mains, the four unlockable TDS specialists, and the two
Warehouse hosts. Each character page covers her role and track, personality
and voice (with a quoted in-game line), her canonical appearance, and exactly
which of her assets exist in this repo — and which do not.

| Character | Role | Page |
|---|---|---|
| Séraphine | Fiber-optic OSP splicing (main) | [seraphine.md](seraphine.md) |
| Ondine | Coax / broadband RF (main) | [ondine.md](ondine.md) |
| Linka | Mobile / cellular RF (main) | [linka.md](linka.md) |
| Lattice | Ethernet / copper LAN (main) | [lattice.md](lattice.md) |
| Clara | Calix CMS provisioning (specialist) | [clara.md](clara.md) |
| Aino | Nokia AMS network ops (specialist) | [aino.md](aino.md) |
| Hikari | FTTH/OSP field buildout (specialist) | [hikari.md](hikari.md) |
| Léa | WA 09 Telecom Admin prep (specialist) | [lea.md](lea.md) |
| Bianca | Warehouse host — test bench | [bianca.md](bianca.md) |
| Tessa | Warehouse host — field | [tessa.md](tessa.md) |

## Completeness matrix

Verified against the tree and against the paths the game code loads
(`game/src/waifu/mod.rs` `portrait_path` / `sprite_path` / `picker_stem`,
`game/src/warehouse/mod.rs` `HOSTS`). Art lives in two mirrored trees:
`game/assets/` (what the game loads) and repo-root `assets/` (source and
promotional art). Portraits exist in both unless noted.

| Character | Portrait | Mature portrait | Sprite sheet (code-loaded) | Picker frames | Expression portraits | Idle GIF / profile | Dialogue bank | Levels / footage / theme |
|---|---|---|---|---|---|---|---|---|
| Séraphine | ✅ | — (in sheet) | ✅ base + fullbody + mature | ✅ | ✅ 5 moods | ✅ | ✅ full JSON bank | ✅ |
| Ondine | ✅ | — (in sheet) | ✅ base + fullbody + mature | ✅ | ✅ 5 moods | ✅ | ✅ full JSON bank | ✅ |
| Linka | ✅ | — (in sheet) | ✅ base + fullbody + mature | ✅ | ✅ 5 moods | ✅ | ✅ full JSON bank | ✅ |
| Lattice | ✅ | — (in sheet) | ✅ base + fullbody + mature | ✅ | ✅ 5 moods | ✅ | ✅ full JSON bank | ✅ |
| Clara | ✅ | ✅ (see note 1) | ❌ missing | ❌ missing | ❌ none | ❌ none | ⚠️ greeting + win only | ✅ |
| Aino | ✅ | ✅ | ❌ missing | ❌ missing | ❌ none | ❌ none | ⚠️ greeting + win only | ✅ |
| Hikari | ✅ | ✅ | ❌ missing | ❌ missing | ❌ none | ❌ none | ⚠️ greeting + win only | ✅ |
| Léa | ✅ (added 2026-10-06) | — (not referenced) | ❌ missing | ❌ missing | ❌ none | ❌ none | ⚠️ greeting + win only | ✅ |
| Bianca | ✅ | — | — (none expected) | — | ❌ none | ❌ none | ⚠️ lines live in `warehouse/mod.rs` | — (Warehouse) |
| Tessa | ✅ | — | — (none expected) | — | ❌ none | ❌ none | ⚠️ lines live in `warehouse/mod.rs` | — (Warehouse) |

## Missing art — generation queue

Exact paths the game or the repo conventions expect, in priority order.

1. **Specialist sprite sheets** (code-loaded via `Companion::sprite_path`;
   the load fails today when a specialist is fielded):
   - `game/assets/sprites/clara/clara_sheet_fullbody.png`
   - `game/assets/sprites/aino/aino_sheet_fullbody.png`
   - `game/assets/sprites/hikari/hikari_sheet_fullbody.png`
   - `game/assets/sprites/lea/lea_sheet_fullbody.png`

   Convention (see `docs/ART_STYLE.md`): 6 mood rows × 4 frames; the base
   tier is 384×1152 at 96×192 per frame. Aseprite sources sit beside the
   PNG for the existing sheets.
2. **Specialist picker frames** (`Companion::picker_stem` defines the
   stems; the select screen currently fields only the base four, so this
   is latent until the specialists join the picker):
   `game/assets/sprites/picker/{clara,aino,hikari,lea}_select_{0..5}.png`
   plus each `{stem}_select_strip.png`.
3. **Specialist expression portraits** (painterly keeper convention,
   `assets/art/expressions/media-generation-{name}-{mood}-0-*.webp`):
   Clara, Aino, Hikari, and Léa × {alarmed, blush, celebrate, pout, wink}.
   None exist yet.
4. **Profile/README art for specialists and hosts**:
   `assets/art/companions/{clara,aino,hikari,lea,bianca,tessa}_animated.gif`
   family and `{name}_profile_64.png`. Promotional only; no runtime use.
5. **Warehouse host expressions**: Bianca and Tessa ship a single static
   portrait each. No code path expects more today; listed so the gap is a
   decision, not a surprise.

## Expression coverage note

In-game "expressions" are the six sprite-sheet moods — Idle, Blush, Wink,
Pout, Celebrate, Alarmed — which map onto the canonical emotion set as
neutral, embarrassed, fun/playful, annoyed, happy, and surprised. The
remaining canonical emotions (angry, sad, worried, determined, smug) have
no assets for any character, and the dialogue UI's `Expression` enum
currently implements Neutral only: the painterly expression portraits
under `assets/art/expressions/` are a source-art library awaiting
plumbing, not a runtime feature.

## Notes

1. The two `clara_portrait_mature.jpg` copies (in `game/assets/` and in
   repo-root `assets/`) are *different images* (verified by hash). Which
   one is canonical is undecided; neither was touched during the
   2026-10-06 inventory. The shipped portraits for Clara, Aino, and Hikari
   also diverge in details from the locked generation descriptions in the
   art pipeline's keeper records — see each character's page.
2. Léa's portrait was pulled into the repo on 2026-10-06 from her locked
   keeper generation (the "librarian look": chestnut bun, round glasses,
   burgundy cardigan, quiz tablet and study book, Swiss flag pin),
   converted to the repo's 1280×1920 portrait format. Before that, the
   code path `art/companions/lea_portrait.jpg` was a documented
   placeholder with no file behind it.
3. Séraphine's five painterly expression keepers were pulled into
   `assets/art/expressions/` on 2026-10-06, completing the four mains'
   sets; hers had previously existed only in the art working folders.
