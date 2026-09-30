# Séraphine — MATURE Variant Sheet

Mature/adult-proportioned variant of the séraphine full-body set. Same
character, same outfit, same art style — an older, less youthful read.

## What changed

- **Head**: smaller relative to body (compressed to ~31% height), jaw
  narrowed slightly — reads adult rather than chibi.
- **Body**: legs lengthened, feet stay planted; torso proportions
  adjusted to match.
- Derived from the repaired base sheet (see Pout repair note below), so
  all four pout frames carry clean frowns with no smile remnants.

## What's preserved

- Outfit, fiber-braid ponytail, light-pipe visor, color palette, and
  all accessories — pixel-identical design, only proportions change.
- Mood-row layout, frame order, and timing: 6 rows × 4 frames @ 180 ms,
  6 px mood bar, same wink-and-spin celebrate animation.
- `seraphine_sheet_fullbody_mature.aseprite` is the editable project and
  round-trips pixel-identical to the PNG.

## Pout repair note

The shipped base sheet had a paint-over defect in the pout row: the
smile's left half was never erased, leaving a smile remnant beside the
right-shifted frown in all four frames (plus a black smear in frame 3).
The smile region was erased to clean cheek and the frown completed
symmetrically from its clean right arm; this mature sheet was derived
from the repaired base.
