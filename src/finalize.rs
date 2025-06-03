use std::path::{Path, PathBuf};
use std::fs;
use crate::config::Config;

pub async fn finalize_rom(tmp_dir: &Path, config: &Config, dry_run: bool) -> anyhow::Result<PathBuf> {
    let output_filename = config.output.filename.clone();
    let output_path = PathBuf::from(&output_filename);
    crate::rezip::rezip_rom(tmp_dir, &output_path, dry_run)?;
    crate::sign::sign_rom(&output_path, config, dry_run).await?;
    if config.cleanup {
        if dry_run {
            crate::utils::print_info("🔍 DRY RUN: Would clean up temporary files...");
        } else {
            crate::utils::print_info("🧹 Cleaning up temporary files...");
            match fs::remove_dir_all(tmp_dir) {
                Ok(_) => crate::utils::print_success("✅ Temporary files cleaned up successfully"),
                Err(e) => crate::utils::print_warning(&format!("⚠️ Failed to clean up temporary files: {}", e)),
            }
        }
    } else {
        crate::utils::print_info(&format!(
            "💾 Keeping temporary files at: {}",
            tmp_dir.display()
        ));
    }

    Ok(output_path)
}