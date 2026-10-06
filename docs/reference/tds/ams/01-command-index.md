# Alphabetical Command Index

## A–C

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_activate.sh` | `root` | Activate installed AMS release | Configuration |
| `ams_apps_stats_converter` | `amssys` | Convert application statistics | Read/convert |
| `ams_audit_agent_alarm <ID>` | `amssys` | Reset agent alarm counter | Targeted change |
| `ams_backup.sh [options] <URL>` | `amssys` | Back up AMS data | I/O intensive |
| `ams_change_ip_subnet_server` | `amssys`/`root` | Change client, cluster, NE networks | **Service-affecting** |
| `ams_check_ssl.sh` | `amssys` | Check JBoss and SSL state | Read-only |
| `ams_cleanup_sip_data_and_files.sh [-d]` | `amssys` | Remove orphaned SIP data/files | Data cleanup |
| `ams_cluster <option>` | `amssys` | Cluster lifecycle, status, switching | Varies |
| `ams_configure_ssh_timeouts.sh ...` | privileged | Configure SSH timeout | Security config |
| `ams_copy_datafiles ...` | privileged | Copy migration persistency | **High risk** |
| `ams_createfirstuser.sh ...` | privileged | Create initial administrator | Security |

## D–M

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_db_defragment.sh ...` | `root` | Analyze/defragment DB tables | Execute is **service-affecting** |
| `ams_disable_ssl.sh` | `amssys` | Disable TLS | **Service-affecting** |
| `ams_enable_ssl.sh ...` | `amssys` | Enable TLS | **Service-affecting** |
| `ams_export ...` | `amssys` | Export data | Writes/overwrites file |
| `ams_exttl1gw_integration.sh ...` | `amssys` | External TL1 Gateway integration | Varies |
| `ams_geo_configure.sh` | `amssys` | Configure geo redundancy | **Service-affecting** |
| `ams_group_dep` | `amssys` | Group dependency operation | Configuration |
| `ams_hub_sub_link_mgr ...` | `amssys` | Bulk hub/subtended links | Provisioning |
| `ams_import.sh ...` | `amssys` | Import data | **Data-changing** |
| `ams_install.sh ...` | `amssys` | Manage solution components | Varies |
| `ams_install_license` | privileged | Install licenses | Configuration |
| `ams_link_mgr ...` | `amssys` | Bulk links | Provisioning |
| `ams_log_manager.sh ...` | `amssys` | Configure/collect/reset logs | Varies |
| `ams_mediagw_mgr ...` | `amssys` | Bulk media gateways | Provisioning |

## N–R

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_nbi_encryption_key` | privileged | Regenerate NBI key | Security |
| `ams_ne_cli ...` | `amssys` | Run command files against NEs | Potential service impact |
| `ams_ne_mgr ...` | `amssys` | Create/modify NEs from CSV | Provisioning |
| `ams_nebackup.sh ...` | `amssys` | Back up NE data | NE operation |
| `ams_nerestore.sh ...` | `amssys` | Restore NE data | **NE-changing** |
| `ams_recreate_aes_keys.sh ...` | `amssys` | Rotate password AES keys | **High risk** |
| `ams_remove_data.sh` | `amssys` | Purge/reinitialize DB/data | **Destructive** |
| `ams_renew_isam_ssh_info` | privileged | Refresh cached ISAM SSH keys | Connectivity |
| `ams_reset_logs.sh` | `amssys` | Clear AMS logs | Destructive to logs |
| `ams_restore.sh ...` | `amssys` | Restore AMS data | **High risk** |
| `ams_retrieve_ip_by_nename.sh ...` | `amssys` | Resolve NE name to IP | Read-only |
| `ams_retrieve_pap_ne_from_db.sh ...` | `amssys` | Export NE/PAP mapping | Read-only |

## S–Z

| Command | Account | Use | Impact |
|---|---|---|---|
| `ams_schedule_backup ...` | `amssys` | Schedule backups | Scheduling |
| `ams_server ...` | `amssys` | Server lifecycle/status/version | Varies |
| `ams_set_sftp_port.sh ...` | privileged | Change SFTP port | Restart required |
| `ams_set_snmp_trap_port` | privileged | Set trap port | Connectivity |
| `ams_show_g6_linked_ne.sh ...` | `amssys` | Show G6-linked NEs | Read-only |
| `ams_show_ne_balancing.sh ...` | `amssys` | Show NE placement | Read-only |
| `ams_simplex_to_cluster.sh` | `amssys` | Convert topology | **Service-affecting** |
| `ams_splitter_mgr ...` | `amssys` | Bulk splitter objects | Provisioning |
| `ams_stop_supervision` | `amssys` | Stop supervision | Visibility impact |
| `ams_support.sh ...` | `amssys` | Diagnostics/security actions | Varies |
| `ams_sw_backup.sh ...` | `root` | Back up AMS software | I/O intensive |
| `ams_switch_active_dataserver ...` | `amssys` | Switch active DB | **Service-affecting** |
| `ams_switch_authentication_local` | `amssys` | Fall back to local auth | Security |
| `ams_tracing` | `amssys` | Configure tracing | Logging overhead |
| `ams_uninstall` | privileged | Uninstall AMS | **Destructive** |
| `ams_update_database_pwd.sh` | privileged | Rotate DB passwords | **Auto-restarts** |
| `ams_update_limit.conf.sh` | non-root | Raise process/file limits | OS config |
| `ams_updatefirewall ...` | `root` | Show/apply firewall rules | Connectivity |
| `ams_user_mgr ...` | `amssys` | Bulk-manage users | Security |
| `convert_to_shorter_line.pl ...` | `amssys` | Wrap long log lines | Read/convert |
| `createUsernamePassword` | `amssys` | Create encrypted credential file | Sensitive |
| `getAgentlist.sh ...` | AMS user | Retrieve agent data | Read-only |
| `getLicenseCounter` | privileged | Read license counters | Read-only |
| `innotop` | `amssys` | InnoDB monitor | Read-only |
| `JMSExpiryConfigurator.sh ...` | privileged | Configure JMS expiry | Configuration |
| `retrieve_nes.sh ...` | AMS user | Export supervised NE list | Read-only |

The Administrator Guide appendix documents these scripts, their accounts, options, output, and logs.[cite:2]
