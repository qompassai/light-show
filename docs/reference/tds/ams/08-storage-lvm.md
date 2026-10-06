# Storage and LVM

## Rules

- Use a dedicated logical volume for the AMS database.
- Leave free VG extents at least as large as the database LV.
- Keep database mount points identical on both data servers.
- LVM is required for cluster installations on RHEL.
- Do not store unrelated data on the database volume.[cite:3][cite:4]

## New disk workflow

```bash
fdisk -l /dev/<disk>
fdisk /dev/<disk>
pvcreate /dev/<partition>
vgcreate <vgname> /dev/<partition>
lvcreate -L <size> -n <lvname> <vgname>
mke2fs -t ext4 /dev/<vgname>/<lvname>
mkdir -p <mountdir>
mount /dev/<vgname>/<lvname> <mountdir>
blkid /dev/<vgname>/<lvname>
```

Verify device identity before writing a partition table.[cite:4]

## Persistent mount

`/etc/fstab`:

```fstab
UUID=<logical-volume-UUID> <mountdir> <type> errors=remount-ro 0 1
```

Validate:

```bash
mount -a
findmnt <mountdir>
df -hT <mountdir>
vgs
lvs
pvs
```

A documented 52 GB VG example allocates 26 GB to DB and leaves 26 GB free for snapshots.[cite:4]

## Warning

Do not extend the database LV until the VG is full. Backup/replication needs room for a temporary snapshot LV that may grow to DB-volume size.[cite:3][cite:4]
