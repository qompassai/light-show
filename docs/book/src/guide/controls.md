# Controls

## ELI5

There is no keyboard control at all — no arrow keys, no shortcuts, no
text entry. You play with a mouse or a finger. Every control in the game
is one of two gestures: **press-and-drag between dots**, or **tap a
button or a pill**. That's it, on desktop and on Android alike.

## The two input devices the code handles

`game/src/board.rs::handle_pointer_input` reads exactly two input
sources:

- `ButtonInput<MouseButton>` — the **left mouse button only**. Press
  and release are the two events.
- `Touches` — touch down / touch up (the first active touch drives the
  pointer; see `track_pointer`).

Everything else — middle/right mouse buttons, mouse wheel, keyboard —
is ignored. A grep of the whole `game` crate finds zero keyboard
handling, so any "keyboard shortcut" you think you remember is not in
this game.

## Board gestures (the puzzle itself)

The board is a graph of nodes (circles) connected by edges. The press /
release decision logic lives in `resolve_press` / `resolve_release`,
kept pure and unit-tested so the geometry math is verifiable without
running the game.

### Press on a node → start a drag

If your press lands within the node hit radius (50 world units around
the node center, `NODE_HIT_RADIUS`; the drawn circle is 26 units) and
that node is the nearest one, the game records the drag's starting node.
While you hold, a live preview line follows your pointer from the start
node (`draw_board_gizmos` draws it in hot pink, `#ff6fae`).

### Release on a different node → connect

If the release lands on a *different* node, and the level offers a
component between those two nodes (`ComponentChoice { from, to, … }`),
the edge connects — using the current default choice for that pair
(the first slot, unless you picked otherwise with a pill). Release on
empty space, on the same node you started from, or on a pair with no
offered component, and nothing happens: `ReleaseAction::None`.

### Tap a pill → pick that component variant

Some gaps can be filled by more than one component type — fusion splice
vs. mechanical splice, for example. Those edges draw a row of **pills**
(small circles, 34 units radius, spread 70 units apart perpendicular to
the edge, centered on its midpoint) — one pill per choice. Tapping a
pill selects that variant immediately: no drag needed. Edges with only
one unambiguous choice draw no pill at all; the drag gesture alone
places them.

### Press on empty space → nothing

A press that hits neither a node nor a pill resolves to
`PressAction::None`. No accidental placements.

### The board stays live during an outage

The board systems (`track_pointer`, `handle_pointer_input`,
`draw_board_gizmos`) run in *both* `GameState::Playing` and
`GameState::OutageActive`. That is deliberate: repairing an outage is
the same drag-and-tap interaction under a countdown, and tearing the
board down on the outage transition would have wiped the player's
in-progress repair — so it doesn't happen.

## UI buttons (menu, results, credits)

All screen buttons are Bevy UI buttons driven by `Interaction::Pressed`
only. Hovering does nothing — the menu's own unit tests assert that a
`Hovered` interaction changes no selection and requests no state change.
Tap (or click) is the whole contract.

| Screen | Button | Effect (state transition) |
|---|---|---|
| Main menu | Start | Enters `Playing`; loads the current level |
| Main menu | Credits | Enters `Credits` (music attribution, CC-BY compliance) |
| Main menu | Companion (4 buttons) | Sets the active companion; default is Séraphine (fiber) |
| Results (win) | Continue | Advances to the next bundled level → `Playing` (shown only if another level exists) |
| Results (loss) | Retry | Restarts the same level → `Playing` |
| Results | Main Menu | → `MainMenu`; resets the level index to 0 |
| Credits | Back | Returns to `MainMenu` |

One code smell worth knowing: the menu's doc comment says "title,
companion picker, world/level select" — but no world/level select UI
exists in the code. The only navigation is Start/Credits/companion.
(Start always loads `CurrentLevelIndex`, which defaults to 0 and is
reset to 0 whenever you return to the menu from results.)

## What is NOT a control (unverifiable)

The following do not exist in the game's input code, so don't look for
them:

- **No keyboard input of any kind.** No pause key, no undo key, no
  shortcuts.
- **No multi-touch gestures.** Pinch, two-finger, swipe — the pointer
  tracker reads the first active touch only.
- **No right-click / middle-click / wheel actions.**
- **No in-game pause or settings button** reachable from any state.
