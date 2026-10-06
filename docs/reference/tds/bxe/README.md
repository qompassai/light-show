# BxE Field Guides

Field guides for the BxE portal (`https://bxe-portal-prime.content.prod.oscp.ent.tds.net/`) — TDS's internal field system for customer diagnostics and install/service certification. CLI-first, with copy-paste scripts for six platforms.

## Important: what BxE is (and isn't)

- BxE here is the **internal TDS carrier portal** (OpenShift-hosted, `ent.tds.net`), used for customer diagnostics by address and the "certificy" install/service certification workflow (cf. the `## BXE / Certified w/ all tests passed` sections in this repo's `templates/`).
- **BxE has no public CLI or API documentation** (researched 2026-09-30 — nothing published anywhere). The techniques in these guides are built on standard web patterns: replaying the portal's own HTTP requests (discovered via browser DevTools), browser automation, and CPE-side checks.
- Note: `bxenetworks.com` is an unrelated managed-IT provider in El Paso, Texas — **not** the vendor of this portal. Don't expect vendor docs there.
- **Security is load-bearing here:** this is an internal system with customer data. No real credentials, session tokens, customer names, addresses, account numbers, or device serials appear anywhere in these guides — every script uses `<PLACEHOLDERS>` and `$BXE_USER`/`$BXE_PASS` from the environment. Read `portal-login-session.md` first.

## How to use these guides

- Start with `portal-login-session.md` (set up secure auth once per device).
- Then `address-to-diagnostics.md` (the 10-minute DevTools discovery that unlocks everything).
- Every guide: one-line overview → mermaid diagram → CLI workflows (primary) → GUI section (secondary).
- Script blocks for six platforms, alphabetical: **Linux bash, Nix-on-Droid, PowerShell, Termux, Windows cmd, WSL/Arch**.
- `VERIFIED` = confirmed from Matt, this repo's templates, or direct observation. `INFERRED` = standard patterns you must adapt — marked `<...>` everywhere.

## The guides (alphabetical)

- [Address → Customer → Device → Diagnostics](address-to-diagnostics.md) — the fastest path from a street address to live diagnostics (signal levels, errors, speed test, logs), skipping as much slow-portal clicking as possible.
- [API Access Request](api-access-request.md) — BxE has no public API — the template for asking TDS internal teams for sanctioned access, and what to ask for exactly.
- [Browser Automation Alternatives](browser-automation.md) — when the portal has no API and clicking is too slow, drive the browser itself with scripts — what works, what's fragile, and what will get you in trouble.
- [Certification ("Certificy") Workflow](certification.md) — the install/service certification flow — prove the job passes every test, record it the way BxE wants it, and never leave site without the evidence.
- [Portal Login & Session Handling](portal-login-session.md) — how to log into the BxE portal from scripts without ever putting your password in a file, in shell history, or in chat — and how to keep the session alive.
- [Slow-Portal Workarounds](slow-portal-workarounds.md) — the portal is slow — the playbook for making it feel fast anyway: deep links, direct API calls, parallel requests, response caching, and knowing what not to wait for.
