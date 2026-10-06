# Security, Identity, and TLS — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Fall back to local authentication</strong></summary>

**In plain English:** Temporarily use local AMS accounts when LDAP/RADIUS is unavailable.

**Risk:** Security change.

**Before you run it:** Confirm external authentication is unavailable and plan restoration

```bash
ams_switch_authentication_local
```

</details>

<details>
<summary><strong>Create an encrypted credential file</strong></summary>

**In plain English:** Avoid plaintext manager-tool credentials in CSV input.

**Risk:** Sensitive input.

```bash
createUsernamePassword
```

</details>

<details>
<summary><strong>Rotate database passwords</strong></summary>

**In plain English:** Coordinate stopping, credential updates, and automatic cluster startup.

**Risk:** High; the script automatically restarts service.

**Before you run it:** Use `amssys` or root; password is at most 32 characters with no spaces; do not restart manually afterward

```bash
ams_update_database_pwd.sh
```

</details>

<details>
<summary><strong>Create the first administrator</strong></summary>

**In plain English:** Create the initial named human administrator and address filter.

**Risk:** Security-sensitive.

**Before you run it:** Avoid generic names such as `admin`, `helpdesk`, and `support`

```bash
ams_createfirstuser.sh <username> '<global-address-filter>'
ams_createfirstuser.sh --help
```

</details>

<details>
<summary><strong>Check or enable TLS</strong></summary>

**In plain English:** Verify encrypted service or enable it with a site keystore.

**Risk:** Changing TLS is service-affecting.

**Before you run it:** Use a site-owned production PKCS12 certificate and tested rollback

```bash
ams_check_ssl.sh
ams_server stop
ams_enable_ssl.sh /secure/ams-keystore.p12 '<password>'
ams_server start
```

</details>

<details>
<summary><strong>Rotate AES keys</strong></summary>

**In plain English:** Replace the key used to encrypt stored passwords and re-encrypt values.

**Risk:** High; affects stored secrets.

**Before you run it:** Stop application servers and back up affected data/files

```bash
ams_server stop
ams_recreate_aes_keys.sh
ams_recreate_aes_keys.sh -d <existing-key>
ams_recreate_aes_keys.sh -u <replacement-key>
ams_server start
```

</details>

<details>
<summary><strong>Configure SSH timeout</strong></summary>

**In plain English:** Set allowed SSH inactivity time consistently across servers.

**Risk:** Security configuration.

```bash
ams_configure_ssh_timeouts.sh check
ams_configure_ssh_timeouts.sh enable 10
ams_configure_ssh_timeouts.sh disable
```

</details>

<details>
<summary><strong>Protect service accounts from aging</strong></summary>

**In plain English:** Prevent required AMS service accounts from ordinary expiration lockout.

**Risk:** Security policy exception; requires approval.

```bash
sudo chage -I -1 -m 0 -M 99999 -E -1 amssys
sudo chage -I -1 -m 0 -M 99999 -E -1 amssftp
sudo chage -I -1 -m 0 -M 99999 -E -1 amsftp
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
