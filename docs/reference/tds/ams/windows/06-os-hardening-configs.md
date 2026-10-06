# RHEL Hardening Commands and Configs — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Set shell timeout and umask</strong></summary>

**In plain English:** Log out idle shells and make new files private by default.

**Risk:** Can end idle sessions or change permissions.

**Before you run it:** Check AMS support and site policy

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
# Edit /etc/profile:
TMOUT=1800
UMASK=027
```
exit
```

</details>

<details>
<summary><strong>Apply kernel hardening</strong></summary>

**In plain English:** Disable risky forwarding/redirect behavior and strengthen basic TCP handling.

**Risk:** Network-impacting if settings conflict with topology.

**Before you run it:** Review each value; do not paste blindly

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo vi /etc/sysctl.conf
# Add approved settings, then:
sudo sysctl -p
```
exit
```

</details>

<details>
<summary><strong>Remove legacy trust files</strong></summary>

**In plain English:** Remove per-user host trust and lock down the empty system trust file.

**Risk:** Can break legacy trust-based automation.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
rm -f ~/.rhosts ~/.netrc
sudo touch /etc/hosts.equiv
sudo chmod 400 /etc/hosts.equiv
```
exit
```

</details>

<details>
<summary><strong>Install and enable time sync</strong></summary>

**In plain English:** Keep clocks aligned for logs, certificates, and replication.

**Risk:** Changing time sources can affect service.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo yum install chrony
sudo systemctl enable --now chronyd
sudo vi /etc/chrony.conf
```
exit
```

</details>

<details>
<summary><strong>Cap shared memory</strong></summary>

**In plain English:** Limit `/dev/shm` so it cannot consume all RAM.

**Risk:** Wrong sizing can starve AMS or cause swapping.

**Before you run it:** Size according to supported guidance and real memory needs

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo mount -t tmpfs -o size=12G tmpfs /dev/shm
# Persistent /etc/fstab entry:
tmpfs /dev/shm tmpfs nodev,nosuid,size=12G 0 0
```
exit
```

</details>

<details>
<summary><strong>Enable required services</strong></summary>

**In plain English:** Start time sync, scheduling, and secure login now and at boot.

**Risk:** Read-only or low impact.

```powershell
ssh amssys@<ams-host>
# You are now at the remote Linux prompt.
```bash
sudo systemctl enable --now chronyd
sudo systemctl enable --now crond
sudo systemctl enable --now sshd
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
