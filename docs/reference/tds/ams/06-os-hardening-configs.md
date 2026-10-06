# RHEL Hardening Commands and Configs

Apply these extracted settings only after checking local policy and AMS support requirements.[cite:4]

## Accounts and shell

`/etc/profile`:

```bash
TMOUT=1800
UMASK=027
```

Align `UMASK` in `/etc/csh.cshrc`; do not add `.` or empty elements to system `PATH`.[cite:4]

## Kernel configuration

Append to `/etc/sysctl.conf`:

```ini
kernel.core_uses_pid = 1
kernel.sysrq = 0
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0
net.ipv6.conf.all.accept_ra = 0
net.ipv6.conf.default.accept_ra = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.secure_redirects = 0
net.ipv4.conf.default.secure_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
net.ipv6.conf.default.accept_source_route = 0
net.ipv4.tcp_max_syn_backlog = 4096
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_synack_retries = 2
```

Apply after review:

```bash
sysctl -p
```

## Host equivalence

```bash
rm -f ~/.rhosts ~/.netrc
touch /etc/hosts.equiv
chmod 400 /etc/hosts.equiv
```

## Chrony server

```bash
yum install chrony
systemctl enable --now chronyd
vi /etc/chrony.conf
```

```conf
manual
local stratum 8
driftfile /var/lib/chrony/drift
allow 192.168.0.0/16
```

Client:

```conf
server <site-ntp-host> iburst
driftfile /var/lib/chrony/drift
logdir /var/log/chrony
log measurements statistics tracking
```

## tmpfs cap

```bash
mount -t tmpfs -o size=12G tmpfs /dev/shm
```

`/etc/fstab`:

```fstab
tmpfs /dev/shm tmpfs nodev,nosuid,size=12G 0 0
```

Size the cap so AMS retains enough RAM without swapping.[cite:4]

## Required services

```bash
systemctl enable --now chronyd
systemctl enable --now crond
systemctl enable --now sshd
```

Network services remain role-dependent; FTP is needed only for specified NE families/features.[cite:4]
