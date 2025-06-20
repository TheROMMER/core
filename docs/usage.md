# Usage Guide

## Installation

Install ROMMER using Cargo from [crates.io](https://crates.io/crates/rommer):

```bash
cargo install rommer
```

Or install from source:

```bash
git clone https://github.com/TheROMMER/core.git
cd core
cargo install --path .
```

---

## Configuration

ROMMER uses a YAML configuration file (`ROMMER.yaml` by default) to define ROM customization. This file specifies the device, ROM source, patches to apply, output options, and more.

Example `ROMMER.yaml`:

```yaml
device: garnet        # Device codename
rom: lineageos        # ROM name or direct download URL
max_retries: 3        # Download retry attempts
version: 20.0         # ROM version to download
android_version: 15   # Android version
variant: nightly      # ROM variant (required for some ROMs)

# Optional SHA-256 checksum for download verification
expected_checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

# List of patch folders to apply (in order)
patches:
  - patches/remove_bloatware_patch
  - patches/custom_bootanimation_patch

# Output configuration
output:
  filename: lineageos-garnet-custom.zip

# Optional signing configuration
signing:
  method: apksigner    # apksigner, jarsigner, custom, or test
  keystore_path: ~/.android/debug.keystore
  key_alias: androiddebugkey
  keystore_password: android
  key_password: android
  custom_command: null # Only used when method is 'custom'

# Whether to remove temporary files after completion
cleanup: true
```

ROMMER can download ROMs from sources such as LineageOS, PixelOS, EvolutionX, or a custom URL. It supports multiple signing methods: `apksigner`, `jarsigner`, `custom`, and `test` signature methods.

---

## Command-Line Usage

### Main Command

Run ROMMER from the directory containing your `ROMMER.yaml`:

```bash
rommer
```

This will download (if needed), unpack, patch, repack, and sign the ROM according to your configuration.

### Command-Line Options

You can customize ROMMER's behavior using the following options:

```
Usage: rommer [OPTIONS]
```

- `-c, --config <CONFIG>`: Path to config file (default: `ROMMER.yaml`)
- `-r, --romzip <ROMZIP>`: Path to ROM ZIP file (default: `.download`)
- `-n, --no-cleanup`: Override cleanup setting from config (keeps temporary files)
- `-s, --skip-signing`: Skip signing the final ROM
- `-d, --dry-run`: Run in dry-run mode (no changes made)
- `-h, --help`: Print help information
- `-V, --version`: Print version information

#### Examples

Run with the default config file:

```bash
rommer
```

Run in dry-run mode (no changes will be made):

```bash
rommer -d
```

Specify a custom config file:

```bash
rommer -c my-config.yaml
```

Use an existing ROM ZIP instead of downloading:

```bash
rommer -r path/to/existing/rom.zip
```

Keep temporary files (do not clean up after patching):

```bash
rommer -n
```

---

### Subcommands

#### `init`

Initialize a new ROMMER project structure with example configuration and patch files:

```bash
rommer init -n my-rom
```

- `-n, --name <NAME>`: Optional name for the patch folder (defaults to `my-rom`)

This command creates a new directory with a sample `ROMMER.yaml`, an example patch folder, and supporting files. Edit the generated `ROMMER.yaml` to configure your device and ROM settings, and add your patches to the created directory. Then, run `rommer` inside your new project directory to build your custom ROM.

---

## Patching Workflow

1. Prepare your `ROMMER.yaml` configuration.
2. Place your patch folders as specified in the `patches` list.
3. Run `rommer` to build your custom ROM.
4. The tool will download the ROM (if not provided), unpack it, apply patches, repack, and sign the final ZIP.
5. The output file will be placed as specified in the `output.filename` field.

---

## Error Handling

ROMMER provides detailed error messages for common issues such as download failures, checksum mismatches, file permission errors, and signing problems. Review the output for troubleshooting guidance.

---

For more details, see the [README](README.md) and configuration examples.
