# Alphabetical Operational Use Cases

## Activate a release

```bash
sudo /opt/ams/software/<release>/bin/ams_activate.sh
sudo -iu amssys ams_server start
sudo -iu amssys ams_server status
```

## Back up to SFTP

```bash
sudo -iu amssys ams_backup.sh -z 'sftp://<host>/<path>/ams-backup.tar.gz'
```

Prefer key-based authentication and validate archive/log output.[cite:2]

## Change SFTP port

```bash
ams_set_sftp_port.sh --check
ams_set_sftp_port.sh 2222
ams_server restart
ams_updatefirewall
```

## Collect support evidence

```bash
ams_cluster status --detailed
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/ams-support.tar
ams_support.sh --domain app --command jstack --target all --destination /tmp/ams-jstack.tar
```

## Convert simplex to cluster

```bash
ams_server stop
ams_simplex_to_cluster.sh
ams_updatefirewall
ams_server start
ams_cluster status --detailed
```

The interactive conversion requests cluster NIC, multicast addresses, and alternate data-server details.[cite:3]

## Defragment database

```bash
$AMSSCRIPTSDIR/ams_db_defragment.sh all analyse
ams_server stop
$AMSSCRIPTSDIR/ams_db_defragment.sh all execute
ams_server start
```

Run execution only on active data server in a window.[cite:2]

## Evacuate application server

```bash
ams_cluster status --detailed
ams_cluster evacuate_ne <cluster-IP>
ams_show_ne_balancing.sh -a <cluster-IP>
ams_server stop maintenance
```

Return:

```bash
ams_server start
ams_cluster unevacuate_ne <cluster-IP> 1
ams_cluster status --detailed
```

## Fall back to local authentication

```bash
ams_switch_authentication_local
```

Restore intended external authentication after remediation.[cite:2]

## Force data-server switchover

```bash
ams_cluster status --detailed
ams_switch_active_dataserver
ams_cluster status --detailed
```

Use `-f` only in automation with independent checks.[cite:2]

## Migrate from backup

```bash
ams_cluster stop
ams_copy_datafiles --force --from-backup /absolute/path/source-backup.tar
ams_server start
ams_server status all -l 30 -p 60
```

Install required plug-ins before persistency copy.[cite:3]

## Recover administrator

```bash
ams_support.sh --domain security --command killadminsessions
ams_support.sh --domain security --command resetadminpwd
```

Check release/setup restrictions.[cite:2]

## Restore without old licenses

```bash
ams_restore.sh -n /backup/ams-data.tar
ams_install_license
ams_server start
```

Use on a different host ID.[cite:2][cite:4]

## Rotate database credentials

```bash
ams_update_database_pwd.sh
```

Do not manually restart afterward.[cite:2]

## Validate software

```bash
ams_server version save --label GOLDEN983 /secure/GoldenEMSSwConfig
ams_server version verify /secure/GoldenEMSSwConfig
ams_cluster status sw
```

## Verify NBI

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
```

[cite:6]
