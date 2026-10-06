# Logs, Monitoring, and Support

## Status monitoring

```bash
ams_server status
ams_server status all -l 10 -p 60
ams_cluster status
ams_cluster status --detailed
ams_cluster status -l 10 -p 60
innotop
```

Use loops during startup, migration, replication recovery, and switchovers.[cite:2]

## Database fragmentation

```bash
$AMSSCRIPTSDIR/ams_db_defragment.sh all analyse
$AMSSCRIPTSDIR/ams_db_defragment.sh -t <table> analyse
ams_server stop
$AMSSCRIPTSDIR/ams_db_defragment.sh all execute
ams_server start
```

`analyse` may run online. `execute` requires AMS stopped on the active data server and triggers full synchronization afterward.[cite:2]

## Log management

```bash
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/ams-logs.tar
ams_log_manager.sh --resetlogs --category log --target all
ams_log_manager.sh --setlevel '<category>,<level>' --target all
ams_reset_logs.sh
```

Collection categories include `log`, `debug`, `os`, and `all`; targets may be IPs, site names, or `all`.[cite:2]

## Support diagnostics

```bash
ams_support.sh --domain app --command jstack --target all --destination /tmp/jstack.tar
ams_support.sh --domain app --command jmap --target <site-or-IP> --destination /tmp/jmap.tar
ams_support.sh --domain security --command killadminsessions
ams_support.sh --domain security --command resetadminpwd
```

JVM dumps can temporarily stall a heavily loaded server.[cite:2]

## Shorten log lines

```bash
convert_to_shorter_line.pl -f /path/processmonitor.log -l 80
```

The source is preserved and a new file is created.[cite:2]

## Rsyslog routing

`/etc/rsyslog.d/ams.conf`:

```rsyslog
:msg,contains,"IPTables-Dropped" -/var/log/iptables.log
:msg,contains,"Connection established" -/var/log/iptables.log
& stop
```

```bash
systemctl restart rsyslog
```

`/etc/logrotate.d/ams`:

```conf
/var/log/iptables.log {
    rotate 7
    daily
    missingok
    dateext
    delaycompress
    compress
}
```

The server guide also documents a size-based 1024 MB alternative.[cite:4]
