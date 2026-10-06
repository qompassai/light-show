# Server, Cluster, and Geo Operations — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Check cluster health</strong></summary>

**In plain English:** Ask the cluster which nodes are up, their roles, and whether software agrees.

**Risk:** Read-only or low impact.

```bash
ams_cluster status
ams_cluster status --detailed
ams_cluster status sw
ams_cluster status -l 10 -p 60
```

**Success looks like:** Every expected node appears in the correct stable role.

</details>

<details>
<summary><strong>Start, stop, or restart the cluster</strong></summary>

**In plain English:** Control all cluster servers as one unit.

**Risk:** Service-affecting; stop and restart place servers in maintenance mode.

**Before you run it:** Confirm roles, backup, maintenance approval, and rollback plan

```bash
ams_cluster start
ams_cluster stop
ams_cluster restart
```

</details>

<details>
<summary><strong>Control geographic roles</strong></summary>

**In plain English:** Choose which geo site serves traffic and which waits as standby.

**Risk:** High; incorrect forced use can create dual-active service.

**Before you run it:** Verify the remote site and current role from both sites

```bash
ams_cluster start -force active
ams_cluster start -force standby
ams_cluster switch active
ams_cluster switch standby
ams_cluster switch -force active
ams_cluster switch -force standby
```

**Success looks like:** Exactly one site is active and the other is standby.

</details>

<details>
<summary><strong>Evacuate an application server</strong></summary>

**In plain English:** Move NE responsibility away before host maintenance.

**Risk:** Changes NE placement and available capacity.

**Before you run it:** Record current placement and confirm remaining capacity

```bash
ams_cluster evacuate_ne <cluster-ip>
ams_show_ne_balancing.sh -a <cluster-ip>
# Return later:
ams_cluster unevacuate_ne <cluster-ip> <weight>
```

</details>

<details>
<summary><strong>Control one server</strong></summary>

**In plain English:** Inspect or control one AMS server instead of the whole cluster.

**Risk:** Stop/restart can be service-affecting.

```bash
ams_server start
ams_server stop
ams_server stop maintenance
ams_server restart
ams_server status
ams_server status all -l 10 -p 60
ams_server version
```

</details>

<details>
<summary><strong>Configure geo redundancy</strong></summary>

**In plain English:** Run guided setup for site roles, remote data servers, sync network, and automatic switchover.

**Risk:** High and service-affecting.

**Before you run it:** Have an approved topology and one consistent IP family for all geo addresses

```bash
ams_geo_configure.sh
# Timer file:
$AMSSOFTWAREHOME/conf/amsgeomonitor.conf
# Clear current timer only after remediation:
ams_server resetgeo
```

</details>

<details>
<summary><strong>Use the correct startup order</strong></summary>

**In plain English:** Start dependencies before services that rely on them.

**Risk:** Read-only or low impact.

1. Fresh cluster: start each server individually the first time.
2. Normal cluster: preferred data server, non-preferred data server, then application servers.
3. Geo: verify active before standby.
4. Migration: follow the scenario-specific order.

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
