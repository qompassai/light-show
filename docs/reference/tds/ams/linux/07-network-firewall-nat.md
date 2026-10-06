# Network, Firewall, and NAT — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Understand AMS network roles</strong></summary>

**In plain English:** Keep cluster, NE, client/NBI, storage, and geo-sync traffic in the right lanes.

**Risk:** Read-only or low impact.

- **Cluster:** isolated Layer 2, IPv4 only, same MTU, IGMP snooping off.
- **DCN/NE:** isolate from public/user networks.
- **Client/NBI:** protect with firewall/VPN.
- **Storage:** dedicated iSCSI interface recommended.
- **Geo sync:** Layer 3 allowed; use one IP family throughout.

</details>

<details>
<summary><strong>Review or apply firewall rules</strong></summary>

**In plain English:** Show and apply the approved AMS port mapping.

**Risk:** High; mistakes can cut off users, NEs, or cluster traffic.

**Before you run it:** Keep out-of-band access and review mappings first

```bash
sudo ams_updatefirewall
sudo ams_updatefirewall <option>
```

</details>

<details>
<summary><strong>Configure a translated client IP</strong></summary>

**In plain English:** Tell AMS the public/translated address clients use.

**Risk:** Connectivity change; documented designs do not support port translation.

```bash
# Edit $AMSSOFTWAREHOME/conf/ams.conf:
AMSCLIENTCONNECTIP=<translated-public-ip>
ams_server restart
```

</details>

<details>
<summary><strong>Change server IP or subnet</strong></summary>

**In plain English:** Stop, update network identity, restart, and remove the stale host record.

**Risk:** High and service-affecting.

**Before you run it:** Have console access, approved addresses/routes/DNS, and rollback

```bash
ams_cluster stop
sudo ams_change_ip_subnet_server
ams_server start
ams_cluster deletehost <old-cluster-ip>
```

</details>

<details>
<summary><strong>Change SFTP port</strong></summary>

**In plain English:** Set an approved AMS SFTP port and refresh firewall rules.

**Risk:** Connectivity change; restart required.

```bash
ams_set_sftp_port.sh --check
ams_set_sftp_port.sh 2222
ams_server restart
sudo ams_updatefirewall
```

</details>

<details>
<summary><strong>Set SNMP trap port</strong></summary>

**In plain English:** Choose the port where AMS receives traps.

**Risk:** Connectivity change.

**Before you run it:** Coordinate NE destinations and firewall rules

```bash
sudo ams_set_snmp_trap_port
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
