# Server, Cluster, and Geo Operations

## Cluster lifecycle

```bash
ams_cluster status
ams_cluster status --detailed
ams_cluster status sw
ams_cluster status -l 10 -p 60
ams_cluster start
ams_cluster stop
ams_cluster restart
```

`stop` and `restart` place cluster servers in maintenance mode and are service-affecting.[cite:2][cite:3]

## Geographic role control

```bash
ams_cluster start -force active
ams_cluster start -force standby
ams_cluster switch active
ams_cluster switch standby
ams_cluster switch -force active
ams_cluster switch -force standby
```

Use `-force` only after verifying the remote site; bypassing role checks can contribute to dual-active conditions.[cite:2]

## Host evacuation

```bash
ams_cluster evacuate_ne <cluster_ip_address>
ams_cluster unevacuate_ne <cluster_ip_address> <weight>
ams_cluster deletehost <cluster_ip_address>
```

Evacuation sets an application's NE-management weight to zero. Unevacuation invokes rebalance but does not guarantee that the same NEs return.[cite:2]

## Server lifecycle

```bash
ams_server start
ams_server stop
ams_server stop maintenance
ams_server restart
ams_server status
ams_server status all -l 10 -p 60
ams_server version
```

Where supported, service targets include `appserver`, `dataserver`, `arbiter`, and `all`. Forced restart bypasses filesystem synchronization checks and is for exceptional recovery only.[cite:2]

## Geo configuration

```bash
ams_geo_configure.sh
```

The interactive workflow sets active/standby role, remote site, remote preferred/non-preferred data servers, synchronization NIC, and automatic switchover. Geo communication addresses must all use IPv4 or all use IPv6.[cite:3]

`$AMSSOFTWAREHOME/conf/amsgeomonitor.conf`:

```ini
PINGPONGPROTECTIONTIMEOUT=<minutes>
```

Override the current timer only after remediation:

```bash
ams_server resetgeo
```

The documented default is 30 minutes.[cite:3]

## Site-down hooks

```text
$PLATFORMSCRIPTSDIR/activeSiteDown.sh
$PLATFORMSCRIPTSDIR/stanbySiteDown.sh
```

The second filename is spelled `stanbySiteDown.sh` in the guide.[cite:3]

## Startup order

- Fresh cluster: start each server individually the first time.
- Normal cluster: preferred data server, non-preferred data server, then application servers.
- Geo: start and verify the active site before standby.
- Migration: follow the scenario-specific order because first application-server startup can trigger migration.[cite:3]
