mod checksum;
mod config;
mod args;
mod finalize;
mod rezip;
mod sign;
mod download;
mod unzip;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use tokio;
use args::Args;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    utils::print_banner(args.dry_run);
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file '{}'", args.config))?;
    let mut config: Config =
        serde_yaml::from_str(&config_content).with_context(|| "Failed to parse ROMMER.yaml")?;
    if args.no_cleanup {
        config.cleanup = false;
    }
    utils::print_success(&format!(
        "📱 Device: {} | 🔧 Base ROM: {} | 📦 Version: {} | Android Version: {}",
        config.device,
        if config.rom.starts_with("http") {
            "custom"
        } else {
            &config.rom
        },
        config.version,
        config.android_version
    ));

    let romzip_path = if args.romzip == ".download" {
        download::download_rom(&config, args.dry_run).await?
    } else {
        let expanded = shellexpand::tilde(&args.romzip);
        PathBuf::from(expanded.to_string())
    };

    let tmp_dir = tempdir().context("Failed to create temp dir")?;
    utils::print_info(&format!(
        "🗂️  Working directory: {}",
        tmp_dir.path().display()
    ));
    unzip::unzip_rom(&romzip_path, tmp_dir.path(), args.dry_run)?;
    utils::print_section("🔧 APPLYING PATCHES");
    for (i, patch_folder) in config.patches.iter().enumerate() {
        let patch_path = Path::new(patch_folder);
        if !patch_path.exists() {
            utils::print_warning(&format!("Patch folder '{}' does not exist!", patch_folder));
            continue;
        }

        utils::print_info(&format!(
            "[{}/{}] Applying patch '{}'",
            i + 1,
            config.patches.len(),
            patch_folder
        ));
        utils::copy_dir_all(patch_path, tmp_dir.path(), args.dry_run)
            .with_context(|| format!("Failed to copy patch folder '{}'", patch_folder))?;
        utils::handle_deletions(
            patch_path,
            tmp_dir.path(),
            ".rommerdel",
            "directory",
            args.dry_run,
        )?;
        utils::handle_file_deletions(
            patch_path,
            tmp_dir.path(),
            ".rommerfdel",
            "file",
            args.dry_run,
        )?;
    }
    let kept_path = tmp_dir.keep();
    utils::print_section("✅ PATCHING COMPLETE");
    utils::print_success(&format!("📂 Patched ROM: {}", kept_path.display()));
    let final_rom_path = finalize::finalize_rom(&kept_path, &config, args.dry_run).await?;
    utils::print_success(&format!("🎉 Final ROM: {}", final_rom_path.display()));
    Ok(())
}

