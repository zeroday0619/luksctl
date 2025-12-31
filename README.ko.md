# luksctl

LUKS 암호화 볼륨을 쉽게 마운트/언마운트하는 CLI 도구입니다.

[English](README.md) | [日本語](README.ja.md)

## 특징

- 🔐 LUKS 암호화 볼륨의 간편한 마운트/언마운트
- 🆔 UUID 기반 mapper 이름 자동 생성 (충돌 방지)
- 📁 `--mkdir` 옵션으로 마운트 포인트 자동 생성
- ⚙️ 다양한 mount 옵션 지원 (`--ro`, `--fs-type`, `--options`)
- 🌐 다국어 지원 (영어, 한국어, 일본어)

## 설치

### Make 사용 (권장)

```bash
# 빌드 후 /usr/local/bin에 설치
make
sudo make install

# 또는 다른 경로에 설치
sudo make PREFIX=/opt/luksctl install

# 제거
sudo make uninstall
```

### 수동 설치

```bash
cargo build --release
sudo cp target/release/luks_mount /usr/local/bin/
sudo cp target/release/luks_umount /usr/local/bin/
```

## 사용법

### 마운트

```bash
# 기본 사용법
sudo luks_mount /dev/sda1 /mnt/encrypted

# 마운트 포인트가 없으면 자동 생성
sudo luks_mount --mkdir /dev/Video/nvme_video /mnt/nvme_video

# 읽기 전용으로 마운트
sudo luks_mount --ro /dev/sda1 /mnt/encrypted

# 파일시스템 타입 지정
sudo luks_mount --fs-type ext4 /dev/sda1 /mnt/encrypted

# 추가 mount 옵션 지정
sudo luks_mount --options "noatime,nodiratime" /dev/sda1 /mnt/encrypted

# 모든 옵션 조합
sudo luks_mount --mkdir --ro --fs-type ext4 --options "noatime" /dev/sda1 /mnt/encrypted
```

### 언마운트

```bash
# 기본 언마운트 (자동으로 LUKS 락킹)
sudo luks_umount /mnt/encrypted

# 강제 언마운트 (lazy unmount)
sudo luks_umount --force /mnt/encrypted
```

## 명령어 옵션

### luks_mount

| 옵션 | 단축 | 설명 |
|------|------|------|
| `--mkdir` | | 마운트 포인트 디렉토리가 없으면 생성 |
| `--ro` | `-r` | 읽기 전용으로 마운트 |
| `--fs-type` | `-t` | 파일시스템 타입 지정 (예: ext4, xfs, btrfs) |
| `--options` | `-o` | 추가 mount 옵션 (쉼표로 구분) |

### luks_umount

| 옵션 | 단축 | 설명 |
|------|------|------|
| `--force` | `-f` | 강제 언마운트 (lazy unmount) |

## 다국어 지원

`LANG` 환경변수에서 시스템 로케일을 자동으로 감지하여 적절한 언어로 메시지를 표시합니다.

지원 언어:
- 영어 (en) - 기본
- 한국어 (ko)
- 일본어 (ja)

예시:

```bash
# 한국어 사용
LANG=ko_KR.UTF-8 sudo luks_mount /dev/sda1 /mnt/encrypted

# 일본어 사용
LANG=ja_JP.UTF-8 sudo luks_umount /mnt/encrypted
```

## 작동 방식

1. **마운트 시 (`luks_mount`)**:
   - LUKS 장치인지 확인
   - UUID 기반 고유 mapper 이름 생성 (예: `luks-a1b2c3d4-...`)
   - 비밀번호 입력 받아 `cryptsetup open` 실행
   - `/dev/mapper/{mapper_name}`을 지정된 마운트 포인트에 마운트
   - 마운트 정보를 `/run/luksctl/`에 저장

2. **언마운트 시 (`luks_umount`)**:
   - 저장된 매핑 정보 또는 `/proc/mounts`에서 mapper 이름 찾기
   - 파일시스템 언마운트
   - `cryptsetup close`로 LUKS 장치 락킹

## 라이선스

[Menhera Open Source License](LICENSE)

## AI 생성 코드 고지

이 프로젝트의 일부는 AI 도구(예: 대규모 언어 모델)의 도움을 받아 작성되었습니다. 모든 AI 지원 기여는 포함 전에 메인테이너가 검토하고 수정했습니다. 특정 변경 사항의 출처가 필요한 경우 Git 히스토리와 커밋 메시지를 참조하세요.
