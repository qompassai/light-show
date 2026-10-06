# Installation and Software Management — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Activate an installed release</strong></summary>

**In plain English:** Make an installed release operationally selectable.

**Risk:** Configuration and release-selection change.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo /opt/ams/software/<release>/bin/ams_activate.sh
```
exit
```

</details>

<details>
<summary><strong>Manage components</strong></summary>

**In plain English:** Install, activate, or deactivate plug-ins, patches, and emergency fixes.

**Risk:** Service-affecting; unscoped deactivation affects all non-core components.

**Before you run it:** Back up, stop the correct scope, and stage approved files

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_install.sh
ams_install.sh --installActivate <directory-or-bin-files>
ams_install.sh --install <directory-or-bin-files>
ams_install.sh --activate <directory-or-bin-files>
ams_install.sh --deactivate
```
exit
```

</details>

<details>
<summary><strong>Use a safe component workflow</strong></summary>

**In plain English:** Stop, change software, start, and verify in a predictable order.

**Risk:** Service-affecting.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_server stop
ams_install.sh --installActivate /staging/ams-components
ams_server start
ams_server version
```
exit
```

</details>

<details>
<summary><strong>Save and verify a golden configuration</strong></summary>

**In plain English:** Record the known-good software set and compare against it later.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_server version
ams_server version save
ams_server version save --label <Label> /path/GoldenEMSSwConfig
ams_server version verify /path/GoldenEMSSwConfig
ams_cluster status sw
```
exit
```

</details>

<details>
<summary><strong>Install and inspect licenses</strong></summary>

**In plain English:** Install host-specific entitlements and read counters.

**Risk:** Install changes configuration; counters are read-only.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo ams_install_license
getLicenseCounter
```
exit
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
