mod config;
use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use zip::ZipArchive;
use reqwest;
use tokio;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "ROMMER.yaml")]
    config: String,

    #[arg(short, long, default_value = ".download")]
    romzip: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();
    let args = Args::parse();
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file '{}'", args.config))?;
    let config: Config = serde_yaml::from_str(&config_content)
        .with_context(|| "Failed to parse ROMMER.yaml")?;
    print_success(&format!("📱 Device: {} | 🔧 Base ROM: {} | 📦 Version: {}", 
        config.device, if config.rom.starts_with("http") {"custom"} else {&config.rom}, config.version));

    let romzip_path = if args.romzip == ".download" {
        download_rom(&config).await?
    } else {
        let expanded = shellexpand::tilde(&args.romzip);
        PathBuf::from(expanded.to_string())
    };

    let tmp_dir = tempdir().context("Failed to create temp dir")?;
    print_info(&format!("🗂️  Working directory: {}", tmp_dir.path().display()));
    unzip_rom(&romzip_path, tmp_dir.path())?;
    print_section("🔧 APPLYING PATCHES");
    for (i, patch_folder) in config.patches.iter().enumerate() {
        let patch_path = Path::new(patch_folder);
        if !patch_path.exists() {
            print_warning(&format!("Patch folder '{}' does not exist!", patch_folder));
            continue;
        }

        print_info(&format!("[{}/{}] Applying patch '{}'", 
            i + 1, config.patches.len(), patch_folder));
        copy_dir_all(patch_path, tmp_dir.path())
            .with_context(|| format!("Failed to copy patch folder '{}'", patch_folder))?;
        handle_deletions(patch_path, tmp_dir.path(), ".rommerdel", "directory")?;
        handle_file_deletions(patch_path, tmp_dir.path(), ".rommerfdel", "file")?;
    }
    
    let kept_path = tmp_dir.keep();
    print_section("✅ PATCHING COMPLETE");
    print_success(&format!("📂 Patched ROM: {}", kept_path.display()));
    //TODO: rezip and sign rom
    Ok(())
}

async fn download_rom(config: &Config) -> Result<PathBuf> {
    print_section("📥 DOWNLOADING ROM");
    let download_url = construct_download_url(config)?;
    print_info(&format!("🌐 URL: {}", download_url));
    let client = reqwest::Client::new();
    let response = client.get(&download_url).send().await
        .context("Failed to start download")?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
    }
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "));
    let rom_filename = format!("{}_{}_{}.zip", config.device, if config.rom.starts_with("http") {"custom"} else {&config.rom}, config.version);
    let rom_path = PathBuf::from(&rom_filename);
    let mut file = fs::File::create(&rom_path)
        .with_context(|| format!("Failed to create file '{}'", rom_filename))?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read chunk")?;
        file.write_all(&chunk).context("Failed to write chunk")?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message("Download complete!");
    print_success(&format!("💾 Downloaded: {}", rom_filename));
    Ok(rom_path)
}

fn construct_download_url(config: &Config) -> Result<String> {
    let base_urls = std::collections::HashMap::from([
        ("lineageos", "https://download.lineageos.org"),
        ("pixelexperience", "https://download.pixelexperience.org"),
        ("evolutionx", "https://sourceforge.net/projects/evolution-x"),
    ]);
    let mut base_url;
    if !base_urls.contains_key(config.rom.as_str()) {
        base_url = config.rom.clone();
    } else {
    base_url = base_urls.get(config.rom.to_lowercase().as_str())
        .ok_or_else(|| anyhow::anyhow!("Unsupported ROM: {}", config.rom))?.to_string();
    }
    if !config.rom.starts_with("http") {
        Ok(format!("{}/builds/{}/{}", base_url, config.device, config.version))
    } else {
        Ok(config.rom.clone())
    }
}

fn unzip_rom(zip_path: &Path, out_dir: &Path) -> Result<()> {
    print_section("📦 EXTRACTING ROM");
    let file = fs::File::open(zip_path)
        .with_context(|| format!("Failed to open zip file '{}'", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;
    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files")
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "));

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = out_dir.join(file.mangled_name());
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
        pb.inc(1);
    }
    pb.finish_with_message("Extraction complete!");
    print_success(&format!("📂 Extracted to: {}", out_dir.display()));
    Ok(())
}

fn handle_deletions(patch_path: &Path, tmp_dir: &Path, filename: &str, item_type: &str) -> Result<()> {
    let del_path = patch_path.join(filename);
    if del_path.exists() {
        let items_to_delete = read_paths(&del_path)?;
        for item in items_to_delete {
            let full_path = tmp_dir.join(&item);
            if full_path.exists() && full_path.is_dir() {
                fs::remove_dir_all(&full_path)
                    .with_context(|| format!("Failed to delete {} '{}'", item_type, full_path.display()))?;
                print_info(&format!("🗑️  Deleted {}: {}", item_type, item.display()));
            }
        }
    }
    Ok(())
}

fn handle_file_deletions(patch_path: &Path, tmp_dir: &Path, filename: &str, item_type: &str) -> Result<()> {
    let del_path = patch_path.join(filename);
    if del_path.exists() {
        let items_to_delete = read_paths(&del_path)?;
        for item in items_to_delete {
            let full_path = tmp_dir.join(&item);
            if full_path.exists() && full_path.is_file() {
                fs::remove_file(&full_path)
                    .with_context(|| format!("Failed to delete {} '{}'", item_type, full_path.display()))?;
                print_info(&format!("🗑️  Deleted {}: {}", item_type, item.display()));
            }
        }
    }
    Ok(())
}

fn read_paths(file_path: &Path) -> Result<Vec<PathBuf>> {
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

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn print_banner() {
    println!("\n{}", "=".repeat(60));
    println!("🔧 ROMMER - #KeepROMMING");
    println!("{}\n", "=".repeat(60));
}

fn print_section(title: &str) {
    println!("\n{}", "─".repeat(50));
    println!("🔹 {}", title);
    println!("{}", "─".repeat(50));
}

fn print_success(msg: &str) {
    println!("✅ {}", msg);
}

fn print_info(msg: &str) {
    println!("ℹ️  {}", msg);
}

fn print_warning(msg: &str) {
    println!("⚠️  {}", msg);
}
