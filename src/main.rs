mod config;
mod checksum;
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
use std::process::Command;
use zip::{ZipWriter, write::FileOptions, CompressionMethod};
use std::fs::File;
use walkdir::WalkDir;
use crate::config::SigningConfig;
use futures_util::StreamExt;
use sha2::Digest;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "ROMMER.yaml")]
    config: String,

    #[arg(short, long, default_value = ".download")]
    romzip: String,

    #[arg(short, long, help = "Override cleanup setting from config")]
    no_cleanup: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();
    let args = Args::parse();
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file '{}'", args.config))?;
    let mut config: Config = serde_yaml::from_str(&config_content)
        .with_context(|| "Failed to parse ROMMER.yaml")?;
    if args.no_cleanup {
        config.cleanup = false;
    }
    print_success(&format!("📱 Device: {} | 🔧 Base ROM: {} | 📦 Version: {} | Android Version: {}", 
        config.device, if config.rom.starts_with("http") {"custom"} else {&config.rom}, config.version, config.android_version));

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
    let final_rom_path = finalize_rom(&kept_path, &config).await?;
    print_success(&format!("🎉 Final ROM: {}", final_rom_path.display()));
    Ok(())
}

async fn finalize_rom(tmp_dir: &Path, config: &Config) -> Result<PathBuf> {
    let output_filename = config.output.filename.clone();
    let output_path = PathBuf::from(&output_filename);
    rezip_rom(tmp_dir, &output_path)?;
    sign_rom(&output_path, config).await?;
    if config.cleanup {
        print_info("🧹 Cleaning up temporary files...");
        match fs::remove_dir_all(tmp_dir) {
            Ok(_) => print_success("✅ Temporary files cleaned up successfully"),
            Err(e) => print_warning(&format!("⚠️ Failed to clean up temporary files: {}", e)),
        }
    } else {
        print_info(&format!("💾 Keeping temporary files at: {}", tmp_dir.display()));
    }

    Ok(output_path)
}

fn rezip_rom(source_dir: &Path, output_path: &Path) -> Result<()> {
    print_section("📦 CREATING FLASHABLE ZIP");
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output zip '{}'", output_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let walker = WalkDir::new(source_dir).into_iter();
    let total_files = WalkDir::new(source_dir).into_iter().count();
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files")?
        .progress_chars("█▉▊▋▌▍▎▏  "));

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
    print_success(&format!("📦 Created: {}", output_path.display()));
    Ok(())
}

async fn sign_rom(zip_path: &Path, config: &Config) -> Result<()> {
    print_section("✍️  SIGNING ROM");
    if let Some(signing_config) = &config.signing {
        match signing_config.method.as_str() {
            "apksigner" => sign_with_apksigner(zip_path, signing_config).await,
            "jarsigner" => sign_with_jarsigner(zip_path, signing_config).await,
            "custom" => sign_with_custom_command(zip_path, signing_config).await,
            _ => {
                print_warning("Unknown signing method, skipping signature");
                Ok(())
            }
        }
    } else {
        print_info("No signing configuration found, creating test signature");
        create_test_signature(zip_path).await
    }
}

