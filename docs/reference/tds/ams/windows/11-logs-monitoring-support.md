# Logs, Monitoring, and Support — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Watch status</strong></summary>

**In plain English:** Poll health during startup, migration, recovery, or switchover.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_server status
ams_server status all -l 10 -p 60
ams_cluster status
ams_cluster status --detailed
ams_cluster status -l 10 -p 60
innotop
```
exit
```

</details>

<details>
<summary><strong>Handle DB fragmentation</strong></summary>

**In plain English:** Analyze waste online; rebuild only while AMS is stopped on active data server.

**Risk:** Execute is service-affecting and triggers full synchronization.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
$AMSSCRIPTSDIR/ams_db_defragment.sh all analyse
$AMSSCRIPTSDIR/ams_db_defragment.sh -t <table> analyse
ams_server stop
$AMSSCRIPTSDIR/ams_db_defragment.sh all execute
ams_server start
```
exit
```

</details>

<details>
<summary><strong>Collect logs</strong></summary>

**In plain English:** Bundle AMS log, debug, and OS evidence.

**Risk:** Evidence can contain sensitive operational data.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/ams-logs.tar
exit
scp amssys@<ams-host>:/tmp/ams-logs.tar .\ams-logs.tar
exit
```

</details>

<details>
<summary><strong>Reset or change logging</strong></summary>

**In plain English:** Clear selected logs or change verbosity.

**Risk:** Reset destroys evidence; verbose logging adds load and disk use.

**Before you run it:** Collect evidence and record original level

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_log_manager.sh --resetlogs --category log --target all
ams_log_manager.sh --setlevel '<category>,<level>' --target all
ams_reset_logs.sh
```
exit
```

</details>

<details>
<summary><strong>Collect JVM diagnostics</strong></summary>

**In plain English:** Capture thread or memory evidence.

**Risk:** Can temporarily stall a loaded JVM.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_support.sh --domain app --command jstack --target all --destination /tmp/jstack.tar
ams_support.sh --domain app --command jmap --target <site-or-ip> --destination /tmp/jmap.tar
```
exit
```

</details>

<details>
<summary><strong>Recover administrator access</strong></summary>

**In plain English:** Terminate stuck admin sessions or reset admin password.

**Risk:** Security-sensitive.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
ams_support.sh --domain security --command killadminsessions
ams_support.sh --domain security --command resetadminpwd
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
