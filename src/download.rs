use crate::utils;
use std::path::PathBuf;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::fs::File;
use anyhow::Context;
use sha2::Digest;
use futures_util::StreamExt;
use std::io::Write;
use crate::checksum;
use crate::config::Config;

pub async fn download_rom(config: &Config, dry_run: bool) -> anyhow::Result<PathBuf> {
    crate::utils::print_section("📥 DOWNLOADING ROM");
    let download_url = construct_download_url(config)?;
    crate::utils::print_info(&format!("🌐 URL: {}", download_url));

    if dry_run {
        crate::utils::print_info("🔍 DRY RUN: Would download ROM from URL");
        let rom_filename = format!(
            "{}_{}_{}.zip",
            config.device,
            if config.rom.starts_with("http") {
                "custom"
            } else {
                &config.rom
            },
            config.version
        );
        crate::utils::print_info(&format!("🔍 DRY RUN: Would save as: {}", rom_filename));
        return Ok(PathBuf::from(rom_filename));
    }

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
                        crate::utils::print_warning(&format!(
                            "Attempt {}/{}: Download failed with status: {}. Retrying in {}ms...",
                            attempt, max_retries, status, RETRY_DELAY_MS
                        ));
                        tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS))
                            .await;
                    } else {
                        last_error =
                            Some(anyhow::anyhow!("Download failed with status: {}", status));
                    }
                }
            }
            Err(e) => {
                if attempt < max_retries {
                    crate::utils::print_warning(&format!(
                        "Attempt {}/{}: Download failed: {}. Retrying in {}ms...",
                        attempt, max_retries, e, RETRY_DELAY_MS
                    ));
                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                } else {
                    last_error = Some(anyhow::Error::new(e));
                }
            }
        }
    }

    let response = match response {
        Some(resp) => resp,
        None => {
            return Err(last_error.unwrap_or_else(|| {
                anyhow::anyhow!("Failed to download after {} attempts", max_retries)
            }));
        }
    };
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}, {msg})")?
        .progress_chars("█▉▊▋▌▍▎▏  "));
    let rom_filename = format!(
        "{}_{}_{}.zip",
        config.device,
        if config.rom.starts_with("http") {
            "custom"
        } else {
            &config.rom
        },
        config.version
    );
    let rom_path = PathBuf::from(&rom_filename);
    if rom_path.exists() {
        crate::utils::print_info("File already exists! Checking integrity...");
        if let Some(expected_hash) = &config.expected_checksum {
            match checksum::verify_checksum(&rom_path, expected_hash) {
                Ok(true) => {
                    crate::utils::print_success("✅ Existing file checksum verified successfully");
                    return Ok(rom_path);
                }
                Ok(false) => {
                    crate::utils::print_warning(
                        "⚠️ Checksum verification failed for existing file. Re-downloading...",
                    );
                    fs::remove_file(&rom_path).context("Failed to remove corrupted file")?;
                }
                Err(e) => {
                    crate::utils::print_warning(&format!(
                        "⚠️ Could not verify checksum: {}. Re-downloading...",
                        e
                    ));
                    fs::remove_file(&rom_path)
                        .context("Failed to remove potentially corrupted file")?;
                }
            }
        } else {
            crate::utils::print_info("File already exists! Using the existing file...");
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
            } else {
                0.0
            };
            let elapsed = pb.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let speed = downloaded as f64 / elapsed / 1024.0 / 1024.0;
                pb.set_message(format!(
                    "{}% • {:.2} MiB/s",
                    progress_percentage as u32, speed
                ));
            } else {
                pb.set_message(format!("{}%", progress_percentage as u32));
            }
        }
    }

    let file_hash = hasher.finalize();
    let hash_hex = format!("{:x}", file_hash);
    pb.finish_with_message(format!("SHA256: {}...", &hash_hex[..8]));
    crate::utils::print_success(&format!(
        "💾 Downloaded: {} (SHA256: {})",
        rom_filename, hash_hex
    ));
    if let Some(expected_hash) = &config.expected_checksum {
        if expected_hash.to_lowercase() != hash_hex {
            return Err(anyhow::anyhow!(
                "Checksum verification failed! Expected: {}, Got: {}",
                expected_hash,
                hash_hex
            ));
        }
        crate::utils::print_success("✅ Checksum verified successfully");
    }
    utils::run_hook(&config.hooks, "post-download");
    Ok(rom_path)
}

fn construct_download_url(config: &Config) -> anyhow::Result<String> {
    let base_urls = std::collections::HashMap::from([
        ("lineageos", "https://download.lineageos.org"),
        ("pixelexperience", "https://download.pixelexperience.org"),
        (
            "evolutionx",
            "https://sourceforge.net/projects/evolution-x/files",
        ),
    ]);

    if config.rom.starts_with("http") {
        Ok(config.rom.clone())
    } else {
        let base_url = base_urls
            .get(config.rom.to_lowercase().as_str())
            .ok_or_else(|| anyhow::anyhow!("Unsupported ROM: {}", config.rom))?;
        Ok(format!(
            "{}/builds/{}/{}",
            base_url, config.device, config.version
        ))
    }
}
