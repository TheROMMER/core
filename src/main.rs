mod args;
mod checksum;
mod config;
mod download;
mod finalize;
mod patchmeta;
mod rezip;
mod sign;
mod unzip;
mod utils;
use crate::args::Commands;
use anyhow::{Context, Result};
use args::Args;
use clap::Parser;
use config::Config;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tempfile::tempdir;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    utils::print_banner();
    let args = Args::parse();
    match &args.command {
        Some(Commands::Init { name }) => {
            return initsubcommand(name).await;
        }
        None => {
            nosubcommand(args).await?;
            Ok(())
        }
    }
}
async fn nosubcommand(args: Args) -> Result<()> {
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
    let _ = utils::run_hook(&config.hooks, "pre-run");
    let romzip_path = if args.romzip == ".download" {
        let _ = utils::run_hook(&config.hooks, "pre-download");
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
    let _ = utils::run_hook(&config.hooks, "pre-unzip");
    unzip::unzip_rom(&romzip_path, tmp_dir.path(), args.dry_run)?;
    let _ = utils::run_hook(&config.hooks, "post-unzip");
    utils::print_section("🔧 APPLYING PATCHES");
    let _ = utils::run_hook(&config.hooks, "pre-patch");
    for (i, patch_folder) in config.patches.iter().enumerate() {
        let patch_path = Path::new(patch_folder);
        if !patch_path.exists() {
            utils::print_warning(&format!("Patch folder '{}' does not exist!", patch_folder));
            continue;
        }

        let patch_path = Path::new(patch_folder);
        let patch_meta = patchmeta::load_patch_meta(patch_path);
        if let Some(ref tags_filter) = args.tags {
            if let Some(meta) = &patch_meta {
                let tags = meta.tags.clone().unwrap_or_default();
                let matches = tags.iter().any(|t| tags_filter.contains(t));
                if !matches {
                    utils::print_info(&format!(
                        "🚫 Skipping patch '{}' (tag mismatch)",
                        patch_folder
                    ));
                    continue;
                }
            } else {
                utils::print_info(&format!(
                    "🚫 Skipping patch '{}' (no patch.yaml, no tags)",
                    patch_folder
                ));
                continue;
            }
        }
        if let Some(meta) = &patch_meta {
            if let Some(req) = &meta.requires_android {
                if !utils::android_version_matches(req, config.android_version) {
                    utils::print_info(&format!(
                        "🚫 Skipping patch '{}' (requires Android {}, current is {})",
                        patch_folder, req, config.android_version
                    ));
                    continue;
                }
            }
        }
        if let Some(meta) = &patch_meta {
            utils::print_info(&format!(
                "[{}/{}] Applying patch: {}{} by {}",
                i + 1,
                config.patches.len(),
                meta.name
                    .clone()
                    .unwrap_or_else(|| patch_folder.to_string()),
                meta.version
                    .as_ref()
                    .map(|v| format!(" v{}", v))
                    .unwrap_or_default(),
                meta.author.clone().unwrap_or_else(|| "Unknown".to_string()),
            ));
            if let Some(desc) = &meta.description {
                utils::print_info(&format!("📝 {}", desc));
            }
        } else {
            utils::print_info(&format!(
                "[{}/{}] Applying patch '{}'",
                i + 1,
                config.patches.len(),
                patch_folder
            ));
        }
        let start = Instant::now();
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
        let duration = start.elapsed();
        utils::print_info(&format!("⏱️ Patch applied in {:.2?}", duration).to_string());
    }
    let kept_path = tmp_dir.keep();
    utils::print_section("✅ PATCHING COMPLETE");
    utils::print_success(&format!("📂 Patched ROM: {}", kept_path.display()));
    let _ = utils::run_hook(&config.hooks, "post-patch");
    let final_rom_path = finalize::finalize_rom(&kept_path, &config, args.dry_run).await?;
    utils::print_success(&format!("🎉 Final ROM: {}", final_rom_path.display()));
    Ok(())
}

async fn initsubcommand(name: &Option<String>) -> Result<()> {
    let project_name = name.as_ref().unwrap();
    utils::print_section("🚀 INITIALIZING NEW ROMMER PROJECT");
    let project_path = Path::new(project_name);
    fs::create_dir_all(project_path).context("Failed to create project directory")?;
    let config_path = project_path.join("ROMMER.yaml");
    let example_config = r#"device: your_device_codename
rom: lineageos
max_retries: 3
version: 20.0
android_version: 15
timestamp: 20250614 # required, example.
variant: nightly # required, example.
patches:
  - example_patch/

output:
  filename: custom-rom.zip

cleanup: true
"#;
    fs::write(&config_path, example_config).context("Failed to create ROMMER.yaml config file")?;
    let patches_dir = project_path;
    let example_patch_dir = patches_dir.join("example_patch");
    fs::create_dir_all(&example_patch_dir).context("Failed to create example patch directory")?;
    let example_patch_system_dir = example_patch_dir.join("system").join("etc");
    fs::create_dir_all(&example_patch_system_dir)
        .context("Failed to create example patch system directory")?;
    let example_file_path = example_patch_system_dir.join("example_custom_file.txt");
    fs::write(
        &example_file_path,
        "This is an example custom file that will be added to the ROM\n",
    )
    .context("Failed to create example custom file")?;
    let rommerdel_path = example_patch_dir.join(".rommerdel");
    fs::write(
        &rommerdel_path,
        "system/app/ExampleBloatwareApp\nsystem/priv-app/UnwantedSystemApp\n",
    )
    .context("Failed to create .rommerdel file")?;
    let rommerfdel_path = example_patch_dir.join(".rommerfdel");
    fs::write(
        &rommerfdel_path,
        "system/media/bootanimation.zip\nsystem/etc/example_unwanted_file.conf\n",
    )
    .context("Failed to create .rommerfdel file")?;
    let gitignore_path = project_path.join(".gitignore");
    fs::write(
        &gitignore_path,
        r#"# ROMMER Output Files
# Generated ROM ZIP files and build artifacts
*.zip
custom-rom*.zip
*-custom.zip
evox-*.zip
lineageos-*.zip
pixelexperience-*.zip

# Downloaded ROM Files
# Original ROM ZIPs downloaded by ROMMER
*.orig.zip
original-*.zip
base-*.zip

# Build and Cache Files
# Files created during the patching and signing process
*.sig
*.rsa
*.dsa
*.sf
MANIFEST.MF
META-INF/

# Signing Keys and Certificates
# Never commit private keys or keystores
*.keystore
*.jks
*.p12
*.pfx
*.key
*.pem
*.crt
*.cer
keys/
certs/

# OS Generated Files
# System-specific files that shouldn't be tracked
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db
desktop.ini

# Editor and IDE Files
# Various editor configurations and temporary files
.vscode/
.idea/
*.swp
*.swo
*~
.#*
\#*#

# Backup Files
# Backup files created by editors or manual backups
*.bak
*.backup
*.old
*.orig

# Downloaded Dependencies
# Large files that should be re-downloaded
downloads/
cache/
.cache/

# Environment and Config Overrides
# Local configuration files with sensitive data
*.env
.env
.env.local

# Checksum Files
# Generated checksum files
*.sha256
*.md5
checksums.txt

# Documentation Generated Files
# Auto-generated documentation
docs/build/
site/"#,
    )
    .context("Failed to create .rommerfdel file")?;
    utils::print_success(&format!(
        "✅ Project '{}' initialized successfully!",
        project_name
    ));
    utils::print_info("📝 Edit ROMMER.yaml to configure your device and ROM settings");
    utils::print_info("📂 Add your patches to the created directory");
    utils::print_info("🚀 Run 'rommer' inside your created directory to build your custom ROM");
    Ok(())
}
