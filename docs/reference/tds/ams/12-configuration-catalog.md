# Alphabetical Configuration Catalog

## Files

| Path | Purpose |
|---|---|
| `$AMSSOFTWAREHOME/conf/ams.conf` | Core runtime/network configuration |
| `$AMSSOFTWAREHOME/conf/amsgeomonitor.conf` | Geo monitoring |
| `$AMSSOFTWAREHOME/lib/dataserver/bin/switchover_hook` | Pre-switchover decision hook |
| `$PLATFORMSCRIPTSDIR/activeSiteDown.sh` | Active-site-down hook |
| `$PLATFORMSCRIPTSDIR/stanbySiteDown.sh` | Standby-site-down hook |
| `/etc/chrony.conf` | Time synchronization |
| `/etc/fstab` | Persistent mounts |
| `/etc/hosts.equiv` | Host equivalence; empty, mode `0400` |
| `/etc/logrotate.d/ams` | Log rotation |
| `/etc/profile` | `TMOUT`, `UMASK` |
| `/etc/rsyslog.d/ams.conf` | iptables log routing |
| `/etc/security/limits.conf` | `nproc`, `nofile` |
| `/etc/ssh/sshd_config` | SSH service settings |
| `/etc/sysctl.conf` | Kernel/network hardening |
| `/etc/sysconfig/i18n` | Locale on applicable RHEL |
| `/etc/vsftpd/vsftpd.conf` | FTP chroot |

## Environment variables

| Variable | Use |
|---|---|
| `AMSDEBUGDIR` | Debug/trace directory |
| `AMSEXTERNALLOCALDATAHOME` | Release-external local data |
| `AMSLOCALDATADIR` | Local data root; default `/var/opt` |
| `AMSLOCALDATAHOME` | Release-local data home |
| `AMSLOGDIR` | Log directory |
| `AMSSCRIPTSDIR` | Script directory |
| `AMSSHAREDDATAHOME` | Shared data home |
| `AMSSOFTWAREHOME` | Active release software home |
| `NAGID` | Installation base group-ID override |
| `NAUID` | Installation base user-ID override |
| `PLATFORMSCRIPTSDIR` | Platform hook directory |

[cite:2]

## `ams.conf` parameters

```ini
AMSBLOCKIP=<value>
AMSCLIENTBINDIP=<value>
AMSCLIENTCONNECTIP=<value>
AMSCLIENTNETWORK=<value>
AMSCLIENTNETWORKNIC=<value>
AMSNEBINDIP=<value>
AMSNENETWORK=<value>
AMSNENETWORKNIC=<value>
AMSOSSERVICEMETHOD=<value>
AMSPROCESSMONITORTRACELEVEL=<value>
AMSREPLICATIONTRACELEVEL=<value>
AMSSCRIPTSTRACELEVEL=<value>
```

[cite:3]

Additional referenced parameters:

```ini
AMSAPPSERVERENABLED=<true|false>
AMSARBITERENABLED=<true|false>
AMSCLUSTERBINDIP=<value>
AMSCLUSTERNETWORK=<IPv4-subnet>
AMSDATASERVERENABLED=<true|false>
AMSDATASERVERS=<host-list>
AMSGEOLOCALBINDIP=<value>
AMSMULTICAST2IP=<multicast-IP>
AMSMULTICASTIP=<multicast-IP>
AMSPREFERREDSERVER=<true|false>
AMSSSHCLIENTTIMEOUT=<seconds>
AMSSSHSERVERTIMEOUT=<seconds>
AMSUSERDICTIONARY=<Nokia|ALU>
```

Use the dedicated script instead of direct editing when available.[cite:2][cite:3]

## Locale

```ini
CMASK=022
LC_COLLATE=en_US.ISO8859-1
LC_CTYPE=en_US.ISO8859-1
LC_MESSAGES=C
LC_MONETARY=en_US.ISO8859-1
LC_NUMERIC=en_US.ISO8859-1
LC_TIME=en_US.ISO8859-1
LANG=C
```

Keep application/data servers consistent.[cite:3]

## Resource limits

`ams_update_limit.conf.sh` adds equivalents of:

```conf
amssys soft nproc 650000
amssys hard nproc 650000
amssys soft nofile 650000
amssys hard nofile 650000
```

Run the supplied script as non-root.[cite:2]
