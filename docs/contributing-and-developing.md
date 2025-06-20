# Developer Guide

## How to Contribute

Contributions to ROMMER are welcome. To contribute, follow these steps:

1. Fork the repository.
2. Create a feature branch:  
   `git checkout -b feature/your-feature-name`
3. Commit your changes:  
   `git commit -m 'Add some amazing feature'`
4. Push to your branch:  
   `git push origin feature/your-feature-name`
5. Open a Pull Request on GitHub.

## Adding Features

To add new features, you will typically:

- Add or modify modules in the `src/` directory. Each module encapsulates a core aspect of ROMMER's functionality.
- Extend command-line arguments in `src/args.rs` if your feature requires new CLI options.
- Update the configuration parsing in `src/config.rs` if your feature introduces new configuration options.
- Add new patch logic or hooks if your feature involves custom ROM modifications.
- Write or update tests and example configurations as needed.

ROMMER is designed for modularity. New features should follow the existing structure: keep logic isolated in new or existing modules, and update the main workflow in `src/main.rs` as appropriate.

## File Overview

### Top-Level Files

- `README.md`: Project overview, usage, and contribution guidelines.
- `Cargo.toml`: Project metadata and dependencies.
- `CHANGELOG.md`: Project history and notable changes.

### Source Directory (`src/`)

- `main.rs`: Application entry point. Handles command-line parsing, orchestrates the main workflow, and dispatches subcommands. The `Init` subcommand scaffolds a new ROMMER project structure, while the default workflow processes ROM customization from configuration to final output.
- `args.rs`: Defines command-line arguments and subcommands using the `clap` library. Includes options for configuration file, ROM ZIP path, cleanup, signing, dry-run mode, and the `Init` subcommand.
- `config.rs`: Defines the `Config` struct, which holds all configuration options parsed from `ROMMER.yaml`. Includes device, ROM source, retries, version, patches, signing, output, checksum, cleanup, and hooks.
- `download.rs`: Handles downloading ROM ZIPs from supported sources (LineageOS, PixelOS, EvolutionX, or custom URLs), with retry and checksum verification logic.
- `unzip.rs`: Extracts ROM ZIP files to a working directory. Supports dry-run mode for simulation.
- `rezip.rs`: Repackages the modified ROM directory into a flashable ZIP file. Supports dry-run mode.
- `finalize.rs`: Finalizes the ROM build process, including zipping, signing, and cleanup of temporary files.

### Configuration and Patches

- `ROMMER.yaml`: Main configuration file. Defines device, ROM source, version, patches, output, and other options.
- Patch directories: Mirror the ROM filesystem. Add or replace files by placing them in the patch folder. Delete files or directories by listing them in `.rommerfdel` or `.rommerdel` files, respectively.

### Example: Adding a New Patch

1. Create a new directory under your project (e.g., `my_patch/`).
2. Add files or directories to mirror the ROM structure.
3. To delete files or directories, list their paths in `.rommerfdel` or `.rommerdel` inside the patch directory.
4. Add the patch directory to the `patches` list in `ROMMER.yaml`.

### Example: Extending the CLI

To add a new command-line option:

1. Edit `src/args.rs` to add a new field to the `Args` struct.
2. Update `src/main.rs` to handle the new argument in the workflow.

### Example: Adding a New ROM Source

1. Update the logic in `src/download.rs` to support the new ROM source.
2. Add any necessary configuration options to `src/config.rs` and document them in `README.md`.

## Testing and Validation

- Use the dry-run mode (`-d` or `--dry-run`) to simulate operations without making changes.
- Add or update tests and example configurations as needed.
