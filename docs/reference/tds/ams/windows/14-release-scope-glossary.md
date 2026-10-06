# Release Scope and Glossary — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Know the scope</strong></summary>

**In plain English:** Use this set for Nokia 5520 AMS 9.8.3 simplex, cluster, and geo operations.

**Risk:** Read-only or low impact.

The same scope applies. Windows is the operator workstation; server-side AMS commands and paths remain on supported RHEL.

</details>

<details>
<summary><strong>Know the boundaries</strong></summary>

**In plain English:** Understand what this quick reference does not invent or replace.

**Risk:** Read-only or low impact.

- PowerShell/CMD does not make AMS a Windows server application.
- Exact sizing and full port matrices are not reproduced.
- NE CLI remains family/release-specific.
- SOAP envelopes come from active WSDL.
- Replace every placeholder.

</details>

<details>
<summary><strong>Glossary</strong></summary>

**In plain English:** Translate abbreviations used throughout.

**Risk:** Read-only or low impact.

| Term | Meaning |
|---|---|
| AMS | Access Management System |
| CLI | Command-line interface |
| DCN | Data communication network |
| GR | Geographic redundancy |
| LVM | Logical Volume Manager |
| NBI | Northbound interface |
| NE | Network element |
| OSS | Operations support system |
| RHEL | Red Hat Enterprise Linux |
| SFTP | SSH File Transfer Protocol |
| SOAP | XML messaging protocol |
| TLS | Transport Layer Security |
| WSDL | Web Services Description Language |

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
