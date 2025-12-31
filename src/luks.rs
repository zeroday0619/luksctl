//! LUKS operations using cryptsetup
//!
//! This module handles all LUKS-related operations with security hardening:
//! - Secure password handling with zeroization
//! - Input validation and sanitization
//! - Safe process execution

use anyhow::{bail, Context, Result};
use rust_i18n::t;
use secrecy::{ExposeSecret, SecretString};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Maximum allowed mapper name length (Linux dm-crypt limit)
const MAX_MAPPER_NAME_LEN: usize = 128;

/// Allowed characters in mapper names (alphanumeric, dash, underscore)
const ALLOWED_MAPPER_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

/// Validate a mapper name for safety
fn validate_mapper_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("{}", t!("luks.mapper_name_empty"));
    }
    
    if name.len() > MAX_MAPPER_NAME_LEN {
        bail!("{}", t!("luks.mapper_name_too_long", max = MAX_MAPPER_NAME_LEN));
    }
    
    // Check for path traversal attempts
    if name.contains("..") || name.contains('/') || name.contains('\0') {
        bail!("{}", t!("luks.mapper_name_forbidden_chars"));
    }
    
    // Only allow safe characters
    if !name.chars().all(|c| ALLOWED_MAPPER_CHARS.contains(c)) {
        bail!("{}", t!("luks.mapper_name_forbidden_chars"));
    }
    
    Ok(())
}

/// Validate that a device path is safe to use
fn validate_device_path(device: &Path) -> Result<()> {
    // Must be an absolute path
    if !device.is_absolute() {
        bail!("{}", t!("luks.device_path_must_absolute"));
    }
    
    // Check for path traversal
    let path_str = device.to_string_lossy();
    if path_str.contains("..") {
        bail!("{}", t!("luks.device_path_invalid_components"));
    }
    
    // Must exist and be a block device or in /dev/
    if !device.exists() {
        bail!("{}", t!("luks.device_not_exist", path = device.display().to_string()));
    }
    
    // Verify it's under /dev/ hierarchy
    if !path_str.starts_with("/dev/") {
        bail!("{}", t!("luks.device_must_in_dev"));
    }
    
    // Check that it's a block device (type check)
    let metadata = std::fs::metadata(device)
        .context(t!("luks.failed_get_device_metadata").to_string())?;
    
    // On Linux, block devices have specific mode bits
    // S_IFBLK = 0o60000
    let file_type = metadata.mode() & 0o170000;
    let is_block_device = file_type == 0o60000;
    
    // Also allow symlinks to block devices (common for LVM)
    let is_symlink = device.is_symlink();
    
    if !is_block_device && !is_symlink {
        bail!("{}", t!("luks.path_not_block_device", path = device.display().to_string()));
    }
    
    Ok(())
}

/// Open a LUKS device with the given password
/// 
/// # Security
/// - Password is handled via SecretString and zeroized after use
/// - Mapper name is validated to prevent injection attacks
/// - Device path is validated to prevent path traversal
pub fn luks_open(device: &Path, mapper_name: &str, password: &SecretString) -> Result<()> {
    // Validate inputs
    validate_device_path(device)?;
    validate_mapper_name(mapper_name)?;
    
    let mut child = Command::new("cryptsetup")
        .args(["open", "--type", "luks"])
        .arg(device)
        .arg(mapper_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(t!("luks.failed_execute_cryptsetup").to_string())?;

    // Write password to stdin - exposed only momentarily
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(password.expose_secret().as_bytes())
            .context(t!("luks.failed_write_password").to_string())?;
        // stdin is dropped here, closing the pipe
    }

    let output = child.wait_with_output()
        .context(t!("luks.failed_wait_cryptsetup").to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't expose detailed error messages that might leak information
        if stderr.contains("No key available") || stderr.contains("wrong") {
            bail!("{}", t!("luks.failed_open_luks_incorrect"));
        }
        bail!("{}", t!("luks.failed_open_luks", error = stderr.trim()));
    }

    Ok(())
}

/// Close a LUKS device
/// 
/// # Security
/// - Mapper name is validated to prevent injection attacks
pub fn luks_close(mapper_name: &str) -> Result<()> {
    // Validate mapper name
    validate_mapper_name(mapper_name)?;
    
    let output = Command::new("cryptsetup")
        .args(["close", mapper_name])
        .output()
        .context(t!("luks.failed_execute_cryptsetup").to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", t!("luks.failed_close_luks", error = stderr.trim()));
    }

    Ok(())
}

/// Check if a device is a LUKS device
/// 
/// # Security
/// - Device path is validated before use
pub fn is_luks_device(device: &Path) -> Result<bool> {
    // Basic path validation (existence check is done separately)
    if !device.is_absolute() {
        bail!("{}", t!("luks.device_path_must_absolute"));
    }
    
    let path_str = device.to_string_lossy();
    if path_str.contains("..") || !path_str.starts_with("/dev/") {
        bail!("{}", t!("luks.invalid_device_path"));
    }
    
    let output = Command::new("cryptsetup")
        .args(["isLuks"])
        .arg(device)
        .output()
        .context(t!("luks.failed_execute_isluks").to_string())?;

    Ok(output.status.success())
}
