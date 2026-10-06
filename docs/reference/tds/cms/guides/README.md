# CMS Field Guides

Field guides for the Calix toolset in this repo's `cms/` docs (vendor PDFs in `cms/CMS.zip`). Written for field service technicians and network specialists — CLI-first, with copy-paste scripts for six platforms.

## How to use these guides

- Every guide opens with a one-line overview, a mermaid diagram, then CLI workflows (primary), then a GUI section (secondary).
- Every guide has script blocks for six platforms, alphabetical: **Linux bash, Nix-on-Droid, PowerShell, Termux, Windows cmd, WSL/Arch**.
- `<placeholder>` values in `"<angle brackets>"` must be replaced with your site's values before running.
- All bash blocks pass `bash -n`; PowerShell/cmd blocks are marked where unverified.
- Impact warnings (service-affecting / destructive / high-risk) are called out prominently in each guide.

## The guides (alphabetical)

- [AXOS & EXOS Release Notes R26.3.0](axos-exos-release-notes.md) — AXOS R26.3 makes the E9-2 OLT smarter about routing, subscriber security, and monitoring (and fixes a nasty silent alarm-stream bug) — while EXOS R26.3 adds new Wi-Fi 7 boxes, better cloud health reporting, and changes what the LEDs tell you; upgrade SMx *before* the OLT, and the ONT *before* the OLT.
- [CMS NBI API](cms-nbi-api.md) — the CMS NBI is the "talk to the server with code instead of clicks" door into Calix Management System — you POST XML requests to it and it activates or queries subscriber services on Calix access gear (C7, E3/E5, E7 GPON/xDSL, AE ONTs) without touching the GUI.
- [CMS Release Notes 15.2 & 16.0](cms-release-notes.md) — CMS 15.2 is the full-featured release for mixed legacy networks (C7/E7/E3/E5/B6/AE); CMS 16.0 is a security-hardened release that *only* manages E7 GPON — upgrading to 16.0 on the wrong network strands your other platforms.
- [EXOS Provisioning](exos-provisioning.md) — EXOS is the software inside Calix's GigaSpire and GigaPro boxes — provisioning one means getting it a Registration ID, letting it phone home to Calix Cloud over TR-069, and having it pull its config automatically.
- [SmartMDU Application Guide](smartmdu.md) — SmartMDU is Calix's managed-Wi-Fi-in-apartment-buildings system — one GigaSpire per unit plus property-wide networks, all run from Calix Cloud, with a portal (PropertyWorx) that lets the building manager move tenants in and out without calling you.
- [SMx API](smx-api.md) — SMx is Calix's modern, REST/JSON API for managing AXOS gear (like the E9-2 OLT) — the successor to the older CMS XML northbound interface, and the API to use for anything AXOS.

## Sources

Synthesized from the vendor PDFs in `cms/CMS.zip`: CMS R15.x NBI API Guide, SMx APIdoc, EXOS-ES-SPG provisioning guide, SmartMDU AppGuide, CMS RN 15.2.129, CMS RN 16.0.0, EXOS R26.3.0 RN, AXOS R26.3.0 E9-2 OLT RN. No large verbatim copies — these teach; the PDFs remain the reference.
