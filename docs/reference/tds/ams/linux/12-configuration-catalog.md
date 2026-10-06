# Configuration Catalog — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Find the right file</strong></summary>

**In plain English:** Use the file that owns the setting.

**Risk:** Read-only or low impact.

| Path | Purpose |
|---|---|
| `$AMSSOFTWAREHOME/conf/ams.conf` | Runtime/network |
| `$AMSSOFTWAREHOME/conf/amsgeomonitor.conf` | Geo monitoring |
| `/etc/chrony.conf` | Time sync |
| `/etc/fstab` | Persistent mounts |
| `/etc/logrotate.d/ams` | Log rotation |
| `/etc/profile` | Shell timeout/umask |
| `/etc/security/limits.conf` | Limits |
| `/etc/ssh/sshd_config` | SSH |
| `/etc/sysctl.conf` | Kernel/network |

</details>

<details>
<summary><strong>Inspect environment paths</strong></summary>

**In plain English:** Locate active software, scripts, logs, shared data, and local data.

**Risk:** Read-only or low impact.

```bash
printf "%s\n" "$AMSSOFTWAREHOME" "$AMSSCRIPTSDIR" "$AMSLOGDIR" "$AMSSHAREDDATAHOME" "$AMSLOCALDATAHOME"
```

</details>

<details>
<summary><strong>Review network keys</strong></summary>

**In plain English:** Bind traffic types to intended interfaces and publish translated address only when needed.

**Risk:** Connectivity-sensitive.

```ini
AMSCLIENTBINDIP=<client-facing-list>
AMSCLIENTCONNECTIP=<translated-public-ip>
AMSCLUSTERBINDIP=<cluster-interface-or-address>
AMSGEOLOCALBINDIP=<geo-sync-interface-or-address>
AMSNEBINDIP=<ne-facing-list>
```

</details>

<details>
<summary><strong>Set resource limits</strong></summary>

**In plain English:** Use the supplied script instead of editing values by hand.

**Risk:** OS configuration.

**Before you run it:** Run as non-root per the supplied guide

```bash
ams_update_limit.conf.sh
```

</details>

<details>
<summary><strong>Check locale</strong></summary>

**In plain English:** Keep sorting, messages, numbers, and time formats consistent across servers.

**Risk:** Changing locale can affect parsing and behavior.

```bash
locale
cat /etc/sysconfig/i18n
```

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
