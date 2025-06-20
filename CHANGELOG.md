# v0.1.0 "Android"
- Initial release.

# v1.0.0 "Brownie"
- Added Dry-Run Mode. (`-d` or `--dry-run`)
- Added skip signing. (`-s` or `--skip-signing`)
- chore: Modularize the code
- Added `init` subcommand to automatically generate an example structure for your ROM.
- Added Hooks! `pre-run`, `post-run`, `pre-unzip`, `post-unzip`, `pre-zip`, `post-zip`, `pre-sign`, `post-sign`, `pre-download`, `post-download`, `pre-cleanup`, `post-cleanup`
- Added `timestamp` and `variant` keys to the config format.
- Finally! you can now download `LineageOS`, `EvolutionX` and `PixelOS` just by typing the name instead of URL!

# WIP
- Added per-patch optional config file (patch.yaml) which specifies patch name, version, author, required android version (>=, <=, >, <, etc.) and conflicts with which patches.
