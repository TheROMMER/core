use std::path::{Path, PathBuf};
use std::{fs, io};
use walkdir::WalkDir;
use anyhow::Context;

pub fn handle_deletions(
    patch_path: &Path,
    tmp_dir: &Path,
    filename: &str,
    item_type: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    let del_path = patch_path.join(filename);
    if del_path.exists() {
        let items_to_delete = read_paths(&del_path)?;
        for item in items_to_delete {
            let full_path = tmp_dir.join(&item);
            if full_path.exists() && full_path.is_dir() {
                if dry_run {
                    print_info(&format!(
                        "🔍 DRY RUN: Would delete {}: {}",
                        item_type,
                        item.display()
                    ));
                } else {
                    fs::remove_dir_all(&full_path).with_context(|| {
                        format!("Failed to delete {} '{}'", item_type, full_path.display())
                    })?;
                    print_info(&format!("🗑️  Deleted {}: {}", item_type, item.display()));
                }
            }
        }
    }
    Ok(())
}

pub fn handle_file_deletions(
    patch_path: &Path,
    tmp_dir: &Path,
    filename: &str,
    item_type: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    let del_path = patch_path.join(filename);
    if del_path.exists() {
        let items_to_delete = read_paths(&del_path)?;
        for item in items_to_delete {
            let full_path = tmp_dir.join(&item);
            if full_path.exists() && full_path.is_file() {
                if dry_run {
                    print_info(&format!(
                        "🔍 DRY RUN: Would delete {}: {}",
                        item_type,
                        item.display()
                    ));
                } else {
                    fs::remove_file(&full_path).with_context(|| {
                        format!("Failed to delete {} '{}'", item_type, full_path.display())
                    })?;
                    print_info(&format!("🗑️  Deleted {}: {}", item_type, item.display()));
                }
            }
        }
    }
    Ok(())
}

fn read_paths(file_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let content = fs::read_to_string(file_path)?;
    let mut paths = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            paths.push(PathBuf::from(trimmed));
        }
    }
    Ok(paths)
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, dry_run: bool) -> io::Result<()> {
    if dry_run {
        let mut file_count = 0;
        let mut dir_count = 0;
        for entry in WalkDir::new(&src) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    file_count += 1;
                } else if entry.file_type().is_dir() {
                    dir_count += 1;
                }
            }
        }
        println!(
            "🔍 DRY RUN: Would copy {} files and {} directories from {} to {}",
            file_count,
            dir_count,
            src.as_ref().display(),
            dst.as_ref().display()
        );
        return Ok(());
    }

    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()), dry_run)?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn print_banner() {
    print_section("🔧 ROMMER");
}

pub fn print_section(title: &str) {
    println!("\n{}", "─".repeat(22));
    println!(" {}", title);
    println!("{}", "─".repeat(22));
}

pub fn print_success(msg: &str) {
    println!("✅ {}", msg);
}

pub fn print_info(msg: &str) {
    println!("ℹ️  {}", msg);
}

pub fn print_warning(msg: &str) {
    println!("⚠️  {}", msg);
}