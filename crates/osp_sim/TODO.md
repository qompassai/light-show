# crates/osp_sim/ — TODO

## alarm.rs (new, in progress)
- [ ] `AlarmSeverity` enum + `from_outage()` 
- [ ] `AlarmAck` enum (New/Acknowledged/Resolved/Cleared)
- [ ] `Alarm` struct (wraps `Outage` + NOC metadata)
- [ ] Unit tests (50/50 validation/adversarial)

## Future
- [ ] Ratio splitter support (for Terra DFN levels)
- [ ] `ServiceProfile` type (for Calista provisioning)
- [ ] Protection switch component (for Amara failover scenarios)
