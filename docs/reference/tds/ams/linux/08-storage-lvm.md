# Storage and LVM — Linux

This edition is for an operator already signed in to an AMS Linux server. Most commands run as `amssys`; use `root` or `sudo` only where stated.

> **Safety:** Replace every `<placeholder>`. Confirm the host, site, role, and account. Read the risk note, take a tested backup before changes, and use an approved maintenance window for service-affecting work. Nokia documentation and site procedures remain authoritative.

> Select an arrow to expand a task. Read **In plain English**, **Risk**, and **Before you run it** before copying a command.

<details>
<summary><strong>Know the storage rules</strong></summary>

**In plain English:** Give the database a dedicated logical volume and reserve snapshot space.

**Risk:** Read-only or low impact.

- Dedicated LV for the AMS DB.
- Free VG extents at least as large as the DB LV.
- Identical DB mount paths on both data servers.
- RHEL clusters require LVM.
- No unrelated data on the DB volume.

</details>

<details>
<summary><strong>Create storage on a new disk</strong></summary>

**In plain English:** Build a partition, PV, VG, LV, filesystem, and mount from a verified empty disk.

**Risk:** Destructive if the wrong disk is selected.

**Before you run it:** Match serial/WWN to the approved disk; do not trust only `/dev/sdX`

```bash
sudo fdisk -l /dev/<disk>
sudo fdisk /dev/<disk>
sudo pvcreate /dev/<partition>
sudo vgcreate <vgname> /dev/<partition>
sudo lvcreate -L <size> -n <lvname> <vgname>
sudo mke2fs -t ext4 /dev/<vgname>/<lvname>
sudo mkdir -p <mountdir>
sudo mount /dev/<vgname>/<lvname> <mountdir>
```

</details>

<details>
<summary><strong>Make the mount persistent</strong></summary>

**In plain English:** Use the filesystem UUID so the correct volume mounts at boot.

**Risk:** A bad fstab entry can interrupt boot.

**Before you run it:** Back up `/etc/fstab` and keep console access

```bash
sudo blkid /dev/<vgname>/<lvname>
# Add to /etc/fstab:
UUID=<logical-volume-uuid> <mountdir> ext4 errors=remount-ro 0 1
# Validate:
sudo mount -a
findmnt <mountdir>
df -hT <mountdir>
vgs
lvs
pvs
```

</details>

<details>
<summary><strong>Preserve snapshot space</strong></summary>

**In plain English:** Check that backup/replication has room for a temporary snapshot.

**Risk:** Consuming free extents can break backup or replication.

```bash
vgs
lvs
pvs
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
