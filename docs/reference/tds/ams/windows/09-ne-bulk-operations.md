# NE and Bulk Operations — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>List agents safely</strong></summary>

**In plain English:** Retrieve agent inventory while prompting for the password.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
getAgentlist.sh -u <username> -p
```
exit
```

</details>

<details>
<summary><strong>Create or modify NEs in bulk</strong></summary>

**In plain English:** Apply many NE records from a prepared input file.

**Risk:** Provisioning change.

**Before you run it:** Validate CSV; an IP change cannot share a request with other attribute changes

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
createUsernamePassword
ams_ne_mgr [options] <input-file>
```
exit
```

</details>

<details>
<summary><strong>Run NE-native CLI in bulk</strong></summary>

**In plain English:** Transport a command file through AMS to selected NEs.

**Risk:** Potentially service-affecting.

**Before you run it:** Validate exact NE family/release commands and every target

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_ne_cli <ne-list-or-address> <input-command-file> <output-file> <timeout>
ams_ne_cli -protocol
ams_ne_cli -buffer
```
exit
```

</details>

<details>
<summary><strong>Back up or restore NE data</strong></summary>

**In plain English:** Save or restore AMS-held NE backup records.

**Risk:** Restore changes data.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_nebackup.sh [options] <backupfile>
ams_nerestore.sh [options] <backupfile>
ams_nerestore.sh -b <backupfile>
```
exit
```

</details>

<details>
<summary><strong>Look up NE placement</strong></summary>

**In plain English:** Resolve addresses, export PAP mapping, and inspect assignment.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_retrieve_ip_by_nename.sh <ne-name>
ams_retrieve_pap_ne_from_db.sh -o /tmp/ne-pap.txt
ams_show_g6_linked_ne.sh --all
ams_show_ne_balancing.sh -a <application-server-ip>
ams_show_ne_balancing.sh -n <ne-name>
ams_show_ne_balancing.sh -i <ne-ip>
ams_show_ne_balancing.sh --netypereleasecount
```
exit
```

</details>

<details>
<summary><strong>Export supervised NEs</strong></summary>

**In plain English:** Write IPv4 and IPv6 supervised NEs to CSV.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host> "retrieve_nes.sh -f /tmp/nes.csv -u <username> -p"
scp amssys@<ams-host>:/tmp/nes.csv .\nes.csv
```

</details>

<details>
<summary><strong>Manage topology objects</strong></summary>

**In plain English:** Bulk-manage links, gateways, and splitters from input files.

**Risk:** Provisioning change.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_hub_sub_link_mgr [options] <input-file>
ams_link_mgr [options] <input-file>
ams_mediagw_mgr [options] <input-file>
ams_splitter_mgr [options] [input-file]
```
exit
```

</details>

<details>
<summary><strong>Stop supervision</strong></summary>

**In plain English:** Stop AMS monitoring selected NEs.

**Risk:** Reduces management visibility.

**Before you run it:** Confirm targets and maintenance window

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_stop_supervision
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
