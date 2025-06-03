use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use std::{fs::File, io::Read, path::Path};

/// Calculates the SHA-256 checksum of a file
pub fn calculate_file_checksum(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open file for checksum calculation: {}", path.display()))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 64]; // 64KB buffer

    loop {
        let bytes_read = file.read(&mut buffer)
            .with_context(|| "Failed to read file during checksum calculation")?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verify a file's checksum against an expected value
pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool> {
    let calculated = calculate_file_checksum(path)?;
    Ok(calculated.to_lowercase() == expected.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum_calculation() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Hello, ROMMER!")?;

        // Expected SHA-256 hash for "Hello, ROMMER!"
        let expected = "9c9c041fd4c2d9be53503827fedb365c82e26bcea9d45bc3de9c0a8a8205a10d";

        let calculated = calculate_file_checksum(temp_file.path())?;
        assert_eq!(calculated, expected);

        let verified = verify_checksum(temp_file.path(), expected)?;
        assert!(verified);

        Ok(())
    }
}
