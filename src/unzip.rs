use std::path::Path;
use std::fs::File;
use zip::ZipArchive;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use anyhow::Context;

pub fn unzip_rom(zip_path: &Path, out_dir: &Path, dry_run: bool) -> anyhow::Result<()> {
    crate::utils::print_section("📦 EXTRACTING ROM");

    if dry_run {
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Would extract files to: {}",
            out_dir.display()
        ));
        return Ok(());
    }

    let file = File::open(zip_path)
        .with_context(|| format!("Failed to open zip file '{}'", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;
    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files",
            )?
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = out_dir.join(file.mangled_name());
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
        pb.inc(1);
    }
    pb.finish_with_message("Extraction complete!");
    crate::utils::print_success(&format!("📂 Extracted to: {}", out_dir.display()));
    Ok(())
}