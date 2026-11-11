# LightShow — business use at TDS

**What it is.** An educational puzzle game about fiber optics (`ai.qompass.lightshow`), built by the requestor as a Qompass AI product. Rust + Bevy. Single-player, fully offline. This document is deliberately plain about the central fact: **LightShow is a game. It is not a business tool, and this submission does not pretend otherwise.**

**Why it is in a TRIS package at all.** Three honest reasons, in order of weight:

1. **Governance.** It is an application written by a TDS employee, distributed publicly under his own company, on the same devices and in the same life as his TDS work. Employee-developed software is exactly the kind of thing that should be on record with IT rather than discovered later. Submitting it converts a potential shadow-IT question into a reviewed fact. That is a business use of the *process*, and it is the main one.
2. **Workforce capability, demonstrated rather than claimed.** LightShow is a complete, signed, store-submitted product built end-to-end by one field technician: Rust systems programming, a game engine, Android packaging and signing, privacy and store-compliance work. For a company that hires and promotes technical staff, a working artifact is better evidence of applied capability than a résumé line. (The companion submissions — ONTrack and Fathom — are where that capability points at operational problems.)
3. **Fiber-optics education, at an honest weight.** The game's subject is literally TDS's product: how light moves through fiber, splicing, and routing. It could serve as an informal, optional familiarization or outreach aid — a new hire or a STEM-outreach audience can play with the concepts. This is a minor, optional use. It is not a training system, it has no curriculum, no assessment, and no completion tracking, and it should not be booked as one.

**What it replaces.** Nothing. No TDS technology, training product, or process is displaced (CHECKLIST row 17).

**Risk posture.** This is the lowest-risk item in the package, and the claims are artifact-verified, not asserted:
- The signed bundle's manifest requests **zero permissions** — no INTERNET, no location, no storage, no contacts (read from the bundle itself in the 2026-10-13 gate audit).
- No data collected, no data shared, no accounts, no analytics or advertising SDKs; progress stays on the device. The live privacy policy (qompass.ai/privacy) says the same, so the public statement and the build agree.
- Open source under Apache-2.0: any reviewer can read every line that the claims rest on.
- No TDS data, no TDS network, no TDS system is touched in any way.

**The honest limit.** If IT's answer is "a game needs no approval," that answer is also a good outcome — it is a determination on record, which is the point of submitting. What this document will not do is dress a game up as productivity software to get it through the door.

*Draft, 2026-11-10. Not submitted.*
