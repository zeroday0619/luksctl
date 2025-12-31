# luksctl

[![Crates.io](https://img.shields.io/crates/v/luksctl.svg)](https://crates.io/crates/luksctl)
[![Downloads](https://img.shields.io/crates/d/luksctl.svg)](https://crates.io/crates/luksctl)

LUKS暗号化ボリュームを簡単にマウント・アンマウントするためのCLIツールです。

[English](README.md) | [한국어](README.ko.md)

## 機能

- 🔐 LUKS暗号化ボリュームの簡単なマウント・アンマウント
- 🆔 UUIDベースのmapper名自動生成（衝突防止）
- 📁 `--mkdir`オプションでマウントポイントを自動作成
- ⚙️ 多様なマウントオプション対応（`--ro`、`--fs-type`、`--options`）
- 🌐 多言語対応（英語、韓国語、日本語）

## インストール

### Cargoを使用（推奨）

```bash
cargo install luksctl
```

### Makeを使用

```bash
# ビルドして/usr/local/binにインストール
make
sudo make install

# カスタムの場所にインストール
sudo make PREFIX=/opt/luksctl install

# アンインストール
sudo make uninstall
```

### 手動インストール

```bash
cargo build --release
sudo cp target/release/luks_mount /usr/local/bin/
sudo cp target/release/luks_umount /usr/local/bin/
```

## 使い方

### マウント

```bash
# 基本的な使い方
sudo luks_mount /dev/sda1 /mnt/encrypted

# マウントポイントが存在しない場合は自動作成
sudo luks_mount --mkdir /dev/Video/nvme_video /mnt/nvme_video

# 読み取り専用でマウント
sudo luks_mount --ro /dev/sda1 /mnt/encrypted

# ファイルシステムタイプを指定
sudo luks_mount --fs-type ext4 /dev/sda1 /mnt/encrypted

# 追加のマウントオプションを指定
sudo luks_mount --options "noatime,nodiratime" /dev/sda1 /mnt/encrypted

# すべてのオプションを組み合わせる
sudo luks_mount --mkdir --ro --fs-type ext4 --options "noatime" /dev/sda1 /mnt/encrypted
```

### アンマウント

```bash
# 基本的なアンマウント（自動的にLUKSをロック）
sudo luks_umount /mnt/encrypted

# 強制アンマウント（遅延アンマウント）
sudo luks_umount --force /mnt/encrypted
```

## コマンドオプション

### luks_mount

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--mkdir` | | マウントポイントディレクトリが存在しない場合は作成 |
| `--ro` | `-r` | 読み取り専用でマウント |
| `--fs-type` | `-t` | ファイルシステムタイプを指定（例：ext4、xfs、btrfs） |
| `--options` | `-o` | 追加のマウントオプション（カンマ区切り） |

### luks_umount

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--force` | `-f` | 強制アンマウント（遅延アンマウント） |

## 多言語対応

ツールは`LANG`環境変数からシステムのロケールを自動検出し、適切な言語でメッセージを表示します。

対応言語:
- 英語 (en) - デフォルト
- 韓国語 (ko)
- 日本語 (ja)

例:

```bash
# 韓国語を使用
LANG=ko_KR.UTF-8 sudo luks_mount /dev/sda1 /mnt/encrypted

# 日本語を使用
LANG=ja_JP.UTF-8 sudo luks_umount /mnt/encrypted
```

## 動作原理

1. **マウント時（`luks_mount`）**:
   - デバイスがLUKSデバイスであることを確認
   - UUIDベースのユニークなmapper名を生成（例：`luks-a1b2c3d4-...`）
   - パスワードを入力し、`cryptsetup open`を実行
   - `/dev/mapper/{mapper_name}`を指定されたマウントポイントにマウント
   - マウント情報を`/run/luksctl/`に保存

2. **アンマウント時（`luks_umount`）**:
   - 保存されたマッピング情報または`/proc/mounts`からmapper名を検索
   - ファイルシステムをアンマウント
   - `cryptsetup close`でLUKSデバイスをロック

## ライセンス

[Menhera Open Source License](LICENSE)

## AI生成コードに関する告知

このプロジェクトの一部は、AIツール（大規模言語モデルなど）の支援を受けて作成されました。すべてのAI支援による貢献は、含める前にメンテナーによってレビューおよび調整されています。特定の変更の出所が必要な場合は、Gitの履歴とコミットメッセージを参照してください。
