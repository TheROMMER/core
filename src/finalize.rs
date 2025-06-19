use crate::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn finalize_rom(
    tmp_dir: &Path,
    config: &Config,
    dry_run: bool,
) -> anyhow::Result<PathBuf> {
    let output_filename = config.output.filename.clone();
    let output_path = PathBuf::from(&output_filename);
    crate::utils::run_hook(&config.hooks, "pre-zip");
    crate::rezip::rezip_rom(tmp_dir, &output_path, dry_run)?;
    crate::utils::run_hook(&config.hooks, "post-zip");
    crate::utils::run_hook(&config.hooks, "pre-sign");
    crate::sign::sign_rom(&output_path, config, dry_run).await?;
    crate::utils::run_hook(&config.hooks, "post-sign");
    if config.cleanup {
        crate::utils::run_hook(&config.hooks, "pre-cleanup");
    }
    if config.cleanup {
        if dry_run {
            crate::utils::print_info("🔍 DRY RUN: Would clean up temporary files...");
        } else {
            crate::utils::print_info("🧹 Cleaning up temporary files...");
            match fs::remove_dir_all(tmp_dir) {
                Ok(_) => crate::utils::print_success("✅ Temporary files cleaned up successfully"),
                Err(e) => crate::utils::print_warning(&format!(
                    "⚠️ Failed to clean up temporary files: {}",
                    e
                )),
            }
        }
        crate::utils::run_hook(&config.hooks, "post-cleanup");
    } else {
        crate::utils::print_info(&format!(
            "💾 Keeping temporary files at: {}",
            tmp_dir.display()
        ));
    }

    Ok(output_path)
}
