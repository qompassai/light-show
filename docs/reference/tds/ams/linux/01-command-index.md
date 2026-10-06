# Alphabetical Command Index — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>A–C</strong></summary>

**In plain English:** Find the command, required account, purpose, and likely impact.

**Risk:** Read-only or low impact.

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_activate.sh` | `root` | Activate release | **Configuration** |
| `ams_backup.sh` | `amssys` | Back up data | **I/O** |
| `ams_change_ip_subnet_server` | `amssys/root` | Change network | **Service** |
| `ams_check_ssl.sh` | `amssys` | Check TLS | **Read** |
| `ams_cluster` | `amssys` | Cluster control | **Varies** |
| `ams_copy_datafiles` | `privileged` | Copy persistency | **High** |
| `ams_createfirstuser.sh` | `privileged` | Create first admin | **Security** |

</details>

<details>
<summary><strong>D–M</strong></summary>

**In plain English:** Find the command, required account, purpose, and likely impact.

**Risk:** Read-only or low impact.

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_db_defragment.sh` | `root` | DB analysis/rebuild | **Varies** |
| `ams_enable_ssl.sh` | `amssys` | Enable TLS | **Service** |
| `ams_geo_configure.sh` | `amssys` | Configure geo | **Service** |
| `ams_install.sh` | `amssys` | Manage components | **Varies** |
| `ams_log_manager.sh` | `amssys` | Logs | **Varies** |

</details>

<details>
<summary><strong>N–R</strong></summary>

**In plain English:** Find the command, required account, purpose, and likely impact.

**Risk:** Read-only or low impact.

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_ne_cli` | `amssys` | Commands to NEs | **Potential service** |
| `ams_ne_mgr` | `amssys` | Bulk NE changes | **Provisioning** |
| `ams_recreate_aes_keys.sh` | `amssys` | Rotate AES | **High** |
| `ams_remove_data.sh` | `amssys` | Purge data | **Destructive** |
| `ams_restore.sh` | `amssys` | Restore data | **High** |

</details>

<details>
<summary><strong>S–Z</strong></summary>

**In plain English:** Find the command, required account, purpose, and likely impact.

**Risk:** Read-only or low impact.

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_server` | `amssys` | Server control/status | **Varies** |
| `ams_support.sh` | `amssys` | Diagnostics/security | **Varies** |
| `ams_update_database_pwd.sh` | `privileged` | Rotate DB password | **Auto-restart** |
| `ams_updatefirewall` | `root` | Firewall | **Connectivity** |
| `createUsernamePassword` | `amssys` | Encrypted credential | **Sensitive** |
| `getLicenseCounter` | `privileged` | License counters | **Read** |
| `retrieve_nes.sh` | `AMS user` | Export NEs | **Read** |

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
