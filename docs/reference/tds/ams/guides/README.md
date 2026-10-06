# AMS Field Guides

Field guides for the Nokia 5520 AMS 9.8.3 operations topics in this repo's `ams/` docs. Written for field service technicians and network specialists — CLI-first, with copy-paste scripts for six platforms.

## How to use these guides

- Every guide opens with a one-line overview, a mermaid diagram, then CLI workflows (primary), then a GUI section (secondary).
- Every guide has script blocks for six platforms, alphabetical: **Linux bash, Nix-on-Droid, PowerShell, Termux, Windows cmd, WSL/Arch**.
- `<placeholder>` values in `"<angle brackets>"` must be replaced with your site's values before running.
- All 273 bash blocks pass `bash -n`; PowerShell/cmd blocks are marked where unverified.
- Impact warnings (service-affecting / destructive / high-risk) are called out prominently in each guide.

## The guides (alphabetical)

- [Backup, Restore & Migration](backup-restore-migration.md) — the safety net: how to snapshot the AMS server's data and software, put it back when something breaks, and carry it to a new release — including the one command that wipes everything and must never be typed casually.
- [Command Index](command-index.md) — the A–Z phone book of every AMS command: what it does, which Linux account is allowed to run it, and how much damage it can do — so you find the right tool instead of guessing.
- [Configuration Catalog](configuration-catalog.md) — the map of where every AMS setting lives — the config files, the environment variables, and the `ams.conf` keys — plus which tool you're actually supposed to touch them with (hint: usually not a text editor).
- [Installation & Software](installation-software.md) — how AMS software gets installed, activated, patched, and licensed — and the stop-change-start rhythm that keeps every change safe, verifiable, and reversible.
- [Logs, Monitoring & Support](logs-monitoring-support.md) — how to watch AMS health in real time, gather the evidence Nokia support will ask for, and manage log volume — without deleting the very evidence you need to diagnose the problem.
- [Master Reference](master-reference.md) — the whole AMS server in one alphabetized cheat sheet — if you can find your task's letter, you can find the command.
- [NBI SOAP Reference](nbi-soap-reference.md) — SOAP is the machine-to-machine remote control for AMS — your OSS or script talks XML over HTTPS and AMS creates NEs, users, and links without anyone clicking anything.
- [NE Bulk Operations](ne-bulk-operations.md) — doing the same thing to hundreds of network elements at once — create them, talk to them, back them up — from files instead of one-by-one clicking.
- [Network, Firewall & NAT](network-firewall-nat.md) — the plumbing diagram and the locked doors — which network each kind of AMS traffic must use, how the firewall rules get applied, and how AMS behaves behind NAT.
- [OS Hardening Configs](os-hardening-configs.md) — locking down the Linux underneath AMS — idle logouts, kernel network defenses, no ancient trust files, correct time, capped memory, and only the services AMS needs.
- [Release Scope & Glossary](release-scope-glossary.md) — which AMS setups these instructions cover, what they deliberately leave out, and what all the acronyms mean.
- [Security, Identity & SSL](security-identity-ssl.md) — the "locks and keys" chapter — who can log in, how passwords and secrets are protected, and how traffic between AMS components gets encrypted.
- [Server, Cluster & Geo](server-cluster-geo.md) — the "keeping AMS alive" chapter — starting and stopping servers, running them as a cluster so one failure doesn't kill management, and running two sites so a whole datacenter failure doesn't either.
- [Storage & LVM](storage-lvm.md) — the "disk plumbing" chapter — how AMS's database gets its own dedicated disk space using Linux LVM, and the one rule (keep snapshot room free) that protects your backups.
- [Use Cases](use-cases.md) — sixteen alphabetized, CLI-first playbooks for the real jobs you'll do on an AMS system — each one says when to use it, the exact commands, what success looks like, and how to back out.

## Sources

Synthesized from the `ams/` topic documents (Nokia 5520 AMS 9.8.3 operations reference, root + `linux/` + `windows/` flavors) and the vendor PDFs in `ams/AMS.zip`. No large verbatim copies — these teach; the sources remain the reference.
