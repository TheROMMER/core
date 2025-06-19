use anyhow::Context;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn rezip_rom(source_dir: &Path, output_path: &Path, dry_run: bool) -> anyhow::Result<()> {
    crate::utils::print_section("📦 CREATING FLASHABLE ZIP");
    if dry_run {
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Would create zip file: {}",
            output_path.display()
        ));
        let _walker = WalkDir::new(source_dir).into_iter();
        let total_files = WalkDir::new(source_dir).into_iter().count();
        crate::utils::print_info(&format!("🔍 DRY RUN: Would compress {} files", total_files));
        return Ok(());
    }

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output zip '{}'", output_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let walker = WalkDir::new(source_dir).into_iter();
    let total_files = WalkDir::new(source_dir).into_iter().count();
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files",
            )?
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    let options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(source_dir)?;
        if path.is_file() {
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name.to_string_lossy(), options)?;
        }
        pb.inc(1);
    }

    zip.finish()?;
    pb.finish_with_message("Rezip complete!");
    crate::utils::print_success(&format!("📦 Created: {}", output_path.display()));
    Ok(())
}
