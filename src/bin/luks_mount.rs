//! luks_mount - Mount LUKS encrypted volumes with ease
//!
//! This binary provides a secure interface for mounting LUKS encrypted volumes
//! with automatic mapper name generation and proper cleanup on failure.

use anyhow::{bail, Context, Result};
use clap::{Arg, ArgAction, Command};
use rust_i18n::t;
use secrecy::SecretString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use luksctl::i18n::init_locale;
use luksctl::luks::{is_luks_device, luks_open};
use luksctl::mapper::{generate_mapper_name, get_mapper_path, mapper_exists, store_mount_mapping};
use luksctl::mount::{mount_device, MountOptions};

rust_i18n::i18n!("locales", fallback = "en");

fn build_cli() -> Command {
    Command::new("luks_mount")
        .about(t!("help.luks_mount.about").to_string())
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("device")
                .help(t!("help.luks_mount.device").to_string())
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("mount_point")
                .help(t!("help.luks_mount.mount_point").to_string())
                .required(true)
                .index(2)
        )
        .arg(
            Arg::new("mkdir")
                .long("mkdir")
                .help(t!("help.luks_mount.mkdir").to_string())
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("ro")
                .long("ro")
                .short('r')
                .help(t!("help.luks_mount.ro").to_string())
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fs_type")
                .long("fs-type")
                .short('t')
                .help(t!("help.luks_mount.fs_type").to_string())
                .value_name("TYPE")
        )
        .arg(
            Arg::new("options")
                .long("options")
                .short('o')
                .help(t!("help.luks_mount.options").to_string())
                .value_name("OPTIONS")
        )
}

fn main() -> Result<()> {
    // Initialize locale from LANG environment variable
    init_locale();

    let matches = build_cli().get_matches();

    let device = PathBuf::from(matches.get_one::<String>("device").unwrap());
    let mount_point = PathBuf::from(matches.get_one::<String>("mount_point").unwrap());
    let mkdir = matches.get_flag("mkdir");
    let ro = matches.get_flag("ro");
    let fs_type = matches.get_one::<String>("fs_type").cloned();
    let options = matches.get_one::<String>("options").cloned();

    // Check if running as root
    if !nix::unistd::Uid::effective().is_root() {
        bail!("{}", t!("luks_mount.program_must_root"));
    }

    // Validate device path is absolute
    if !device.is_absolute() {
        bail!("{}", t!("luks_mount.device_path_must_absolute"));
    }

    // Check for path traversal attempts
    let device_str = device.to_string_lossy();
    if device_str.contains("..") || device_str.contains('\0') {
        bail!("{}", t!("luks_mount.invalid_device_path"));
    }

    // Check if device exists
    if !device.exists() {
        bail!("{}", t!("luks_mount.device_not_exist", path = device.display().to_string()));
    }

    // Check if device is a LUKS device
    if !is_luks_device(&device)? {
        bail!("{}", t!("luks_mount.device_not_luks", path = device.display().to_string()));
    }

    // Validate mount point path
    if !mount_point.is_absolute() {
        bail!("{}", t!("luks_mount.mount_point_must_absolute"));
    }

    let mount_str = mount_point.to_string_lossy();
    if mount_str.contains("..") || mount_str.contains('\0') {
        bail!("{}", t!("luks_mount.invalid_mount_point"));
    }

    // Create mount point if --mkdir is specified
    if mkdir && !mount_point.exists() {
        fs::create_dir_all(&mount_point)
            .context(t!("errors.failed_create_mount_dir").to_string())?;
        // Set secure permissions on created directory (0755)
        fs::set_permissions(&mount_point, fs::Permissions::from_mode(0o755))
            .context(t!("errors.failed_set_permissions").to_string())?;
        println!("{}", t!("luks_mount.created_mount_point", path = mount_point.display().to_string()));
    }

    // Check if mount point exists
    if !mount_point.exists() {
        bail!("{}", t!("luks_mount.mount_point_not_exist", path = mount_point.display().to_string()));
    }

    // Check if mount point is a directory
    if !mount_point.is_dir() {
        bail!("{}", t!("luks_mount.mount_point_not_dir", path = mount_point.display().to_string()));
    }

    // Generate unique mapper name with retry limit
    const MAX_RETRIES: u32 = 10;
    let mapper_name = {
        let mut attempts = 0;
        loop {
            let name = generate_mapper_name();
            if !mapper_exists(&name) {
                break name;
            }
            attempts += 1;
            if attempts >= MAX_RETRIES {
                bail!("{}", t!("luks_mount.failed_generate_mapper", count = MAX_RETRIES));
            }
        }
    };

    println!("{}", t!("luks_mount.opening_luks_device", path = device.display().to_string()));
    println!("{}", t!("luks_mount.using_mapper", name = &mapper_name));

    // Prompt for password - wrapped in SecretString for secure handling
    let password_raw = rpassword::prompt_password(t!("luks_mount.enter_passphrase").to_string())
        .context(t!("luks_mount.failed_read_password").to_string())?;
    
    // Wrap in SecretString for zeroization on drop
    let password = SecretString::from(password_raw);

    // Open LUKS device
    luks_open(&device, &mapper_name, &password)?;
    // password is automatically zeroized when dropped here
    
    println!("{}", t!("luks_mount.luks_opened_success"));

    // Get mapper device path
    let mapper_path = get_mapper_path(&mapper_name);

    // Prepare mount options
    let mount_options = MountOptions {
        read_only: ro,
        fs_type,
        options,
    };

    // Mount the device
    println!("{}", t!("luks_mount.mounting_to", path = mount_point.display().to_string()));
    if let Err(e) = mount_device(&mapper_path, &mount_point, &mount_options) {
        // If mount fails, close the LUKS device
        eprintln!("{}", t!("luks_mount.mount_failed_closing"));
        let _ = luksctl::luks::luks_close(&mapper_name);
        return Err(e);
    }

    // Store the mapping for later unmount
    store_mount_mapping(&mount_point, &mapper_name, &device)?;

    println!("\n{}", t!("luks_mount.success_mounted"));
    println!("{}", t!("luks_mount.label_device", path = device.display().to_string()));
    println!("{}", t!("luks_mount.label_mount_point", path = mount_point.display().to_string()));
    println!("{}", t!("luks_mount.label_mapper", name = &mapper_name));
    println!("{}", t!("luks_mount.label_security"));
    if mount_options.read_only {
        println!("{}", t!("luks_mount.label_mode_readonly"));
    }

    Ok(())
}
