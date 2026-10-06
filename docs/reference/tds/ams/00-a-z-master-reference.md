# AMS 9.8.3 A–Z Master Reference

This master index is alphabetized by operational subject. Each entry includes executable syntax or an exact configuration key where the supplied manuals provide one.

## A — Activation and AES

```bash
sudo /opt/ams/software/<release>/bin/ams_activate.sh
sudo -iu amssys ams_server stop
sudo -iu amssys ams_recreate_aes_keys.sh
sudo -iu amssys ams_server start
```

Inspect or supply an AES key:

```bash
ams_recreate_aes_keys.sh -d <existing-key>
ams_recreate_aes_keys.sh -u <replacement-key>
```

AES rotation is high risk because the script updates encrypted passwords in database and AMS configuration files.[cite:2]

## B — Backup

```bash
ams_backup.sh /backup/ams-data.tar
ams_backup.sh -z /backup/ams-data.tar.gz
ams_backup.sh -c /backup/ams-data.tar
ams_backup.sh -f /backup/ams-data.tar
sudo ams_sw_backup.sh /backup/ams-software
```

Supported remote destination forms:

```text
ftp://<user>:<password>@<host>/<path>/<file>
sftp://<user>:<password>@<host>/<path>/<file>
sftp://<host>/<path>/<file>
```

[cite:2]

## C — Cluster

```bash
ams_cluster status
ams_cluster status --detailed
ams_cluster status sw
ams_cluster start
ams_cluster stop
ams_cluster restart
ams_cluster switch active
ams_cluster switch standby
ams_cluster evacuate_ne <cluster-IP>
ams_cluster unevacuate_ne <cluster-IP> <weight>
ams_cluster deletehost <old-cluster-IP>
```

[cite:2]

## D — Database

```bash
$AMSSCRIPTSDIR/ams_db_defragment.sh all analyse
$AMSSCRIPTSDIR/ams_db_defragment.sh -t <table> analyse
ams_server stop
$AMSSCRIPTSDIR/ams_db_defragment.sh all execute
ams_server start
ams_update_database_pwd.sh
```

`execute` requires a stopped AMS on the active data server.[cite:2]

