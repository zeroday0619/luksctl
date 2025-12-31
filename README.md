# luksctl

A CLI tool for easily mounting and unmounting LUKS encrypted volumes.

[한국어](README.ko.md) | [日本語](README.ja.md)

## Features

- 🔐 Easy mount/unmount of LUKS encrypted volumes
- 🆔 UUID-based mapper name auto-generation (collision prevention)
- 📁 Auto-create mount point with `--mkdir` option
- ⚙️ Various mount options support (`--ro`, `--fs-type`, `--options`)
- 🌐 Multi-language support (English, Korean, Japanese)

## Installation

### Using Make (Recommended)

```bash
# Build and install to /usr/local/bin
make
sudo make install

# Or install to a custom location
sudo make PREFIX=/opt/luksctl install

# Uninstall
sudo make uninstall
```

### Manual Installation

```bash
cargo build --release
sudo cp target/release/luks_mount /usr/local/bin/
sudo cp target/release/luks_umount /usr/local/bin/
```

## Usage

### Mount

```bash
# Basic usage
sudo luks_mount /dev/sda1 /mnt/encrypted

# Auto-create mount point if it doesn't exist
sudo luks_mount --mkdir /dev/Video/nvme_video /mnt/nvme_video

# Mount as read-only
sudo luks_mount --ro /dev/sda1 /mnt/encrypted

# Specify filesystem type
sudo luks_mount --fs-type ext4 /dev/sda1 /mnt/encrypted

# Specify additional mount options
sudo luks_mount --options "noatime,nodiratime" /dev/sda1 /mnt/encrypted

# Combine all options
sudo luks_mount --mkdir --ro --fs-type ext4 --options "noatime" /dev/sda1 /mnt/encrypted
```

### Unmount

```bash
# Basic unmount (automatically locks LUKS)
sudo luks_umount /mnt/encrypted

# Force unmount (lazy unmount)
sudo luks_umount --force /mnt/encrypted
```

## Command Options

### luks_mount

| Option | Short | Description |
|--------|-------|-------------|
| `--mkdir` | | Create mount point directory if it doesn't exist |
| `--ro` | `-r` | Mount as read-only |
| `--fs-type` | `-t` | Specify filesystem type (e.g., ext4, xfs, btrfs) |
| `--options` | `-o` | Additional mount options (comma-separated) |

### luks_umount

| Option | Short | Description |
|--------|-------|-------------|
| `--force` | `-f` | Force unmount (lazy unmount) |

## Localization

The tool automatically detects your system locale from the `LANG` environment variable and displays messages in the appropriate language.

Supported languages:
- English (en) - Default
- Korean (ko)
- Japanese (ja)

Example:

```bash
# Use Korean
LANG=ko_KR.UTF-8 sudo luks_mount /dev/sda1 /mnt/encrypted

# Use Japanese
LANG=ja_JP.UTF-8 sudo luks_umount /mnt/encrypted
```

## How It Works

1. **On mount (`luks_mount`)**:
   - Verify the device is a LUKS device
   - Generate a unique UUID-based mapper name (e.g., `luks-a1b2c3d4-...`)
   - Prompt for password and execute `cryptsetup open`
   - Mount `/dev/mapper/{mapper_name}` to the specified mount point
   - Save mount information to `/run/luksctl/`

2. **On unmount (`luks_umount`)**:
   - Find mapper name from saved mapping info or `/proc/mounts`
   - Unmount the filesystem
   - Lock the LUKS device with `cryptsetup close`

## License

[Menhera Open Source License](LICENSE)

## AI-Generated Code Notice

Parts of this project were created with assistance from AI tools (e.g. large language models). All AI-assisted contributions were reviewed and adapted by maintainers before inclusion. If you need provenance for specific changes, please refer to the Git history and commit messages.
