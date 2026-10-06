# NBI SOAP Reference

## Verify endpoints

```bash
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/services/UserManagementMgr
curl --cacert /path/ams-ca.pem https://<host>:8443/ams/schema/doc/html/index.html
```

A functional Axis service returns its service message.[cite:6]

## Endpoints

| Family | Endpoint |
|---|---|
| NE create/modify/delete | `https://<host>:8443/ams/services/EquipmentProvisioningMgr` |
| NE retrieval | `https://<host>:8443/ams/services/ManagedElementMgr` |
| Supervision | `https://<host>:8443/ams/services/ManagedElementMgrExtns` |
| Topological links | `https://<host>:8443/ams/services/TopologicalLinkControlMgr` |
| Users | `https://<host>:8443/ams/services/UserManagementMgr` |

[cite:6]

## User operations

- `addUser`
- `deleteUser`
- `expirePassword`
- `listUser`
- `modifyUser`
- `resetPassword`
- `resumeUser`
- `suspendUser`

## NE operations

- `createManagedElement`
- `createManagedObject`
- `createTopologicalLink`
- `deleteManagedElement`
- `deleteManagedObject`
- `deleteTopologicalLink`
- `disableMaintenanceMode`
- `enableMaintenanceMode`
- `getAllManagedElements`
- `getAllManagedElementsIterator`
- `getManagedElement`
- `getTopologicalLink`
- `getTopologicalLinks`
- `modifyManagedElement`
- `modifyManagedObject`
- `startSupervision`
- `stopSupervision`

## Test operations

- `createMaintenanceAssociation`
- `createMaintenanceDomain`
- `createMaintenancePoint`
- `createMaintenancePointList`
- `deleteMaintenancePointList`
- `executeAction`
- `getStatus`
- `getTP`

Requests and responses use SOAP XML envelopes. Use the activated server's schema/WSDL for exact namespaces and headers.[cite:6]

## Generic transport wrapper

```bash
curl --fail-with-body   --cacert /path/ams-ca.pem   --user '<nbi-user>:<password>'   --header 'Content-Type: text/xml; charset=utf-8'   --data-binary @request.xml   'https://<host>:8443/ams/services/<ServiceName>'
```

Do not place production secrets in shell history.

## Exceptions

| Exception | Meaning |
|---|---|
| `EXCPTACCESSDENIED` | Authorization denied |
| `EXCPTCAPACITYEXCEEDED` | Concurrent-operation capacity reached |
| `EXCPTCOMMFAILURE` | Communication failure |
| `EXCPTENTITYNOTFOUND` | Object absent |
| `EXCPTINTERNALERROR` | Internal error |
| `EXCPTINVALIDINPUT` | Invalid/unsupported/out-of-range input |
| `EXCPTNOTIMPLEMENTED` | Operation not implemented |
| `EXCPTUNABLETOCOMPLY` | Valid request cannot be completed |

[cite:6]
