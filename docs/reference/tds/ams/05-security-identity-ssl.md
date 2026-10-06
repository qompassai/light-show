# Security, Identity, and TLS

## Authentication fallback

```bash
ams_switch_authentication_local
```

Use when LDAP/RADIUS failure prevents login. Restore intended external authentication after remediation.[cite:2]

## Encrypted credential file

```bash
createUsernamePassword
```

This interactively encrypts AMS credentials for the user, NE, link, and hub-subtended-link managers, avoiding plaintext credentials in CSV input.[cite:2]

## Database password rotation

```bash
ams_update_database_pwd.sh
```

Run as `amssys` or `root`. Passwords cannot exceed 32 characters or contain spaces. The script stops AMS, updates credentials/configuration, and automatically starts the cluster; do not manually restart afterward.[cite:2]

## Initial administrator

```bash
ams_createfirstuser.sh <username> '<global-address-filter>'
ams_createfirstuser.sh --help
```

Avoid generic operational usernames such as `admin`, `helpdesk`, and `support`.[cite:2][cite:4]

## TLS

```bash
ams_check_ssl.sh
ams_server stop
ams_enable_ssl.sh
ams_server start
```

Custom PKCS12 keystore:

```bash
ams_server stop
ams_enable_ssl.sh /secure/ams-keystore.p12 '<password>'
ams_server start
```

Disable only when explicitly required:

```bash
ams_server stop
ams_disable_ssl.sh
ams_server start
```

Nokia-provided certificates are for lab/trial use; production should use site certificates.[cite:2][cite:3]

## AES keys

```bash
ams_server stop
ams_recreate_aes_keys.sh
ams_recreate_aes_keys.sh -d <existing-key>
ams_recreate_aes_keys.sh -u <replacement-key>
ams_server start
```

This updates shared AES keys and re-encrypts stored passwords. Stop application servers and back up affected files first.[cite:2]

## NBI encryption

```bash
ams_nbi_encryption_key
```

Coordinate key changes with OSS consumers.[cite:2]

## SSH timeout

```bash
ams_configure_ssh_timeouts.sh check
ams_configure_ssh_timeouts.sh enable 10
ams_configure_ssh_timeouts.sh disable
```

The allowed timeout is 1–15 minutes. Apply the same setting to every cluster/geo server.[cite:2][cite:3]

## Service-account aging

```bash
chage -I -1 -m 0 -M 99999 -E -1 amssys
chage -I -1 -m 0 -M 99999 -E -1 amssftp
chage -I -1 -m 0 -M 99999 -E -1 amsftp
```

AMS service accounts must not be disabled by password aging/lockout policy.[cite:4]
