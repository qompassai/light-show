# Network, Firewall, and NAT

## Network roles

| Network | Purpose | Rule |
|---|---|---|
| Cluster | Intra-site servers | Isolated L2, IPv4 only, same MTU, IGMP snooping off |
| DCN/NE | AMS-to-NE | Isolate from public/user networks |
| Northbound/client | GUI and OSS/NBI | Firewall/VPN before public connectivity |
| Storage | iSCSI | Dedicated interface recommended |
| Synchronization | Geo replication | L3 allowed; one IP family throughout |

Only one NIC may connect to the cluster subnet; virtual interfaces are unsupported for cluster connectivity.[cite:4]

## Firewall

```bash
ams_updatefirewall
ams_updatefirewall <option>
```

Use `root` to apply and review mappings first.[cite:2][cite:4]

`ams.conf` bind categories:

```ini
AMSCLIENTBINDIP=<client-facing-list>
AMSCLUSTERBINDIP=<cluster-interface-or-address>
AMSGEOLOCALBINDIP=<geo-sync-interface-or-address>
AMSNEBINDIP=<NE-facing-list>
```

Before installation, INPUT/OUTPUT default policies must be `ACCEPT`; FORWARD may be `DROP`.[cite:4]

## NAT

`$AMSSOFTWAREHOME/conf/ams.conf`:

```ini
AMSCLIENTCONNECTIP=<translated-public-IP>
```

```bash
ams_server restart
```

Port translation is unsupported in documented designs; forward required ports unchanged.[cite:3][cite:4]

## Network reconfiguration

```bash
ams_cluster stop
ams_change_ip_subnet_server
ams_server start
ams_cluster deletehost <old-cluster-IP>
```

The cluster network is IPv4 only. Geo uses `ams_geo_configure.sh`.[cite:2][cite:3]

## SFTP port

```bash
ams_set_sftp_port.sh --check
ams_set_sftp_port.sh 2222
ams_server restart
ams_updatefirewall
```

Valid range: 1–65535.[cite:2]

## SNMP trap port

```bash
ams_set_snmp_trap_port
```

Coordinate NE destinations and firewall rules.[cite:2]

## FTP chroot

`/etc/vsftpd/vsftpd.conf`:

```conf
chroot_local_user=YES
allow_writeable_chroot=NO
```

Restart vsftpd after modification.[cite:4]
