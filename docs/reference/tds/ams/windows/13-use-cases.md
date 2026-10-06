# Operational Use Cases — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Activate a release</strong></summary>

**In plain English:** Select a release, start AMS, and verify it responds.

**Risk:** Service/configuration change.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo /opt/ams/software/<release>/bin/ams_activate.sh
sudo -iu amssys ams_server start
sudo -iu amssys ams_server status
```
exit
```

</details>

<details>
<summary><strong>Take a pre-change backup</strong></summary>

**In plain English:** Capture health, version, and compressed rollback data.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_cluster status --detailed
ams_server version
ams_backup.sh -z /backup/prechange-ams.tar.gz
```
exit
```

</details>

<details>
<summary><strong>Collect a support bundle</strong></summary>

**In plain English:** Capture status, logs, and JVM thread stacks.

**Risk:** May contain sensitive data; JVM capture adds load.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_cluster status --detailed
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/ams-support.tar
ams_support.sh --domain app --command jstack --target all --destination /tmp/ams-jstack.tar
```
exit
```

</details>

<details>
<summary><strong>Evacuate for maintenance</strong></summary>

**In plain English:** Move NE work away, stop in maintenance mode, return, and rebalance.

**Risk:** Capacity and placement impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_cluster status --detailed
ams_cluster evacuate_ne <cluster-ip>
ams_show_ne_balancing.sh -a <cluster-ip>
ams_server stop maintenance
# Return:
ams_server start
ams_cluster unevacuate_ne <cluster-ip> 1
ams_cluster status --detailed
```
exit
```

</details>

<details>
<summary><strong>Restore on different hardware</strong></summary>

**In plain English:** Restore without old licenses, install new-host licenses, and start.

**Risk:** High and service-affecting.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_restore.sh -n /backup/ams-data.tar
sudo ams_install_license
ams_server start
```
exit
```

</details>

<details>
<summary><strong>Validate installed software</strong></summary>

**In plain English:** Save or compare a known-good component set.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_server version save --label GOLDEN983 /secure/GoldenEMSSwConfig
ams_server version verify /secure/GoldenEMSSwConfig
ams_cluster status sw
```
exit
```

</details>

<details>
<summary><strong>Verify NBI from workstation</strong></summary>

**In plain English:** Check user service and schema before SOAP debugging.

**Risk:** Read-only or low impact.

```powershell
curl.exe --cacert C:\path\ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
curl.exe --cacert C:\path\ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
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
