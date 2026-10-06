# Nokia 5520 AMS 9.8.3 Operations Reference

This bundle reorganizes the supplied Nokia 5520 AMS 9.8.3 manuals into alphabetized, command-oriented Markdown. It emphasizes literal commands, configuration paths, parameters, use cases, and impact warnings.

> **Safety:** Replace placeholders before execution. Validate commands in a lab and against the controlling Nokia documentation before production use. Commands that stop services, purge data, rotate encryption keys, restore databases, switch sites, or alter firewalls are marked as high impact.

## Report index

| File | Contents |
|---|---|
| `01-command-index.md` | Alphabetical CLI/script catalog |
| `02-server-cluster-geo.md` | Server, cluster, and geographic redundancy |
| `03-backup-restore-migration.md` | Data/software backup, restore, and migration |
| `04-installation-software.md` | Activation, components, patches, licenses |
| `05-security-identity-ssl.md` | Accounts, authentication, TLS, AES, SSH |
| `06-os-hardening-configs.md` | RHEL hardening configs and commands |
| `07-network-firewall-nat.md` | Networks, NAT, firewall, SFTP, SNMP |
| `08-storage-lvm.md` | LVM, filesystems, mounts, capacity rules |
| `09-ne-bulk-operations.md` | NE bulk tools, CLI transport, links |
| `10-nbi-soap-reference.md` | SOAP endpoints and operations |
| `11-logs-monitoring-support.md` | Logs, monitoring, diagnostics |
| `12-configuration-catalog.md` | Files, variables, parameters |
| `13-use-cases.md` | Alphabetical operational playbooks |
| `14-release-scope-glossary.md` | Scope, boundaries, glossary |

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice

## Conventions

- `<value>` is required and site-specific; `[value]` is optional.
- Run AMS scripts as `amssys` unless an entry says `root`.
- The `amssys` `PATH` includes most AMS script directories, so a path or `./` is generally unnecessary.[cite:2]
- Script spelling follows the supplied 9.8.3 PDFs, including underscores.
