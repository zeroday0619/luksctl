//! Mount/unmount operations
//!
//! This module handles filesystem mount/unmount operations with security hardening:
//! - Mount option validation and sanitization
//! - Path validation to prevent attacks
//! - Safe command execution

use anyhow::{bail, Context, Result};
use rust_i18n::t;
use std::path::Path;
use std::process::Command;

/// Allowed filesystem types (whitelist approach)
const ALLOWED_FS_TYPES: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "f2fs", "ntfs", "ntfs3",
    "vfat", "exfat", "iso9660", "udf", "hfsplus", "jfs", "reiserfs",
];

/// Forbidden mount option patterns (blacklist for dangerous options)
const FORBIDDEN_MOUNT_OPTIONS: &[&str] = &[
    "suid",     // Allow setuid - could be dangerous
    "dev",      // Allow device files - could be dangerous  
    "exec",     // Allow execution - be explicit about this
];

/// Mount options structure
#[derive(Debug, Default, Clone)]
pub struct MountOptions {
    pub read_only: bool,
    pub fs_type: Option<String>,
    pub options: Option<String>,
}

/// Validate filesystem type
fn validate_fs_type(fs_type: &str) -> Result<()> {
    // Check for null bytes or path separators
    if fs_type.contains('\0') || fs_type.contains('/') {
        bail!("{}", t!("mount.invalid_fs_type"));
    }
    
    // Check length
    if fs_type.len() > 32 {
        bail!("{}", t!("mount.fs_type_too_long"));
    }
    
    // Whitelist check
    let fs_lower = fs_type.to_lowercase();
    if !ALLOWED_FS_TYPES.contains(&fs_lower.as_str()) {
        bail!("{}", t!("mount.unsupported_fs_type", fs_type = fs_type, allowed = format!("{:?}", ALLOWED_FS_TYPES)));
    }
    
    Ok(())
}

/// Validate and sanitize mount options
fn validate_mount_options(options: &str) -> Result<String> {
    // Check for null bytes
    if options.contains('\0') {
        bail!("{}", t!("mount.mount_options_null_bytes"));
    }
    
    // Check total length
    if options.len() > 1024 {
        bail!("{}", t!("mount.mount_options_too_long"));
    }
    
    // Parse individual options and validate
    let mut validated_opts = Vec::new();
    
    for opt in options.split(',') {
        let opt = opt.trim();
        
        if opt.is_empty() {
            continue;
        }
        
        // Check for shell metacharacters and injection attempts
        if opt.contains(|c: char| {
            matches!(c, ';' | '&' | '|' | '$' | '`' | '\n' | '\r' | '\\' | '"' | '\'')
        }) {
            bail!("{}", t!("mount.mount_option_forbidden_chars", opt = opt));
        }
        
        // Extract option name (before '=' if present)
        let opt_name = opt.split('=').next().unwrap_or(opt);
        
        // Check against forbidden options
        for forbidden in FORBIDDEN_MOUNT_OPTIONS {
            if opt_name.eq_ignore_ascii_case(forbidden) {
                // Note: We warn but don't fail - user might want these
                eprintln!("{}", t!("mount.warning_dangerous_option", opt = opt_name));
            }
        }
        
        validated_opts.push(opt.to_string());
    }
    
    Ok(validated_opts.join(","))
}

/// Validate mount point path
fn validate_mount_point(mount_point: &Path) -> Result<()> {
    // Must be absolute
    if !mount_point.is_absolute() {
        bail!("{}", t!("mount.mount_point_must_absolute"));
    }
    
    let path_str = mount_point.to_string_lossy();
    
    // Check for null bytes
    if path_str.contains('\0') {
        bail!("{}", t!("mount.mount_point_null_bytes"));
    }
    
    // Check for path traversal
    if path_str.contains("..") {
        bail!("{}", t!("mount.mount_point_path_traversal"));
    }
    
    // Must exist and be a directory
    if !mount_point.exists() {
        bail!("{}", t!("mount.mount_point_not_exist", path = mount_point.display().to_string()));
    }
    
    if !mount_point.is_dir() {
        bail!("{}", t!("mount.mount_point_not_dir", path = mount_point.display().to_string()));
    }
    
    Ok(())
}

