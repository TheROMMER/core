use crate::config::Config;
use crate::utils;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn finalize_rom(
    tmp_dir: &Path,
    config: &Config,
    dry_run: bool,
) -> anyhow::Result<PathBuf> {
    let output_filename = config.output.filename.clone();
    let output_path = PathBuf::from(&output_filename);
    let _ = utils::run_hook(&config.hooks, "pre-zip");
    crate::rezip::rezip_rom(tmp_dir, &output_path, dry_run)?;
    let _ = utils::run_hook(&config.hooks, "post-zip");
    let _ = utils::run_hook(&config.hooks, "pre-sign");
    crate::sign::sign_rom(&output_path, config, dry_run).await?;
    let _ = utils::run_hook(&config.hooks, "post-sign");
    if config.cleanup {
        let _ = utils::run_hook(&config.hooks, "pre-cleanup");
    }
    if config.cleanup {
        if dry_run {
            utils::print_info("🔍 DRY RUN: Would clean up temporary files...");
        } else {
            utils::print_info("🧹 Cleaning up temporary files...");
            match fs::remove_dir_all(tmp_dir) {
                Ok(_) => utils::print_success("✅ Temporary files cleaned up successfully"),
                Err(e) => {
                    utils::print_warning(&format!("⚠️ Failed to clean up temporary files: {}", e))
                }
            }
        }
        let _ = utils::run_hook(&config.hooks, "post-cleanup");
    } else {
        utils::print_info(&format!(
            "💾 Keeping temporary files at: {}",
            tmp_dir.display()
        ));
    }

    Ok(output_path)
}
