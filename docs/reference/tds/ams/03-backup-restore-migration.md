# Backup, Restore, and Migration

## AMS data backup

```bash
ams_backup.sh /backup/ams-data.tar
ams_backup.sh -z /backup/ams-data.tar.gz
ams_backup.sh -c /backup/ams-data.tar
ams_backup.sh -f /backup/ams-data.tar
```

Remote URL forms:

```text
ftp://<user>:<password>@<host>/<path>/<file>
sftp://<user>:<password>@<host>/<path>/<file>
sftp://<host>/<path>/<file>
```

`-c` excludes common data, `-f` forces backup from the active cluster data server, and `-z` compresses. Prefer the standby data server for cluster backups.[cite:2]

## Scheduled backup

```bash
ams_schedule_backup -int
ams_schedule_backup
systemctl restart crond
```

The script creates a cron job and `runamsbackup.<id>.cfg`. Restart `crond` after an OS time-zone change.[cite:2]

## Software backup

```bash
ams_sw_backup.sh /backup/ams-software
```

Run as `root`; the script appends `<hostname>.bin` and includes AMS configuration, package metadata, repository, and software binary.[cite:2]

## Restore

```bash
ams_restore.sh /backup/ams-data.tar
ams_restore.sh -n /backup/ams-data.tar
ams_nerestore.sh /backup/ne-backup.tar
ams_nerestore.sh -b /backup/ne-backup.tar
```

`ams_restore -n` excludes licenses, useful on new hardware with a different host ID. Uncompress `.gz` input first. `ams_nerestore -b` restores only the NE-backup database.[cite:2][cite:4]

## Migration copy

```bash
ams_copy_datafiles --force
ams_copy_datafiles --force --from-release <previous-release>
ams_copy_datafiles --force --from-backup /absolute/path/backup.tar
```

Exceptional overwrite forms:

```bash
ams_copy_datafiles --force --overwrite
ams_copy_datafiles --force --overwrite --from-release <previous-release>
ams_copy_datafiles --force --overwrite --from-backup /absolute/path/backup.tar
```

> **Destructive:** `--overwrite` deletes the active release's persistency before copying.[cite:2]

## Migration sequence

1. Verify source health and privileged operator access.
2. Save license keys and stage 9.8.3 software.
3. Install required NE plug-ins and enhanced applications before persistency migration.
4. Back up and transfer source data for out-of-place migration.
5. Stop relevant services.
6. Run the appropriate copy command.
7. Start in scenario-specific order and monitor status.
8. Install licenses/client and validate alarms, rules, and NE access.[cite:3]

## Database reinitialization

```bash
ams_server stop
ams_remove_data.sh
ams_server start
```

> **Destructive:** this purges database, shared data, and local data. It is corrupted-database recovery, not routine cleanup.[cite:2]
