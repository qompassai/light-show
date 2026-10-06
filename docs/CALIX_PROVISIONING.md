# Calix Provisioning — Mechanics Document

Clara's track. The player stops splicing individual fibers and starts
thinking like a provisioning engineer: service profiles, bulk turn-ups, and
the constraint that every subscriber on a PON split must fit inside one
optical budget.

## Concept

A Calix OLT port fans out through passive splitters to 32 ONTs. Each ONT
gets a **service profile** (down/up rates, latency class, voice/video/data
flags). The puzzle: assign profiles to subscribers and place splitters so
that (a) every ONT's service tier is satisfiable, and (b) the worst-case
optical path still lands inside the receive window.

## Data model (`game/src/level.rs`)

```rust
struct ServiceProfile {
    name: &'static str,      // "GPON-100", "XGS-1000", ...
    down_mbps: u32,
    up_mbps: u32,
    // Optical requirements for this tier:
    min_rx_dbm: f32,         // most sensitive the ONT may need
    max_rx_dbm: f32,         // overload point
    max_split_ratio: u32,    // 32 for GPON, 64 for XGS-PON, ...
}
```

Levels ship a subscriber list: each entry names a demanded tier and a
distance band. The player picks splitter ratios (1:4 / 1:8 / 1:16 / 1:32
cascades) and assigns ONTs to branches.

## Core mechanic: bulk provisioning

1. **Survey.** The level shows N subscribers with demanded tiers and
   distances. A shared OLT port with a fixed launch power.
2. **Plan the split.** Place splitters; each 1:2 split costs ~3.5 dB plus
   excess loss. Cascaded splits multiply the ratio and add the losses.
3. **Assign profiles.** Drag a service profile onto each ONT. Higher tiers
   need tighter optical margins (XGS-PON wants hotter, cleaner light).
4. **Verify the worst case.** The game computes the longest/highest-loss
   path per branch: `P_rx = P_tx - Σ(splitter + fiber + connector losses)`.
   Every ONT must land inside its profile's `[min_rx, max_rx]` window.
5. **Outage event.** Mid-level: a dirty splitter port or a chewed drop adds
   loss on one branch — find the branch whose ONTs fell out of window and
   re-balance (move a subscriber, clean the port, or re-split).

## Teaching goals

- Split ratio vs optical budget: why you can't put 64 subs on a tired OLT.
- Profile tiers map to real products (GPON vs XGS-PON sensitivities).
- The "failing branch" diagnostic: one bad port, many complaining ONTs —
  the classic PON troubleshooting pattern.

## Win condition

All subscribers provisioned inside their windows, outage branch repaired.
Bonus for fewest splitters (cost) and fastest diagnosis.
