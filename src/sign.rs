use std::path::Path;
use std::process::Command;
use clap::Parser;
use anyhow::Context;
use crate::args::Args;
use crate::config::{Config, SigningConfig};

pub async fn sign_rom(zip_path: &Path, config: &Config, dry_run: bool) -> anyhow::Result<()> {
    let args = Args::parse();
    crate::utils::print_section("✍️  SIGNING ROM");
    if args.skip_signing {
        if let Some(signing_config) = &config.signing {
            match signing_config.method.as_str() {
                "apksigner" => sign_with_apksigner(zip_path, signing_config, dry_run).await,
                "jarsigner" => sign_with_jarsigner(zip_path, signing_config, dry_run).await,
                "custom" => sign_with_custom_command(zip_path, signing_config, dry_run).await,
                _ => {
                    crate::utils::print_warning("Unknown signing method, skipping signature");
                    Ok(())
                }
            }
        } else {
            create_test_signature(zip_path, dry_run).await
        }
    } else {
        crate::utils::print_info("Skipping signing");
        Ok(())
    }
}

async fn sign_with_apksigner(
    zip_path: &Path,
    signing_config: &SigningConfig,
    dry_run: bool,
) -> anyhow::Result<()> {
    if dry_run {
        crate::utils::print_info("🔍 DRY RUN: Would sign ROM with apksigner");
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Keystore: {}",
            signing_config.keystore_path
        ));
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Key alias: {}",
            signing_config.key_alias
        ));
        return Ok(());
    }

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
        .arg(&format!(
            "{}_signed.zip",
            zip_path.file_stem().unwrap().to_string_lossy()
        ))
        .arg(zip_path)
        .output()
        .context("Failed to execute apksigner")?;

    if output.status.success() {
        crate::utils::print_success("✍️  ROM signed successfully with apksigner");
    } else {
        return Err(anyhow::anyhow!(
            "apksigner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

async fn sign_with_jarsigner(
    zip_path: &Path,
    signing_config: &SigningConfig,
    dry_run: bool,
) -> anyhow::Result<()> {
    if dry_run {
        crate::utils::print_info("🔍 DRY RUN: Would sign ROM with jarsigner");
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Keystore: {}",
            signing_config.keystore_path
        ));
        crate::utils::print_info(&format!(
            "🔍 DRY RUN: Key alias: {}",
            signing_config.key_alias
        ));
        return Ok(());
    }

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
        crate::utils::print_success("✍️  ROM signed successfully with jarsigner");
    } else {
        return Err(anyhow::anyhow!(
            "jarsigner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

async fn sign_with_custom_command(
    zip_path: &Path,
    signing_config: &SigningConfig,
    dry_run: bool,
) -> anyhow::Result<()> {
    if let Some(custom_command) = &signing_config.custom_command {
        if dry_run {
            crate::utils::print_info("🔍 DRY RUN: Would sign ROM with custom command");
            let command_with_path =
                custom_command.replace("{zip_path}", &zip_path.to_string_lossy());
            crate::utils::print_info(&format!("🔍 DRY RUN: Command: {}", command_with_path));
            return Ok(());
        }

        let command_with_path = custom_command.replace("{zip_path}", &zip_path.to_string_lossy());
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command_with_path)
            .output()
            .context("Failed to execute custom signing command")?;

        if output.status.success() {
            crate::utils::print_success("✍️  ROM signed successfully with custom command");
        } else {
            return Err(anyhow::anyhow!(
                "Custom signing command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

async fn create_test_signature(zip_path: &Path, dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        crate::utils::print_info("🔍 DRY RUN: Would create test signature");
        crate::utils::print_info("🔍 DRY RUN: Would generate test keys if needed");
        return Ok(());
    }

    let test_key_path = "test_key.p8";
    let test_cert_path = "test_cert.x509.pem";
    if !Path::new(test_key_path).exists() || !Path::new(test_cert_path).exists() {
        crate::utils::print_info("Generating test keys for signing...");
        generate_test_keys(test_key_path, test_cert_path).await?;
    }

    let output = Command::new("python3")
        .arg("-c")
        .arg(&format!(
            r#"
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
"#,
            zip_path.display()
        ))
        .output()
        .context("Failed to create test signature")?;

    if output.status.success() {
        crate::utils::print_success("✍️  Test signature created");
    } else {
        crate::utils::print_warning(&format!(
            "Test signature creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

async fn generate_test_keys(key_path: &str, cert_path: &str) -> anyhow::Result<()> {
    let output = Command::new("openssl")
        .args(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key_path,
            "-out",
            cert_path,
            "-days",
            "365",
            "-nodes",
            "-subj",
            "/C=US/ST=Test/L=Test/O=ROMMER/CN=test",
        ])
        .output()
        .context("Failed to generate test keys with openssl")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "OpenSSL key generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}