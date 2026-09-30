# Ondine — MATURE Variant Sheet

Mature/adult-proportioned variant of the ondine full-body set. Same
character, same outfit, same art style — an older, less youthful read.

## What changed

- **Head**: smaller relative to body (compressed to ~37% height), jaw
  narrowed slightly — reads adult rather than chibi.
- **Body**: legs lengthened, feet stay planted; torso proportions
  adjusted to match.
- Derived from the repaired base sheet (see Pout repair note below), so
  all four pout frames carry clean frowns with no smile remnants.

## What's preserved

- Outfit, headset, coax coils, color palette, and all accessories —
  pixel-identical design, only proportions change.
- Mood-row layout, frame order, and timing: 6 rows × 4 frames @ 180 ms,
  6 px mood bar, same single-hop celebrate animation.
- `ondine_sheet_fullbody_mature.aseprite` is the editable project and
  round-trips pixel-identical to the PNG.

## Pout repair note

The shipped base sheet had a paint-over defect in the pout row: the
smile was not erased before the frown was drawn (frame 1), and smudging
surrounded the mouth in frames 2–3. Clean mouth zones were transplanted
from frame 4; this mature sheet was derived from the repaired base.
