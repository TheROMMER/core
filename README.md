<div align="center">
  <img height="150" src="https://github.com/TheROMMER.png">
  <h1>ROMMER</h1>
  <p><em><strong>#KeepROMMING</strong></em></p>
</div>

ROMMER is a powerful tool designed to customize Android ROM ZIP files **without building from source**. Built with Rust for maximum performance and reliability, ROMMER automates the tedious process of downloading, unpacking, modifying, and repacking Android ROMs.

## Purpose

Customizing Android ROMs typically requires either building from source (which is resource-intensive) or manually modifying ZIP files (which is error-prone). ROMMER bridges this gap by providing a streamlined, declarative approach to ROM customization.

## Key Features

- **Declarative Configuration**: Define your desired modifications in a simple YAML file
- **Automatic ROM Downloading**: Fetch official/latest ROM builds by device and ROM name
- **Modular Patch System**: Apply changes using directory structures that map directly to the ROM filesystem
- **File Management**: Add, replace, or delete files and directories with precision
- **Signing Support**: Configure ROM signing with user-provided keys
- **Checksum Verification**: Verify downloaded ROM integrity with SHA-256
- **Progress Tracking**: Real-time progress indicators for long-running operations
- **Error Handling**: Robust error recovery and detailed logging

## How It Works

1. **Configuration**: User creates a `ROMMER.yaml` file specifying device, ROM source, patches, and options
2. **Download**: ROMMER fetches the specified ROM ZIP from official servers or mirrors
3. **Extraction**: The ROM ZIP is unpacked into a temporary working directory
4. **Patching**: Each patch folder is applied sequentially:
   - Files are copied from patch folders to the unpacked ROM (overwriting existing files)
   - Directories/files listed in `.rommerdel`/`.rommerfdel` are deleted
5. **Repacking**: Modified files are compressed into a new ZIP file
6. **Signing**: If enabled, ROMMER signs the ZIP with provided keys
7. **Output**: Final flashable ZIP is created with the configured filename

## Installation

From [crates.io](https://crates.io/crates/rommer):
```bash
cargo install rommer
```

From source:
```bash
git clone https://github.com/TheROMMER/core.git
cd rommer
cargo install --path .
```

## Quick Start

```bash
# Run with default config file (ROMMER.yaml in current directory)
rommer

# dry-run mode
rommer -d # or --dry-run

# Specify a custom config file
rommer -c my-config.yaml

# dry-run mode with a custom config
rommer -d -c my-config.yaml

# Use an existing ROM ZIP instead of downloading
# Also supports using tilde (~) which is the home directory
rommer -r path/to/existing/rom.zip

# dry-run mode with an existing ROM ZIP
rommer -d -r path/to/existing/rom.zip

# Keep temporary files (override cleanup setting)
rommer -n

# Initialize a sample ROM
rommer init -n my-rom # or --name, also name optional, defaults to my-rom
```

## Configuration File

ROMMER uses a YAML configuration file (`ROMMER.yaml` by default) to define how your ROM should be customized:

```yaml
device: garnet        # Device codename
rom: lineageos        # ROM name or direct download URL
max_retries: 3        # Download retry attempts
version: 20.0         # ROM version to download
android_version: 15   # Android version
variant: nightly      # ROM Variant, required for downloading LineageOS.

# Optional SHA-256 checksum for download verification
expected_checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

# List of patch folders to apply (in order)
patches:
  - patches/remove_bloatware_patch
  - patches/custom_bootanimation_patch

# Output configuration
output:
  filename: lineageos-pixel8pro-custom.zip

# Optional signing configuration
signing:
  method: apksigner    # apksigner, jarsigner, or custom
  keystore_path: ~/.android/debug.keystore
  key_alias: androiddebugkey
  keystore_password: android
  key_password: android
  custom_command: null # Only used when method is 'custom'

# Whether to remove temporary files after completion
cleanup: true
```

## Supported ROM Sources

ROMMER can download ROMs from the following sources:

- LineageOS (`rom: lineageos`)
- PixelOS (`rom: pixelos`)
- EvolutionX (`rom: evolutionx`)
- Custom URL (`rom: https://example.com/path/to/rom.zip`)

## Patch Structure

Patches are directories that mirror the structure of the ROM's filesystem. Each patch can:

1. **Add or replace files**: Files in the patch folder are copied to the corresponding location in the ROM
2. **Delete directories**: Listed in `.rommerdel` file within the patch folder
3. **Delete files**: Listed in `.rommerfdel` file within the patch folder

### Example Patch Structure

```
patches/remove_bloatware_patch/
├── .rommerdel           # Contains paths of directories to delete
│   └── Contents: "system/app/Bloatware"
├── .rommerfdel          # Contains paths of files to delete
│   └── Contents: "system/app/UnwantedApp.apk"
└── system/
    └── etc/
        └── custom_file  # File to add/replace in the ROM
```

### Example `.rommerdel` file

```
system/app/Facebook
system/priv-app/GooglePlay
system/product/app/YouTube
```

### Example `.rommerfdel` file

```
system/build.prop
system/etc/permissions/unwanted_permission.xml
system/media/bootanimation.zip
```

## Command Line Options

```
Usage: rommer [OPTIONS]

Options:
  -c, --config <CONFIG>    Path to config file [default: ROMMER.yaml]
  -r, --romzip <ROMZIP>    Path to ROM ZIP file [default: .download]
  -n, --no-cleanup         Override cleanup setting from config
  -s, --skip-signing       Skip signing the final ROM
  -d, --dry-run            Running in dry-run mode
  -h, --help               Print help information
  -V, --version            Print version information
```

## ROM Signing Methods

ROMMER supports multiple signing methods:

1. **apksigner**: Uses Android SDK's apksigner tool
2. **jarsigner**: Uses Java's jarsigner tool
3. **custom**: Executes a custom command specified in the config
4. **test**: Creates a test signature (default when no signing config is provided)

## Error Handling

ROMMER provides detailed error messages and handles common issues such as:

- Download failures (with configurable retry mechanism)
- Checksum verification failures
- File access permission issues
- Signing errors

## Building from Source

```bash
git clone https://github.com/TheROMMER/core.git
cd rommer
cargo build --release
```

## Dependencies

ROMMER relies on the following Rust crates:

- `reqwest`: HTTP client for downloading ROMs
- `serde` & `serde_yaml`: For parsing YAML configuration
- `sha2`: For checksum verification
- `anyhow`: Error handling
- `walkdir`: Directory traversal
- `tokio`: Async runtime
- `tempfile`: Temporary file management
- `clap`: Command-line argument parsing
- `zip`: ZIP file manipulation
- `indicatif`: Progress bars

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/nameof-feature`
3. Commit your changes: `git commit -m 'Add some amazing feature'`
4. Push to the branch: `git push origin feature/nameof-feature`
5. Open a Pull Request

## License

This project is licensed under the [GNU General Public License v3 License](LICENSE).

## Acknowledgments

- The Android ROM development community
- All ROM projects for their amazing work
- The Rust community for providing excellent libraries

---

<div align="center"><p><strong>#KeepROMMING</strong></p></div>