## E — Endpoint Verification

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/ManagedElementMgr
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
```

[cite:6]

## F — Firewall and FTP

```bash
sudo ams_updatefirewall
sudo ams_updatefirewall <option>
sudo systemctl restart vsftpd
```

`/etc/vsftpd/vsftpd.conf`:

```conf
chroot_local_user=YES
allow_writeable_chroot=NO
```

[cite:2][cite:4]

## G — Geographic Redundancy

```bash
ams_geo_configure.sh
ams_cluster start -force active
ams_cluster start -force standby
ams_cluster switch active
ams_cluster switch standby
ams_server resetgeo
```

`$AMSSOFTWAREHOME/conf/amsgeomonitor.conf`:

```ini
PINGPONGPROTECTIONTIMEOUT=<minutes>
```

[cite:2][cite:3]

## H — Hardening

```bash
sudo sysctl -p
sudo chmod 400 /etc/hosts.equiv
sudo systemctl enable --now chronyd crond sshd
```

Core `/etc/sysctl.conf` controls:

```ini
kernel.core_uses_pid = 1
kernel.sysrq = 0
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0
net.ipv4.tcp_syncookies = 1
```

[cite:4]

## I — Installation and Import

```bash
ams_install.sh
ams_install.sh --installActivate /staging/components
ams_install.sh --install /staging/components
ams_install.sh --activate /staging/components
ams_install.sh --deactivate
ams_import.sh -filename /path/export.tar -overwrite
```

[cite:2][cite:3]

## J — JMS

```bash
JMSExpiryConfigurator.sh <option>
```

Use the script's help/output on the installed release for the accepted expiry option values; JMS notifications are part of the NBI.[cite:2][cite:6]

## K — Keystore

```bash
ams_server stop
ams_enable_ssl.sh /secure/ams-keystore.p12 '<keystore-password>'
ams_server start
ams_check_ssl.sh
```

Use a site-owned PKCS12 keystore in production.[cite:2][cite:3]

## L — Licenses, Links, and Logs

```bash
ams_install_license
getLicenseCounter
ams_link_mgr [options] <input_file>
ams_hub_sub_link_mgr [options] <input_file>
ams_log_manager.sh --collect --category all --target all --destination file:///tmp/ams-logs.tar
ams_reset_logs.sh
```

[cite:2]

## M — Migration and Media Gateways

```bash
ams_copy_datafiles --force
ams_copy_datafiles --force --from-release <previous-release>
ams_copy_datafiles --force --from-backup /absolute/path/backup.tar
ams_mediagw_mgr [options] <input_file>
```

`--overwrite` variants delete current persistency before copying and are reserved for exceptional recovery.[cite:2][cite:3]

## N — NAT, NBI, and NE Tools

`$AMSSOFTWAREHOME/conf/ams.conf`:

```ini
AMSCLIENTCONNECTIP=<translated-public-IP>
AMSCLIENTBINDIP=<client-interface-or-address-list>
AMSNEBINDIP=<NE-interface-or-address-list>
```

Commands:

```bash
ams_nbi_encryption_key
ams_ne_mgr [options] <input_file>
ams_ne_cli <NE-list> <command-file> <output-file> <timeout>
ams_nebackup.sh [options] <backupfile>
ams_nerestore.sh [options] <backupfile>
```

[cite:2][cite:3]

## O — OS Limits

```bash
ams_update_limit.conf.sh
```

Resulting `/etc/security/limits.conf` entries:

```conf
amssys soft nproc 650000
amssys hard nproc 650000
amssys soft nofile 650000
amssys hard nofile 650000
```

[cite:2]

## P — PAP and Passwords

```bash
ams_retrieve_pap_ne_from_db.sh -o /tmp/ne-pap.txt
createUsernamePassword
ams_createfirstuser.sh <username> '<global-address-filter>'
```

[cite:2]

## Q — Query and Quick Health Check

```bash
ams_server status
ams_cluster status --detailed
ams_show_ne_balancing.sh --netypereleasecount
getLicenseCounter
innotop
```

These read-oriented commands form a quick pre-maintenance evidence set.[cite:2]

## R — Restore and Recovery

```bash
ams_restore.sh /backup/ams-data.tar
ams_restore.sh -n /backup/ams-data.tar
ams_nerestore.sh -b /backup/ne-backup.tar
ams_support.sh --domain security --command killadminsessions
ams_support.sh --domain security --command resetadminpwd
```

Destructive database reinitialization:

```bash
ams_server stop
ams_remove_data.sh
ams_server start
```

[cite:2]

## S — Server, SFTP, SNMP, SSL, and Support

```bash
ams_server start
ams_server stop
ams_server restart
ams_server status
ams_set_sftp_port.sh --check
ams_set_sftp_port.sh 2222
ams_set_snmp_trap_port
ams_check_ssl.sh
ams_support.sh --domain app --command jstack --target all --destination /tmp/jstack.tar
```

[cite:2]

## T — Time, TLS, and Tracing

```bash
systemctl enable --now chronyd
ams_server stop
ams_enable_ssl.sh
ams_server start
ams_tracing
```

`/etc/chrony.conf` client example:

```conf
server <site-ntp-host> iburst
driftfile /var/lib/chrony/drift
logdir /var/log/chrony
```

[cite:2][cite:4]

## U — Uninstall and Users

```bash
/opt/ams/software/<release>/bin/ams_uninstall -f
ams_user_mgr [options] <input_file>
```

Uninstall is destructive. Back up data/software and verify the target release path first.[cite:2][cite:3]

## V — Version Verification

```bash
ams_server version
ams_server version save
ams_server version save --label GOLDEN983 /secure/GoldenEMSSwConfig
ams_server version verify /secure/GoldenEMSSwConfig
ams_cluster status sw
```

[cite:2]

## W — WSDL and Web Services

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/EquipmentProvisioningMgr
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/TopologicalLinkControlMgr
```

Generate and validate SOAP envelopes from the activated schema/WSDL.[cite:6]

## X — XML SOAP Submission

```bash
curl --fail-with-body   --cacert /path/ams-ca.pem   --user '<nbi-user>:<password>'   --header 'Content-Type: text/xml; charset=utf-8'   --data-binary @request.xml   'https://<host>:8443/ams/services/<ServiceName>'
```

Use an approved secret source instead of a literal production password.

## Y — Yum and Required Packages

```bash
yum install chrony
systemctl enable --now chronyd
```

Install the RHEL package set specified for the exact supported OS release before AMS installation; the complete list remains in the Server Configuration Technical Guidelines.[cite:4]

## Z — Zero/Destructive Guardrails

Before any command marked destructive:

```bash
ams_cluster status --detailed
ams_server version
ams_backup.sh -z /backup/prechange-ams.tar.gz
```

Do not run `ams_remove_data.sh`, `ams_copy_datafiles --overwrite`, uninstall, or restore without a validated rollback artifact and maintenance authorization.[cite:2][cite:3]
