# Architectural Decisions — light-show

## Bevy 0.19 render features (2026-10-04)

**Decision**: Add `bevy_sprite_render` and `bevy_ui_render` to Bevy feature
lists (desktop + Android).

**Context**: Bevy 0.19 split rendering features from logic features.
`bevy_sprite`/`bevy_ui` alone produce a black screen (clear-color only).

**Consequence**: Any Bevy 0.19+ upgrade must audit feature flags for the
render/logic split.

## Audio decode guard (2026-10-04)

**Decision**: Pre-decode audio files on main thread with rodio before
handing to Bevy; skip and log undecodable files.

**Context**: bevy_audio 0.19.1 worker thread panics with `unwrap()` on
end-of-stream during menu transitions, killing audio.

**Consequence**: Audio failures degrade gracefully instead of crashing.

## Companion art direction (2026-10-03/04)

**Decision**: Base models = younger/safe (store-compliant); mature models =
adult (default in-game). Matt's references are locked design authority.

**Context**: Google Play/F-Droid require non-mature content. Mature set is
the default game target.

## Stabilization method (2026-10-04)

**Decision**: Per-frame alpha-centroid analysis; fix systematic defects
(row seams, drift) with integer translations; leave genuine secondary
motion (braid/cable sway) alone. Visual verification per girl.

**Context**: Blind global centroid alignment made Séraphine worse
(cy_span 6.6→21.4) because bbox tracks accessories, not feet. Metric is
not the territory — visual check wins.
