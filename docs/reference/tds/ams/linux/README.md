# Nokia 5520 AMS 9.8.3 Operations Reference — Linux

This set turns the source command reference into short, collapsible runbooks. Each arrow opens the explanation, risk, prerequisites, command, and success criteria.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

<details open>
<summary><strong>Start here</strong></summary>

1. Confirm this is Nokia 5520 AMS 9.8.3.
2. Confirm the exact host, site, role, and account.
3. Expand and read the entire task.
4. Replace placeholders such as `<ams-host>`.
5. Run read-only checks first and back up before changes.
6. Verify immediately afterward.

```bash
whoami
ams_server status
ams_cluster status --detailed
```

</details>


<details>
<summary><strong>Report index</strong></summary>

| File | Subject |
|---|---|
| `00-a-z-master-reference.md` | A–Z command map |
| `01-command-index.md` | Command/account/impact index |
| `02-server-cluster-geo.md` | Server, Cluster, and Geo Operations |
| `03-backup-restore-migration.md` | Backup, Restore, and Migration |
| `04-installation-software.md` | Installation and Software Management |
| `05-security-identity-ssl.md` | Security, Identity, and TLS |
| `06-os-hardening-configs.md` | RHEL Hardening Commands and Configs |
| `07-network-firewall-nat.md` | Network, Firewall, and NAT |
| `08-storage-lvm.md` | Storage and LVM |
| `09-ne-bulk-operations.md` | NE and Bulk Operations |
| `10-nbi-soap-reference.md` | NBI SOAP Reference |
| `11-logs-monitoring-support.md` | Logs, Monitoring, and Support |
| `12-configuration-catalog.md` | Configuration Catalog |
| `13-use-cases.md` | Operational Use Cases |
| `14-release-scope-glossary.md` | Release Scope and Glossary |

</details>

<details>
<summary><strong>Placeholders and secrets</strong></summary>

- `<value>` means replace a required site-specific value.
- `[value]` in syntax means optional; do not type brackets unless the command expects them.
- AMS commands normally run as `amssys` unless stated otherwise.
- Never place real passwords in notes, tickets, screenshots, or command history.

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
