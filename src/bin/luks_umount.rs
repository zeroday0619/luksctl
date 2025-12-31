//! luks_umount - Unmount and lock LUKS encrypted volumes
//!
//! This binary provides a secure interface for unmounting LUKS encrypted volumes
//! and automatically locking the underlying device.

use anyhow::{bail, Context, Result};
use clap::{Arg, ArgAction, Command};
use rust_i18n::t;
use std::path::PathBuf;

use luksctl::i18n::init_locale;
use luksctl::luks::luks_close;
use luksctl::mapper::{find_mapper_by_mount_point, get_mount_mapping, remove_mount_mapping};
use luksctl::mount::{is_mounted, unmount};

rust_i18n::i18n!("locales", fallback = "en");

fn build_cli() -> Command {
    Command::new("luks_umount")
        .about(t!("help.luks_umount.about").to_string())
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("mount_point")
                .help(t!("help.luks_umount.mount_point").to_string())
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help(t!("help.luks_umount.force").to_string())
                .action(ArgAction::SetTrue)
        )
}

fn main() -> Result<()> {
    // Initialize locale from LANG environment variable
    init_locale();

    let matches = build_cli().get_matches();

    let mount_point_arg = PathBuf::from(matches.get_one::<String>("mount_point").unwrap());
    let force = matches.get_flag("force");

    // Check if running as root
    if !nix::unistd::Uid::effective().is_root() {
        bail!("{}", t!("luks_umount.program_must_root"));
    }

    // Validate mount point path is absolute
    if !mount_point_arg.is_absolute() {
        bail!("{}", t!("luks_umount.mount_point_must_absolute"));
    }

    // Check for path traversal and null bytes
    let mount_str = mount_point_arg.to_string_lossy();
    if mount_str.contains('\0') {
        bail!("{}", t!("luks_umount.invalid_mount_point_null"));
    }

    // Canonicalize the mount point path (resolves symlinks, removes ..)
    let mount_point = mount_point_arg.canonicalize()
        .unwrap_or_else(|_| mount_point_arg.clone());

    // Double-check after canonicalization
    if !mount_point.is_absolute() {
        bail!("{}", t!("luks_umount.invalid_mount_point_canonical"));
    }

    // Check if the mount point is actually mounted
    if !is_mounted(&mount_point)? {
        bail!("{}", t!("luks_umount.mount_point_not_mounted", path = mount_point.display().to_string()));
    }

    // Try to get mapper name from our state file first
    let mapper_name = if let Some((name, _device)) = get_mount_mapping(&mount_point)? {
        Some(name)
    } else {
        // Fall back to finding it from /proc/mounts
        find_mapper_by_mount_point(&mount_point)?
    };

    let mapper_name = match mapper_name {
        Some(name) => name,
        None => bail!("{}", t!("luks_umount.mapper_not_found", path = mount_point.display().to_string())),
    };

    // Validate mapper name before using
    if mapper_name.is_empty() || 
       mapper_name.contains('/') || 
       mapper_name.contains('\0') ||
       mapper_name.contains("..") {
        bail!("{}", t!("luks_umount.invalid_mapper_detected"));
    }

    println!("{}", t!("luks_umount.unmounting", path = mount_point.display().to_string()));
    println!("{}", t!("luks_umount.mapper_info", name = &mapper_name));

    // Unmount the filesystem
    if force {
        // Use lazy unmount for force - validate path before passing
        let mount_path_str = mount_point.to_str()
            .ok_or_else(|| anyhow::anyhow!("{}", t!("luks_umount.invalid_mount_encoding")))?;
        
        let output = std::process::Command::new("umount")
            .args(["-l", mount_path_str])
            .output()
            .context("Failed to execute umount")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to unmount: {}", stderr.trim());
        }
    } else {
        unmount(&mount_point)?;
    }
    println!("{}", t!("luks_umount.filesystem_unmounted"));

    // Close the LUKS device
    println!("{}", t!("luks_umount.closing_luks"));
    luks_close(&mapper_name)?;
    println!("{}", t!("luks_umount.luks_locked"));

    // Remove our state file
    let _ = remove_mount_mapping(&mount_point);

    println!("\n{}", t!("luks_umount.success_unmounted"));
    println!("{}", t!("luks_umount.label_mount_point", path = mount_point.display().to_string()));

    Ok(())
}