/// Validate device path for mounting
fn validate_device_for_mount(device: &Path) -> Result<()> {
    // Must be absolute
    if !device.is_absolute() {
        bail!("{}", t!("mount.device_path_must_absolute"));
    }
    
    let path_str = device.to_string_lossy();
    
    // Check for null bytes
    if path_str.contains('\0') {
        bail!("{}", t!("mount.device_path_null_bytes"));
    }
    
    // Should be under /dev/mapper for our use case
    if !path_str.starts_with("/dev/") {
        bail!("{}", t!("mount.device_must_in_dev"));
    }
    
    // Must exist
    if !device.exists() {
        bail!("{}", t!("mount.device_not_exist", path = device.display().to_string()));
    }
    
    Ok(())
}

/// Mount a device to a mount point
/// 
/// # Security
/// - Validates device path
/// - Validates mount point
/// - Validates and sanitizes mount options
/// - Uses nosuid, nodev by default for security
pub fn mount_device(device: &Path, mount_point: &Path, options: &MountOptions) -> Result<()> {
    // Validate inputs
    validate_device_for_mount(device)?;
    validate_mount_point(mount_point)?;
    
    let mut cmd = Command::new("mount");
    
    // Build secure default options
    let mut mount_opts = Vec::new();
    
    // Add security defaults
    mount_opts.push("nosuid".to_string());  // Ignore setuid bits
    mount_opts.push("nodev".to_string());   // Ignore device files
    
    // Add read-only flag
    if options.read_only {
        mount_opts.push("ro".to_string());
    }

    // Add filesystem type (validated)
    if let Some(ref fs_type) = options.fs_type {
        validate_fs_type(fs_type)?;
        cmd.arg("-t").arg(fs_type);
    }

    // Add additional mount options (validated)
    if let Some(ref opts) = options.options {
        let validated = validate_mount_options(opts)?;
        if !validated.is_empty() {
            mount_opts.push(validated);
        }
    }
    
    // Add all options
    if !mount_opts.is_empty() {
        cmd.arg("-o").arg(mount_opts.join(","));
    }

    cmd.arg(device);
    cmd.arg(mount_point);

    let output = cmd.output()
        .context(t!("mount.failed_execute_mount").to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", t!("mount.failed_mount_device", error = stderr.trim()));
    }

    Ok(())
}

/// Unmount a mount point
/// 
/// # Security
/// - Validates mount point path
pub fn unmount(mount_point: &Path) -> Result<()> {
    // Validate mount point
    if !mount_point.is_absolute() {
        bail!("{}", t!("mount.mount_point_must_absolute"));
    }
    
    let path_str = mount_point.to_string_lossy();
    if path_str.contains('\0') || path_str.contains("..") {
        bail!("{}", t!("mount.invalid_mount_point_path"));
    }
    
    let output = Command::new("umount")
        .arg(mount_point)
        .output()
        .context(t!("mount.failed_execute_umount").to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", t!("mount.failed_unmount", error = stderr.trim()));
    }

    Ok(())
}

/// Check if a path is currently mounted
/// 
/// # Security
/// - Uses canonical paths for reliable comparison
pub fn is_mounted(path: &Path) -> Result<bool> {
    let mounts = std::fs::read_to_string("/proc/mounts")
        .context(t!("mount.failed_read_proc_mounts").to_string())?;
    
    let canonical_path = path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let mounted_on = Path::new(parts[1]);
            let canonical_mounted = mounted_on.canonicalize()
                .unwrap_or_else(|_| mounted_on.to_path_buf());
            
            if canonical_mounted == canonical_path {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}
