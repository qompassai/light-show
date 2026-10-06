# Installation and Software Management

## Activation

```bash
/opt/ams/software/<release>/bin/ams_activate.sh
```

Run as `root`; activation makes the installed release operationally selectable and prepares the plug-in environment.[cite:2][cite:3]

## Component management

```bash
ams_install.sh
ams_install.sh --installActivate <directory-or-bin-files>
ams_install.sh --install <directory-or-bin-files>
ams_install.sh --activate <directory-or-bin-files>
ams_install.sh --deactivate
```

Without a component subset, `--deactivate` deactivates all active plug-ins, patches, and emergency fixes while leaving core AMS active. Back up and stop AMS first.[cite:3]

## Lifecycle wrappers

```bash
ams_server stop
ams_install.sh --installActivate /staging/ams-components
ams_server start
```

Cluster:

```bash
ams_cluster stop
ams_install.sh --installActivate /staging/ams-components
ams_cluster start
```

Install the same component set on every applicable cluster server.[cite:3]

## Golden configuration

```bash
ams_server version
ams_server version save
ams_server version save --label <Label> /path/GoldenEMSSwConfig
ams_server version verify /path/GoldenEMSSwConfig
ams_cluster status sw
```

Labels are alphanumeric and limited to 25 characters. Verification reports missing, unexpected, wrong-version, and aligned components.[cite:2]

## Licenses

```bash
ams_install_license
getLicenseCounter
```

On different hardware, restore with `-n`, then install licenses generated for the current host ID.[cite:2][cite:4]

## Patch workflow

1. Back up AMS.
2. Stop simplex or cluster.
3. Install/activate or deactivate the selected component.
4. Start AMS.
5. Reinstall the client if client code changed.
6. Verify version and Golden configuration.[cite:3]