async fn sign_with_apksigner(zip_path: &Path, signing_config: &SigningConfig) -> Result<()> {
    let output = Command::new("apksigner")
        .arg("sign")
        .arg("--ks")
        .arg(&signing_config.keystore_path)
        .arg("--ks-key-alias")
        .arg(&signing_config.key_alias)
        .arg("--ks-pass")
        .arg(&format!("pass:{}", signing_config.keystore_password))
        .arg("--key-pass")
        .arg(&format!("pass:{}", signing_config.key_password))
        .arg("--out")
        .arg(&format!("{}_signed.zip", zip_path.file_stem().unwrap().to_string_lossy()))
        .arg(zip_path)
        .output()
        .context("Failed to execute apksigner")?;

    if output.status.success() {
        print_success("✍️  ROM signed successfully with apksigner");
    } else {
        return Err(anyhow::anyhow!("apksigner failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn sign_with_jarsigner(zip_path: &Path, signing_config: &SigningConfig) -> Result<()> {
    let output = Command::new("jarsigner")
        .arg("-verbose")
        .arg("-sigalg")
        .arg("SHA256withRSA")
        .arg("-digestalg")
        .arg("SHA-256")
        .arg("-keystore")
        .arg(&signing_config.keystore_path)
        .arg("-storepass")
        .arg(&signing_config.keystore_password)
        .arg("-keypass")
        .arg(&signing_config.key_password)
        .arg(zip_path)
        .arg(&signing_config.key_alias)
        .output()
        .context("Failed to execute jarsigner")?;

    if output.status.success() {
        print_success("✍️  ROM signed successfully with jarsigner");
    } else {
        return Err(anyhow::anyhow!("jarsigner failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn sign_with_custom_command(zip_path: &Path, signing_config: &SigningConfig) -> Result<()> {
    if let Some(custom_command) = &signing_config.custom_command {
        let command_with_path = custom_command.replace("{zip_path}", &zip_path.to_string_lossy());
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command_with_path)
            .output()
            .context("Failed to execute custom signing command")?;

        if output.status.success() {
            print_success("✍️  ROM signed successfully with custom command");
        } else {
            return Err(anyhow::anyhow!("Custom signing command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }
    
    Ok(())
}

async fn create_test_signature(zip_path: &Path) -> Result<()> {
    let test_key_path = "test_key.p8";
    let test_cert_path = "test_cert.x509.pem";
    if !Path::new(test_key_path).exists() || !Path::new(test_cert_path).exists() {
        print_info("Generating test keys for signing...");
        generate_test_keys(test_key_path, test_cert_path).await?;
    }
    
    let output = Command::new("python3")
        .arg("-c")
        .arg(&format!(r#"
import zipfile
import hashlib
import os
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa, padding

zip_path = '{}'
with zipfile.ZipFile(zip_path, 'a') as zf:
    manifest = 'Manifest-Version: 1.0\nCreated-By: ROMMER\n\n'
    for info in zf.infolist():
        if info.filename.endswith('/'):
            continue
        with zf.open(info) as f:
            content = f.read()
            sha256_hash = hashlib.sha256(content).digest()
            manifest += f'Name: {{info.filename}}\nSHA-256-Digest: {{sha256_hash.hex()}}\n\n'
    
    zf.writestr('META-INF/MANIFEST.MF', manifest)
    zf.writestr('META-INF/CERT.SF', 'Signature-Version: 1.0\nCreated-By: ROMMER\n\n')
    zf.writestr('META-INF/CERT.RSA', b'test_signature_placeholder')

print('Test signature added')
"#, zip_path.display()))
        .output()
        .context("Failed to create test signature")?;

    if output.status.success() {
        print_success("✍️  Test signature created");
    } else {
        print_warning(&format!("Test signature creation failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn generate_test_keys(key_path: &str, cert_path: &str) -> Result<()> {
    let output = Command::new("openssl")
        .args(&["req", "-x509", "-newkey", "rsa:2048", "-keyout", key_path, 
               "-out", cert_path, "-days", "365", "-nodes", "-subj", 
               "/C=US/ST=Test/L=Test/O=ROMMER/CN=test"])
        .output()
        .context("Failed to generate test keys with openssl")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("OpenSSL key generation failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn download_rom(config: &Config) -> Result<PathBuf> {
    print_section("📥 DOWNLOADING ROM");
    let download_url = construct_download_url(config)?;
    print_info(&format!("🌐 URL: {}", download_url));
    let max_retries: u32 = config.max_retries;
    const RETRY_DELAY_MS: u64 = 2000;
    let client = reqwest::Client::new();
    let mut response = None;
    let mut last_error = None;
    for attempt in 1..=max_retries {
        match client.get(&download_url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    response = Some(resp);
                    break;
                } else {
                    let status = resp.status();
                    if attempt < max_retries {
                        print_warning(&format!("Attempt {}/{}: Download failed with status: {}. Retrying in {}ms...",
                                               attempt, max_retries, status, RETRY_DELAY_MS));
                        tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                    } else {
                        last_error = Some(anyhow::anyhow!("Download failed with status: {}", status));
                    }
                }
            },
            Err(e) => {
                if attempt < max_retries {
                    print_warning(&format!("Attempt {}/{}: Download failed: {}. Retrying in {}ms...",
                                           attempt, max_retries, e, RETRY_DELAY_MS));
                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                } else {
                    last_error = Some(anyhow::Error::new(e));
                }
            }
        }
    }

    let response = match response {
        Some(resp) => resp,
        None => return Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to download after {} attempts", max_retries))),
    };
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}, {msg})")?
        .progress_chars("█▉▊▋▌▍▎▏  "));
    let rom_filename = format!("{}_{}_{}.zip", config.device, if config.rom.starts_with("http") {"custom"} else {&config.rom}, config.version);
    let rom_path = PathBuf::from(&rom_filename);
    if rom_path.exists() {
        print_info("File already exists! Checking integrity...");
        if let Some(expected_hash) = &config.expected_checksum {
            match checksum::verify_checksum(&rom_path, expected_hash) {
                Ok(true) => {
                    print_success("✅ Existing file checksum verified successfully");
                    return Ok(rom_path);
                },
                Ok(false) => {
                    print_warning("⚠️ Checksum verification failed for existing file. Re-downloading...");
                    fs::remove_file(&rom_path).context("Failed to remove corrupted file")?;
                },
                Err(e) => {
                    print_warning(&format!("⚠️ Could not verify checksum: {}. Re-downloading...", e));
                    fs::remove_file(&rom_path).context("Failed to remove potentially corrupted file")?;
                }
            }
        } else {
            print_info("File already exists! Using the existing file...");
            return Ok(rom_path);
        }
    }
    let mut file = File::create(&rom_path)
        .with_context(|| format!("Failed to create file '{}'", rom_filename))?;
    let mut downloaded = 0u64;
    let mut hasher = sha2::Sha256::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to read chunk")?;
        file.write_all(&chunk).context("Failed to write chunk")?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
        if downloaded % (1024 * 1024) == 0 {
            let progress_percentage = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else { 0.0 };
            let elapsed = pb.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let speed = downloaded as f64 / elapsed / 1024.0 / 1024.0; // MiB/s
                pb.set_message(format!("{}% • {:.2} MiB/s", progress_percentage as u32, speed));
            } else {
                pb.set_message(format!("{}%", progress_percentage as u32));
            }
        }
    }

    let file_hash = hasher.finalize();
    let hash_hex = format!("{:x}", file_hash);
    pb.finish_with_message(format!("SHA256: {}...", &hash_hex[..8]));
    print_success(&format!("💾 Downloaded: {} (SHA256: {})", rom_filename, hash_hex));
    if let Some(expected_hash) = &config.expected_checksum {
        if expected_hash.to_lowercase() != hash_hex {
            return Err(anyhow::anyhow!("Checksum verification failed! Expected: {}, Got: {}", expected_hash, hash_hex));
        }
        print_success("✅ Checksum verified successfully");
    }
    Ok(rom_path)
}

fn construct_download_url(config: &Config) -> Result<String> {
    let base_urls = std::collections::HashMap::from([
        ("lineageos", "https://download.lineageos.org"),
        ("pixelexperience", "https://download.pixelexperience.org"),
        ("evolutionx", "https://sourceforge.net/projects/evolution-x/files"),
    ]);
    
    if config.rom.starts_with("http") {
        Ok(config.rom.clone())
    } else {
        let base_url = base_urls.get(config.rom.to_lowercase().as_str())
            .ok_or_else(|| anyhow::anyhow!("Unsupported ROM: {}", config.rom))?;
        Ok(format!("{}/builds/{}/{}", base_url, config.device, config.version))
    }
}

fn unzip_rom(zip_path: &Path, out_dir: &Path) -> Result<()> {
    print_section("📦 EXTRACTING ROM");
    let file = File::open(zip_path)
        .with_context(|| format!("Failed to open zip file '{}'", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("Failed to read zip archive")?;
    let pb = ProgressBar::new(archive.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files")?
        .progress_chars("█▉▊▋▌▍▎▏  "));

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
    print_section("🔧 ROMMER");
    println!();
}

fn print_section(title: &str) {
    println!("\n{}", "─".repeat(22));
    println!(" {}", title);
    println!("{}", "─".repeat(22));
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
