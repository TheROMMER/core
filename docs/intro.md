# Introduction

ROMMER is a command-line tool for customizing Android ROM ZIP files without building from source. Built in Rust for performance and reliability, ROMMER automates the process of downloading, unpacking, modifying, and repacking Android ROMs. It bridges the gap between resource-intensive source builds and error-prone manual ZIP modifications by providing a streamlined, declarative workflow defined in a simple YAML configuration file.

## Key Features

- Declarative configuration via YAML
- Automatic ROM downloading from official sources (LineageOS, PixelOS, EvolutionX, or custom URLs)
- Modular patch system using directory structures that map to the ROM filesystem
- File management: add, replace, or delete files and directories
- Multiple signing methods (apksigner, jarsigner, custom, test)
- Checksum verification for ROM integrity
- Progress tracking and robust error handling
- Hooks for custom scripts at various stages of the process
- Dry-run and skip-signing modes for advanced workflows

## How ROMMER Works

1. **Configuration**: Create a `ROMMER.yaml` file specifying device, ROM source, patches, and options.
2. **Download**: ROMMER fetches the specified ROM ZIP from official servers or mirrors.
3. **Extraction**: The ROM ZIP is unpacked into a temporary working directory.
4. **Patching**: Patch folders are applied sequentially. Files are copied into the ROM, and deletions are handled via `.rommerdel` and `.rommerfdel` files.
5. **Repacking**: The modified files are compressed into a new ZIP file.
6. **Signing**: If enabled, ROMMER signs the ZIP with provided keys.
7. **Output**: The final flashable ZIP is created with the configured filename.

## Installation

Install from [crates.io](https://crates.io/crates/rommer):

```sh
cargo install rommer
```

Or build from source:

```sh
git clone https://github.com/TheROMMER/core.git
cd core
cargo install --path .
```

## Configuration

ROMMER uses a YAML file (`ROMMER.yaml` by default) to define your customization. Example:

```yaml
device: bluejay           # Device codename (e.g., Pixel 6a)
rom: lineageos            # ROM name or direct download URL
max_retries: 3            # Download retry attempts
version: 20.0             # ROM version
android_version: 15       # Android version
timestamp: 20250614       # Required for some ROMs
variant: nightly          # Required for some ROMs

expected_checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

patches:
  - patches/remove_bloatware_patch
  - patches/custom_bootanimation_patch

output:
  filename: lineageos-pixel6a-custom.zip

signing:
  method: apksigner
  keystore_path: ~/.android/debug.keystore
  key_alias: androiddebugkey
  keystore_password: android
  key_password: android
  custom_command: null

cleanup: true

hooks:
  pre-run: scripts/pre_run.sh
  pre-download: scripts/pre_download.sh
```

## Usage

Initialize a new ROMMER project with an example structure:

```sh
rommer init my-custom-rom
```

Edit the generated `ROMMER.yaml` and add your patches to the created directory. To build your custom ROM, run:

```sh
rommer
```

You can override config options with command-line arguments, such as `--config`, `--romzip`, `--skip-signing`, `--dry-run`, and `--no-cleanup`.

## Supported ROM Sources

ROMMER can download ROMs from LineageOS, PixelOS, EvolutionX, or any custom URL.

## Patch System

Each patch is a directory that mirrors the ROM filesystem. To delete directories or files, list them in `.rommerdel` or `.rommerfdel` inside the patch folder. For example, to remove bloatware or replace the boot animation, add the relevant files or deletion lists to your patch directories.

## Hooks

ROMMER supports hooks for custom scripts at various stages, such as `pre-run`, `post-run`, `pre-unzip`, `post-unzip`, `pre-zip`, `post-zip`, `pre-sign`, `post-sign`, `pre-download`, `post-download`, `pre-cleanup`, and `post-cleanup`.

## Error Handling

ROMMER provides detailed error messages and handles common issues such as download failures (with retry), checksum verification failures, file access permission issues, and signing errors.

---

# FAQ

**What is ROMMER?**  
ROMMER is a tool for customizing Android ROM ZIP files declaratively, without building from source or manually editing ZIPs.

**Which ROMs are supported?**  
ROMMER supports LineageOS, PixelOS, EvolutionX, and any ROM available via direct download URL.

**How do I define what gets changed in the ROM?**  
You specify changes in patch directories and reference them in the `patches` list in your `ROMMER.yaml`. Add, replace, or delete files and directories as needed.

**How do I delete files or directories from the ROM?**  
List directories to delete in `.rommerdel` and files to delete in `.rommerfdel` inside your patch folder.

**How do I sign the final ROM ZIP?**  
ROMMER supports signing via apksigner, jarsigner, custom commands, or a test signature. Configure signing in the `signing` section of your YAML file.

**Can I run ROMMER without making changes to the ROM (dry-run)?**  
Yes. Use the `-d` or `--dry-run` option to simulate the process without modifying files.

**How do I skip signing?**  
Use the `-s` or `--skip-signing` option to skip the signing step.

**What if a download fails or the checksum does not match?**  
ROMMER retries downloads up to the configured `max_retries` and verifies checksums if provided. Errors are reported with detailed messages.

**Can I automate actions before or after certain steps?**  
Yes. Use hooks in your YAML config to run custom scripts at various stages of the process.

**How do I get started quickly?**  
Run `rommer init my-rom` to generate a sample project structure, then edit `ROMMER.yaml` and add your patches.
