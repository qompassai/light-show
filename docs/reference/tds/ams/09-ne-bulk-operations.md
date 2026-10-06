# NE and Bulk Operations

## Agent inventory

```bash
getAgentlist.sh -u <username> -p
getAgentlist.sh -u <username> -p '<password>'
```

Prefer prompting instead of exposing passwords in process listings/history.[cite:2]

## Bulk NE create/modify

```bash
ams_ne_mgr [options] <input_file>
```

Generate the manager credential file with `createUsernamePassword`. IP-address modification cannot be combined with other attribute changes in one request.[cite:2]

## NE CLI transport

```bash
ams_ne_cli <NE-list-or-address> <input-command-file> <output-file> <timeout>
ams_ne_cli -protocol
ams_ne_cli -buffer
```

The command file contains NE-native CLI; validate against the specific NE command reference. Review targets because commands can be service-affecting.[cite:2]

## NE backup/restore

```bash
ams_nebackup.sh [options] <backupfile>
ams_nerestore.sh [options] <backupfile>
ams_nerestore.sh -b <backupfile>
```

`-b` restores only the NE-backup database.[cite:2]

## Lookup and placement

```bash
ams_retrieve_ip_by_nename.sh <NEname>
ams_retrieve_pap_ne_from_db.sh -o /tmp/ne-pap.txt
ams_show_g6_linked_ne.sh --all
ams_show_g6_linked_ne.sh <G6-1>,<G6-2>
ams_show_ne_balancing.sh --help
ams_show_ne_balancing.sh -a <application-server-IP>
ams_show_ne_balancing.sh -n <NE-name>
ams_show_ne_balancing.sh -i <NE-IP>
ams_show_ne_balancing.sh --netype <NE-type>
ams_show_ne_balancing.sh --netypereleasecount
```

[cite:2]

## Retrieve supervised NEs

```bash
retrieve_nes.sh -f /tmp/nes.csv -u <username> -p
```

IPv4 and IPv6 NEs are included.[cite:2]

## Link managers

```bash
ams_hub_sub_link_mgr [options] <input_file>
ams_link_mgr [options] <input_file>
ams_mediagw_mgr [options] <input_file>
ams_splitter_mgr [options] [input_file]
```

Default splitter input:

```text
$AMSLOCALDATAHOME/ossconf/amssplittermgr.csv
```

Managers require NBI edit privileges and support properties, encrypted credential, keystore, NBI-host, and action options.[cite:2]

## Supervision

```bash
ams_stop_supervision
```

Confirm targets and window because stopping supervision reduces management visibility.[cite:2]
