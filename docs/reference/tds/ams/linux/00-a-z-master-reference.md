# AMS 9.8.3 A–Z Master Reference — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>A — Activation and AES</strong></summary>

**In plain English:** Activate a release or rotate encryption keys.

**Risk:** Varies; inspect the topic report before any change.

```bash
sudo /opt/ams/software/<release>/bin/ams_activate.sh
ams_recreate_aes_keys.sh
```

</details>

<details>
<summary><strong>B — Backup</strong></summary>

**In plain English:** Create data/software rollback copies.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_backup.sh -z /backup/ams-data.tar.gz
sudo ams_sw_backup.sh /backup/ams-software
```

</details>

<details>
<summary><strong>C — Cluster</strong></summary>

**In plain English:** Inspect/control the server team.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_cluster status --detailed
ams_cluster start
ams_cluster stop
```

</details>

<details>
<summary><strong>D — Database</strong></summary>

**In plain English:** Analyze fragmentation or rotate DB credentials.

**Risk:** Varies; inspect the topic report before any change.

```bash
$AMSSCRIPTSDIR/ams_db_defragment.sh all analyse
ams_update_database_pwd.sh
```

</details>

<details>
<summary><strong>E — Endpoints</strong></summary>

**In plain English:** Check NBI HTTPS services.

**Risk:** Varies; inspect the topic report before any change.

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
```

</details>

<details>
<summary><strong>F — Firewall</strong></summary>

**In plain English:** Apply approved port rules.

**Risk:** Varies; inspect the topic report before any change.

```bash
sudo ams_updatefirewall
```

</details>

<details>
<summary><strong>G — Geo</strong></summary>

**In plain English:** Configure/switch active and standby sites.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_geo_configure.sh
ams_cluster switch active
```

</details>

<details>
<summary><strong>H — Hardening</strong></summary>

**In plain English:** Apply approved RHEL security settings.

**Risk:** Varies; inspect the topic report before any change.

```bash
sudo sysctl -p
sudo chmod 400 /etc/hosts.equiv
```

</details>

<details>
<summary><strong>I — Installation</strong></summary>

**In plain English:** Manage components or import data.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_install.sh --installActivate <directory>
ams_import.sh -filename <path> -overwrite
```

</details>

<details>
<summary><strong>J — JMS</strong></summary>

**In plain English:** Configure message expiry.

**Risk:** Varies; inspect the topic report before any change.

```bash
JMSExpiryConfigurator.sh <option>
```

</details>

<details>
<summary><strong>K — Keystore</strong></summary>

**In plain English:** Enable TLS with site PKCS12.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_enable_ssl.sh /secure/ams-keystore.p12 <password>
```

</details>

<details>
<summary><strong>L — Licenses, links, logs</strong></summary>

**In plain English:** Manage entitlements, links, and evidence.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_install_license
ams_link_mgr [options] <input-file>
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/logs.tar
```

</details>

<details>
<summary><strong>M — Migration</strong></summary>

**In plain English:** Copy persistency.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_copy_datafiles --force --from-backup /absolute/path/backup.tar
```

</details>

<details>
<summary><strong>N — NBI and NE tools</strong></summary>

**In plain English:** Manage encryption and network elements.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_nbi_encryption_key
ams_ne_mgr [options] <input-file>
ams_ne_cli <ne-list> <cmd-file> <output> <timeout>
```

</details>

<details>
<summary><strong>O — OS limits</strong></summary>

**In plain English:** Raise process/file limits.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_update_limit.conf.sh
```

</details>

<details>
<summary><strong>P — PAP and passwords</strong></summary>

**In plain English:** Export mapping or create credentials.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_retrieve_pap_ne_from_db.sh -o /tmp/ne-pap.txt
createUsernamePassword
```

</details>

<details>
<summary><strong>Q — Quick health</strong></summary>

**In plain English:** Collect a fast status picture.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_server status
ams_cluster status --detailed
getLicenseCounter
```

</details>

<details>
<summary><strong>R — Restore/recovery</strong></summary>

**In plain English:** Restore backups or admin access.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_restore.sh /backup/ams-data.tar
ams_support.sh --domain security --command resetadminpwd
```

</details>

<details>
<summary><strong>S — Server and support</strong></summary>

**In plain English:** Control services/interfaces.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_server status
ams_set_sftp_port.sh --check
ams_check_ssl.sh
```

</details>

<details>
<summary><strong>T — Time/TLS/tracing</strong></summary>

**In plain English:** Align time and inspect tracing.

**Risk:** Varies; inspect the topic report before any change.

```bash
systemctl enable --now chronyd
ams_tracing
```

</details>

<details>
<summary><strong>U — Uninstall/users</strong></summary>

**In plain English:** Remove release or bulk-manage users.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_uninstall -f
ams_user_mgr [options] <input-file>
```

</details>

<details>
<summary><strong>V — Version</strong></summary>

**In plain English:** Compare software to golden set.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_server version
ams_server version verify /path/GoldenEMSSwConfig
```

</details>

<details>
<summary><strong>W — WSDL</strong></summary>

**In plain English:** Build calls from active schema.

**Risk:** Varies; inspect the topic report before any change.

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
```

</details>

<details>
<summary><strong>X — XML SOAP</strong></summary>

**In plain English:** Post a prepared envelope.

**Risk:** Varies; inspect the topic report before any change.

```bash
curl --data-binary @request.xml 'https://<host>:8443/ams/services/<ServiceName>'
```

</details>

<details>
<summary><strong>Y — Yum</strong></summary>

**In plain English:** Install supported prerequisites.

**Risk:** Varies; inspect the topic report before any change.

```bash
sudo yum install chrony
```

</details>

<details>
<summary><strong>Z — Zero/destructive guardrails</strong></summary>

**In plain English:** Pause before erase/overwrite/force.

**Risk:** Varies; inspect the topic report before any change.

```bash
ams_cluster status --detailed
ams_server version
ams_backup.sh -z /backup/prechange-ams.tar.gz
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
