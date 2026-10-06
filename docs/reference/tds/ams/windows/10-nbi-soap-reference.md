# NBI SOAP Reference — Windows PowerShell/CMD

This edition starts from Windows PowerShell or Command Prompt. AMS server commands still run on the remote Linux host, normally over SSH; native Windows examples are used for HTTPS and file transfer.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Verify an endpoint</strong></summary>

**In plain English:** Check HTTPS reachability and certificate trust.

**Risk:** Read-only or low impact.

```powershell
curl.exe --cacert C:\path\ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
curl.exe --cacert C:\path\ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
```

**Success looks like:** TLS succeeds and the Axis service/schema responds.

</details>

<details>
<summary><strong>Choose the endpoint</strong></summary>

**In plain English:** Send each operation to the manager that owns that object.

**Risk:** Read-only or low impact.

| Task | Endpoint |
|---|---|
| NE create/modify/delete | `/ams/services/EquipmentProvisioningMgr` |
| NE retrieval | `/ams/services/ManagedElementMgr` |
| Supervision | `/ams/services/ManagedElementMgrExtns` |
| Links | `/ams/services/TopologicalLinkControlMgr` |
| Users | `/ams/services/UserManagementMgr` |

</details>

<details>
<summary><strong>Submit SOAP XML</strong></summary>

**In plain English:** Post an XML envelope file to an AMS service.

**Risk:** Varies; calls can change users, NEs, links, or supervision.

**Before you run it:** Generate exact namespaces/headers from active WSDL and use approved secrets

```powershell
$Credential = Get-Credential
$pair = '{0}:{1}' -f $Credential.UserName, $Credential.GetNetworkCredential().Password
$basic = [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes($pair))
Invoke-WebRequest -Uri 'https://<host>:8443/ams/services/<ServiceName>' -Method Post -Headers @{Authorization = "Basic $basic"} -ContentType 'text/xml; charset=utf-8' -InFile .\request.xml
```

</details>

<details>
<summary><strong>Read common exceptions</strong></summary>

**In plain English:** Translate a machine error into the first troubleshooting question.

**Risk:** Read-only or low impact.

| Exception | Meaning |
|---|---|
| `EXCPTACCESSDENIED` | Authorization denied |
| `EXCPTCAPACITYEXCEEDED` | Too many concurrent operations |
| `EXCPTCOMMFAILURE` | Communication failed |
| `EXCPTENTITYNOTFOUND` | Object absent |
| `EXCPTINVALIDINPUT` | Malformed, unsupported, or out-of-range input |
| `EXCPTUNABLETOCOMPLY` | Valid request cannot complete |

</details>

## Source key

- `[cite:1]` User Guide
- `[cite:2]` Administrator Guide
- `[cite:3]` Installation and Migration Guide
- `[cite:4]` Server Configuration Technical Guidelines
- `[cite:5]` Glossary
- `[cite:6]` Northbound Interface Guide
- `[cite:7]` Release Notice
