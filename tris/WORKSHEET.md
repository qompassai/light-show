# LightShow (package `ai.qompass.lightshow`) — TRIS form-input worksheet

The requestor's working worksheet for this product's TRIS form (Technology Request Information Sheet): every requestor-facing field, pre-filled from the submission evidence wherever the evidence supports a value. Anything only the requestor can supply is marked **[INPUT NEEDED: …]** — nothing here is invented. Numbers in parentheses are the TRIS template's row numbers, so each line can be checked against the form and against [CHECKLIST.md](CHECKLIST.md).

This is one product's slice of the combined six-product submission worksheet prepared 2026-11-11 (ONTrack, LightShow, Fathom, Versemend, the Salesforce MCP integration, and Niflheim were submitted together in one email). IT reviewer fields (template rows 66–91) need no requestor input and are not listed here.

## Shared submitter fields (rows 3–6, repeated on all six forms)

- Requestor (3): Matt A. Porter — Field Service Technician (FST), matthew.porter@tdstelecom.com
- Requestor's Manager (4): [INPUT NEEDED: your manager's name]
- PAT Manager (5): [INPUT NEEDED: PAT Manager name — note: the process document names an Internal Employee Tools PAT contact as the AIR-access contact; that is not evidence he is the PAT Manager for this request]
- CoE Director (6): [INPUT NEEDED: CoE Director for the accountable Agile team — if left blank, the TRIS Administrator determines the accountable team, AM, and AD per the process document]

## Shared vendor fields (rows 10, 11, 42, plus the shared open-source fact at row 46)

Applies to the five Qompass AI products. The Salesforce integration's vendor is Salesforce; its vendor fields are in its own block below.

- Vendor Name (10): Qompass AI (the requestor's company; he is the developer/vendor)
- Website (11): qompass.ai
- What are the company's financial resources? (42): [INPUT NEEDED: the vendor financial-resources statement you want on record for Qompass AI — no figures exist in the package evidence]
- Has the governing Open Source license previously been reviewed by legal? (46, asked on every form): [INPUT NEEDED: whether Apache-2.0 has previously been reviewed by TDS legal — if not, this answer triggers legal review on each form]

## Product answers

- Technology Name (8): LightShow
- Technology Version (9): [INPUT NEEDED: which build is submitted — 0.1.0 / versionCode 1 (the artifact-audited signed bundle) or 0.1.1 / versionCode 2 (the reconciled release per D-1265)]
- Website (11): qompass.ai — privacy policy live at https://qompass.ai/privacy
- Description (14): An educational puzzle game about fiber optics: players route and splice light through fiber networks. Single-player, Rust + Bevy engine, fully offline.
- How used at TDS (15): Submitted for a policy determination and an honest, limited use: it is a game, not an operational tool. Relevance: (1) governance — an employee-developed, publicly distributed application on record rather than in shadow use; (2) an optional, informal fiber-concepts familiarization/outreach aid; (3) a working demonstration of applied development capability in the workforce.
- Who will use it (16): End customers / general players on public channels; at TDS, any employee who chooses to install it. No operational user group.
- Technology replaced/removed (17): Nothing. No TDS technology is removed or displaced.
- Direct customer impact (18): No operational impact — publicly distributed as a game; touches no TDS customer system, website, or service.
- Supported internally or by vendor (19): Internally — by the developer (the requestor / Qompass AI).
- Production or test/development (20): Neither, in the TDS-systems sense — it runs on personal/player devices and public store channels, not on TDS infrastructure.
- Impact if discontinued (21): Nothing at TDS; players lose a game.
- Backout plan/cost (22): Uninstall the app / remove store listings. No data migration — the game collects nothing and keeps progress on-device only. Cost negligible.
- Similar technologies at TDS (23): [INPUT NEEDED: similar/related technologies at TDS — none in the evidence (it is a game); IT may note training/simulation tools as related]
- Alternatives considered (24): Not submitting — rejected on governance grounds (leaves an employee-built, publicly distributed app unrecorded). No competing product evaluated; nothing operational is being procured.
- AI component (26): No — the audited build contains no AI component (zero permissions, no analytics/ad SDKs). Development used AI-assisted tooling, a development-method fact, not a component of the technology.
- Internet access (28): No — the signed bundle's manifest requests zero permissions, including no INTERNET; fully offline by design (verified against the bundle).
- Hosted internal/external (29): On the player's device only. No vendor-site hosting, no server component; no data processed or stored anywhere but the device.
- Uploads data outside the Enterprise network (30): No — with no INTERNET permission the app cannot upload data.
- Inbound access to the Enterprise network (31): No — no network capability at all.
- Data collected (32): None — no permissions, no analytics, no accounts; progress kept on the device only.
- Java required (33): No (Android build toolchain only, at build time).
- Microsoft 365 integration (34): No.
- Security risk assessment required (35): Requestor position: not required — zero permissions, zero data, zero network, open-source code reviewers can inspect end to end. Final determination is Security Engineering's.
- Cardholder/banking data (36): No.
- How licensed (38): Open Source — Apache License 2.0 (repo LICENSE, verified in the F-Droid gate audit).
- License model (39): No per-user, concurrent, or server model; use is not controlled by license software or by TDS.
- License fees and quantity (40): No fees. Quantity not applicable — unrestricted open-source use.
- Support/maintenance fees (41): None.
- How long around (43): In active development and publication preparation through 2026; the signed bundle was first uploaded to Google Play on 2026-10-13.
- Governing License (45): Apache License 2.0.
- License previously reviewed by legal (46): See the shared vendor block.
- Distributed / external use (47): Yes — Google Play and F-Droid channels, plus a Windows build. Per the template this requires legal review; declared openly.
- Modified by TDS (48): No modification by TDS planned.
- Directly impacts a product or service provided to customers (49): No — a standalone game, not used in a manner that directly impacts a product or service provided to customers.
- Mobile — developer (51): Third party — the requestor (Matt A. Porter) as a Qompass AI product.
- Mobile — store certification (52): Google Play — the signed bundle (0.1.0) was uploaded to the Play internal track on 2026-10-13 and the upload response verified. F-Droid — recipe prepared and a from-source proof build completed (F-Droid builds and signs with its own key). Apple — not applicable.
- Mobile — platforms (53): Google Android and MS Windows (plus the F-Droid Android channel). No iOS build.
- Mobile — TDS fee / required for a TDS service (54): No fee; not required to use any TDS service.
- Mobile — confidential information (55): No — the app processes no user data at all.
- Mobile — third-party code (56): Yes — open-source Rust ecosystem crates (Bevy engine and dependencies), all open source; the governing project license is Apache-2.0 and the open-source section above is completed. A full dependency inventory can be generated from the repo's Cargo.lock on request.
- Mobile — user data collection language (57): The app collects no user data. The privacy policy at https://qompass.ai/privacy states the game collects nothing, shares nothing, needs no account, and keeps progress on the device only (verified live in the gate audit).
- Mobile — development tools/SDKs (58): Android SDK/toolchain for the Android build; Rust toolchain. No Apple/Xcode code.
- Mobile — in-app purchase (59) / Apple push (60) / Game Center (61) / iCloud (62) / Passes (63) / Apple Maps (64): No — no iOS build; the game uses no maps at all.

---
*Worksheet slice prepared 2026-11-11 from the TRIS template and process documents and the submission package evidence. DRAFT — not yet submitted. See [README.md](README.md) for the status summary and the public-copy notes.*
