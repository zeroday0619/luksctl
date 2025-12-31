//! Mapper name management using UUID-based naming to prevent conflicts
//!
//! This module handles mapper name generation and state file management with
//! security hardening:
//! - UUID-based naming to prevent collisions
//! - Secure file permissions for state files
//! - Input validation and sanitization

use anyhow::{bail, Context, Result};
use rust_i18n::t;
use std::fs::{self, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAPPER_DIR: &str = "/dev/mapper";
const MAPPER_STATE_DIR: &str = "/run/luksctl";

/// Secure file permissions: owner read/write only (0600)
const STATE_FILE_PERMS: u32 = 0o600;
/// Secure directory permissions: owner read/write/execute only (0700)
const STATE_DIR_PERMS: u32 = 0o700;

/// Maximum length for escaped mount point names
const MAX_ESCAPED_NAME_LEN: usize = 255;

/// Generate a unique mapper name using UUID
/// 
/// Uses UUID v4 for cryptographically secure random generation
pub fn generate_mapper_name() -> String {
    let uuid = Uuid::new_v4();
    format!("luks-{}", uuid)
}

/// Get the mapper device path
pub fn get_mapper_path(mapper_name: &str) -> PathBuf {
    Path::new(MAPPER_DIR).join(mapper_name)
}

/// Check if a mapper name already exists
pub fn mapper_exists(mapper_name: &str) -> bool {
    get_mapper_path(mapper_name).exists()
}

/// Safely escape a mount point path for use as a filename
/// 
/// # Security
/// - Validates input length
/// - Replaces path separators safely
/// - Prevents null byte injection
fn escape_mount_path(mount_point: &Path) -> Result<String> {
    let path_str = mount_point.to_string_lossy();
    
    // Check for null bytes
    if path_str.contains('\0') {
        bail!("{}", t!("mapper.path_contains_null"));
    }
    
    // Escape the path
    let escaped = path_str.replace('/', "_");
    
    // Validate length
    if escaped.len() > MAX_ESCAPED_NAME_LEN {
        bail!("{}", t!("mapper.path_too_long"));
    }
    
    // Ensure the escaped name doesn't start with a dot (hidden file)
    // and doesn't contain path traversal attempts
    if escaped.starts_with('.') || escaped.contains("..") {
        bail!("{}", t!("mapper.path_traversal_detected"));
    }
    
    Ok(escaped)
}

/// Validate mapper name format
fn validate_mapper_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 128 {
        bail!("{}", t!("mapper.name_invalid_length"));
    }
    
    // Must start with "luks-" for our managed mappers
    if !name.starts_with("luks-") {
        bail!("{}", t!("mapper.name_must_start_luks"));
    }
    
    // Check for path traversal or injection
    if name.contains('/') || name.contains('\0') || name.contains("..") {
        bail!("{}", t!("mapper.name_invalid_chars"));
    }
    
    Ok(())
}

/// Store the mapping between mount point and mapper name
/// 
/// # Security
/// - Creates state directory with restricted permissions (0700)
/// - Creates state files with restricted permissions (0600)
/// - Validates all inputs before writing
pub fn store_mount_mapping(mount_point: &Path, mapper_name: &str, device: &Path) -> Result<()> {
    // Validate inputs
    validate_mapper_name(mapper_name)?;
    
    let escaped_mount = escape_mount_path(mount_point)?;
    
    let state_dir = Path::new(MAPPER_STATE_DIR);
    
    // Create state directory with secure permissions
    if !state_dir.exists() {
        fs::create_dir_all(state_dir)
            .context(t!("mapper.failed_create_state_dir").to_string())?;
        fs::set_permissions(state_dir, Permissions::from_mode(STATE_DIR_PERMS))
            .context(t!("mapper.failed_set_state_dir_perms").to_string())?;
    }
    
    let state_file = state_dir.join(&escaped_mount);
    let content = format!("{}:{}", mapper_name, device.to_string_lossy());
    
    // Create file with secure permissions atomically
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(STATE_FILE_PERMS)
        .open(&state_file)
        .context(t!("mapper.failed_create_state_file").to_string())?;
    
    file.write_all(content.as_bytes())
        .context(t!("mapper.failed_write_state_file").to_string())?;
    
    // Ensure data is flushed to disk
    file.sync_all()
        .context(t!("mapper.failed_sync_state_file").to_string())?;
    
    Ok(())
}

/// Retrieve the mapper name and device for a mount point
/// 
/// # Security
/// - Validates the state file content format
/// - Validates retrieved mapper name
pub fn get_mount_mapping(mount_point: &Path) -> Result<Option<(String, PathBuf)>> {
    let escaped_mount = escape_mount_path(mount_point)?;
    
    let state_file = Path::new(MAPPER_STATE_DIR).join(escaped_mount);
    
    if !state_file.exists() {
        return Ok(None);
    }
    
    // Verify the state file is actually a file (not a symlink attack)
    let metadata = fs::symlink_metadata(&state_file)
        .context(t!("mapper.failed_get_metadata").to_string())?;
    
    if !metadata.is_file() {
        bail!("{}", t!("mapper.state_not_regular_file"));
    }
    
    let content = fs::read_to_string(&state_file)
        .context(t!("mapper.failed_read_state_file").to_string())?;
    
    // Limit content size to prevent DoS
    if content.len() > 1024 {
        bail!("{}", t!("mapper.state_content_too_large"));
    }
    
    let parts: Vec<&str> = content.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Ok(None);
    }
    
    let mapper_name = parts[0].to_string();
    let device_path = PathBuf::from(parts[1]);
    
    // Validate the retrieved mapper name
    validate_mapper_name(&mapper_name)?;
    
    Ok(Some((mapper_name, device_path)))
}

/// Remove the mapping for a mount point
/// 
/// # Security
/// - Validates mount point before removing
/// - Verifies target is a regular file
pub fn remove_mount_mapping(mount_point: &Path) -> Result<()> {
    let escaped_mount = escape_mount_path(mount_point)?;
    
    let state_file = Path::new(MAPPER_STATE_DIR).join(escaped_mount);
    
    if state_file.exists() {
        // Verify it's a regular file before removing
        let metadata = fs::symlink_metadata(&state_file)
            .context(t!("mapper.failed_get_metadata").to_string())?;
        
        if !metadata.is_file() {
            bail!("{}", t!("mapper.state_not_regular_file"));
        }
        
        fs::remove_file(&state_file)
            .context(t!("mapper.failed_remove_state_file").to_string())?;
    }
    
    Ok(())
}

/// Find mapper name by looking at /proc/mounts
/// 
/// # Security
/// - Validates found mapper names
/// - Uses canonical paths for comparison
pub fn find_mapper_by_mount_point(mount_point: &Path) -> Result<Option<String>> {
    let mounts = fs::read_to_string("/proc/mounts")
        .context(t!("mapper.failed_read_proc_mounts").to_string())?;
    
    let canonical_mount = mount_point.canonicalize()
        .unwrap_or_else(|_| mount_point.to_path_buf());
    
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let mounted_on = Path::new(parts[1]);
            let canonical_mounted = mounted_on.canonicalize()
                .unwrap_or_else(|_| mounted_on.to_path_buf());
            
            if canonical_mounted == canonical_mount {
                let device = parts[0];
                if let Some(mapper_name) = device.strip_prefix("/dev/mapper/") {
                    // Validate the mapper name before returning
                    if mapper_name.starts_with("luks-") && 
                       !mapper_name.contains('/') && 
                       !mapper_name.contains('\0') {
                        return Ok(Some(mapper_name.to_string()));
                    }
                }
            }
        }
    }
    
    Ok(None)
}
