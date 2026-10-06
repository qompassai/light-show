# Backup, Restore, and Migration — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Back up AMS data</strong></summary>

**In plain English:** Make a rollback copy before maintenance or recovery.

**Risk:** I/O intensive but normally non-destructive.

**Before you run it:** Check destination access and free space

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_backup.sh /backup/ams-data.tar
ams_backup.sh -z /backup/ams-data.tar.gz
ams_backup.sh -c /backup/ams-data.tar
ams_backup.sh -f /backup/ams-data.tar
```
exit
```

**Success looks like:** The log completes cleanly and the archive has a plausible size.

</details>

<details>
<summary><strong>Back up to SFTP</strong></summary>

**In plain English:** Store a copy away from the AMS host.

**Risk:** Network and credential sensitive.

**Before you run it:** Prefer SSH keys or an approved secret source; do not place production passwords in history

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_backup.sh -z 'sftp://<host>/<path>/ams-data.tar.gz'
```
exit
```

</details>

<details>
<summary><strong>Schedule backups</strong></summary>

**In plain English:** Create a cron-based automatic backup schedule.

**Risk:** Changes scheduling.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_schedule_backup -int
ams_schedule_backup
sudo systemctl restart crond
```
exit
```

</details>

<details>
<summary><strong>Back up AMS software</strong></summary>

**In plain English:** Save software, configuration, package metadata, repository, and binary.

**Risk:** I/O intensive.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo ams_sw_backup.sh /backup/ams-software
```
exit
```

</details>

<details>
<summary><strong>Restore AMS data</strong></summary>

**In plain English:** Replace current data from a backup; `-n` omits old licenses for different hardware.

**Risk:** High, data-changing, and service-affecting.

**Before you run it:** Validate and decompress the archive, stop the proper scope, and keep rollback media

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_restore.sh /backup/ams-data.tar
ams_restore.sh -n /backup/ams-data.tar
```
exit
```

</details>

<details>
<summary><strong>Restore NE data</strong></summary>

**In plain English:** Restore NE backup records; `-b` limits restoration to the NE-backup database.

**Risk:** High; changes NE-related data.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_nerestore.sh /backup/ne-backup.tar
ams_nerestore.sh -b /backup/ne-backup.tar
```
exit
```

</details>

<details>
<summary><strong>Copy migration persistency</strong></summary>

**In plain English:** Move data from an older release or backup into the target release.

**Risk:** High; `--overwrite` deletes target persistency first.

**Before you run it:** Install required plug-ins/apps, back up both sides, stop services, and verify absolute paths

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_copy_datafiles --force
ams_copy_datafiles --force --from-release <previous-release>
ams_copy_datafiles --force --from-backup /absolute/path/backup.tar
# Exceptional recovery only:
ams_copy_datafiles --force --overwrite --from-backup /absolute/path/backup.tar
```
exit
```

</details>

<details>
<summary><strong>Reinitialize a damaged database</strong></summary>

**In plain English:** Erase DB, shared, and local data so AMS can start empty.

**Risk:** Destructive; never routine cleanup.

**Before you run it:** Use only for approved corrupted-database recovery with a tested backup

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_server stop
ams_remove_data.sh
ams_server start
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
