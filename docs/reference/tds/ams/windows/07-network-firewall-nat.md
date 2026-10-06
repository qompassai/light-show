# Network, Firewall, and NAT — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Understand AMS network roles</strong></summary>

**In plain English:** Keep cluster, NE, client/NBI, storage, and geo-sync traffic in the right lanes.

**Risk:** Read-only or low impact.

Windows should reach only the approved management/client network. It does not belong on the isolated AMS cluster network.

</details>

<details>
<summary><strong>Review or apply firewall rules</strong></summary>

**In plain English:** Show and apply the approved AMS port mapping.

**Risk:** High; mistakes can cut off users, NEs, or cluster traffic.

**Before you run it:** Keep out-of-band access and review mappings first

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo ams_updatefirewall
sudo ams_updatefirewall <option>
```
exit
```

</details>

<details>
<summary><strong>Configure a translated client IP</strong></summary>

**In plain English:** Tell AMS the public/translated address clients use.

**Risk:** Connectivity change; documented designs do not support port translation.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
# Edit $AMSSOFTWAREHOME/conf/ams.conf:
AMSCLIENTCONNECTIP=<translated-public-ip>
ams_server restart
```
exit
```

</details>

<details>
<summary><strong>Change server IP or subnet</strong></summary>

**In plain English:** Stop, update network identity, restart, and remove the stale host record.

**Risk:** High and service-affecting.

**Before you run it:** Have console access, approved addresses/routes/DNS, and rollback

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_cluster stop
sudo ams_change_ip_subnet_server
ams_server start
ams_cluster deletehost <old-cluster-ip>
```
exit
```

</details>

<details>
<summary><strong>Change SFTP port</strong></summary>

**In plain English:** Set an approved AMS SFTP port and refresh firewall rules.

**Risk:** Connectivity change; restart required.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_set_sftp_port.sh --check
ams_set_sftp_port.sh 2222
ams_server restart
sudo ams_updatefirewall
```
exit
```

</details>

<details>
<summary><strong>Set SNMP trap port</strong></summary>

**In plain English:** Choose the port where AMS receives traps.

**Risk:** Connectivity change.

**Before you run it:** Coordinate NE destinations and firewall rules

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo ams_set_snmp_trap_port
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
