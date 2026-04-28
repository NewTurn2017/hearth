# Hearth 설치 가이드 (한글)

에이전트 (Claude Code · Codex 등) 에서 `hearth` CLI + v1 스킬을 한 줄로 설치하는 방법.

## 한 줄 설치

```bash
curl -sSL https://raw.githubusercontent.com/withgenie/hearth/main/scripts/install.sh | bash
```

이 명령이 하는 일:

1. 플랫폼 감지 (`Darwin-arm64` → macOS Apple Silicon, `Linux-x86_64` → 리눅스).
2. 최신 GitHub Release 버전을 조회.
3. `hearth` 바이너리를 `~/.local/bin/hearth` 로 설치.
4. 통합 `hearth` 스킬을 `~/.local/share/hearth/skills-<version>/` 로 압축 해제하고, 이전 버전의 개별 Hearth 스킬 심링크가 남아 있으면 정리합니다.
5. 감지된 에이전트 호스트 디렉토리 (`~/.claude/skills`, `~/.codex/skills`) 로 심링크 생성.
6. `hearth db path` 로 동작 확인.

## 환경 변수

| 변수                 | 기본값                  | 용도                                                                  |
| -------------------- | ----------------------- | --------------------------------------------------------------------- |
| `HEARTH_VERSION`     | `<latest>`              | 특정 태그 고정 (예: `v0.8.0`)                                         |
| `HEARTH_BIN_DIR`     | `~/.local/bin`          | 바이너리 설치 경로                                                    |
| `HEARTH_SKILLS_DIR`  | 자동 감지               | 스킬 심링크 경로. 하나만 지원; 여러 곳 필요하면 스크립트 여러 번 실행 |
| `HEARTH_STAGING_DIR` | `~/.local/share/hearth` | 스킬 버전별 staging 경로                                              |

예시:

```bash
HEARTH_VERSION=v0.8.0 HEARTH_BIN_DIR=~/bin \
  curl -sSL https://raw.githubusercontent.com/withgenie/hearth/main/scripts/install.sh | bash
```

## 플래그

`bash -s -- <flag>` 형식으로 전달:

- `--version X.Y.Z` — 특정 버전 설치
- `--prefix DIR` — 바이너리 설치 경로
- `--skills-dir DIR` — 스킬 심링크 경로
- `--uninstall` — 제거
- `--dry-run` — 실제 쓰기 없이 계획만 출력

예시:

```bash
curl -sSL ... | bash -s -- --version v0.8.0 --dry-run
```

## PATH 추가

`~/.local/bin` 이 `$PATH` 에 없으면 설치는 됐지만 `hearth` 명령이 안 먹음:

```bash
# zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc
```

확인:

```bash
which hearth
hearth db path
```

## macOS Gatekeeper (첫 실행)

CLI 는 notarize 되지 않습니다 (의도된 선택 — GUI 가 아니어서 Gatekeeper 가 curl|bash 경로에 안 맞음). 첫 실행 시 "cannot be opened" 에러가 나오면:

```bash
xattr -d com.apple.quarantine ~/.local/bin/hearth
```

다시 실행:

```bash
hearth db path
```

한 번만 하면 됩니다.

## 에이전트 호스트별 동작

설치 스크립트는 다음을 자동 감지합니다:

- `~/.claude` 존재 → `~/.claude/skills/` 로 심링크
- `~/.codex` 존재 → `~/.codex/skills/` 로 심링크
- 둘 다 있으면 둘 다 심링크
- 둘 다 없으면 `~/.claude/skills/` 로 기본

수동 지정:

```bash
HEARTH_SKILLS_DIR=~/.codex/skills \
  curl -sSL https://raw.githubusercontent.com/withgenie/hearth/main/scripts/install.sh | bash
```

## 업그레이드

같은 한 줄 설치 명령을 다시 실행하세요. 바이너리와 심링크가 최신으로 교체됩니다. 이전 버전의 스킬은 `~/.local/share/hearth/skills-<old>/` 에 그대로 남아있습니다 (롤백용).

## 삭제

```bash
curl -sSL https://raw.githubusercontent.com/withgenie/hearth/main/scripts/install.sh | bash -s -- --uninstall
```

스크립트가 한 일만 되돌립니다 — 바이너리와 우리가 만든 심링크만 제거. staging 디렉토리는 보존됩니다. 완전 삭제:

```bash
rm -rf ~/.local/share/hearth
```

## 문제 해결

### `hearth: command not found`

PATH 에 `~/.local/bin` 이 없습니다. 위 "PATH 추가" 섹션 참고.

### `cannot be opened because the developer cannot be verified`

macOS Gatekeeper. 위 "macOS Gatekeeper" 섹션 참고.

### `SHA256 checksum verification failed`

다운로드가 중간에 끊겼거나 CDN 캐시 문제. 재실행해 보세요. 계속되면 issue 로 제보.

### `unsupported platform`

현재 macOS aarch64 (Apple Silicon) 와 Linux x86_64 만 지원합니다. 다른 플랫폼은 소스에서 빌드:

```bash
git clone https://github.com/withgenie/hearth.git
cd hearth/src-tauri && cargo build --release -p hearth-cli
```

그리고 `target/release/hearth` 를 PATH 에 둡니다.

### `jq: command not found` 등 도구 부재

`curl` 과 `tar`, `shasum` (또는 `sha256sum`) 만 있으면 됩니다. 리눅스 최소 환경:

```bash
apt-get update && apt-get install -y curl tar ca-certificates
```

## 관련 문서

- CLI 사용법: [`docs/hearth-cli-ko.md`](./hearth-cli-ko.md)
- 스킬 개요: [`skills/README.md`](../skills/README.md)
- 디자인 스펙: [`docs/superpowers/specs/2026-04-23-hearth-auto-deploy-design.md`](./superpowers/specs/2026-04-23-hearth-auto-deploy-design.md)
