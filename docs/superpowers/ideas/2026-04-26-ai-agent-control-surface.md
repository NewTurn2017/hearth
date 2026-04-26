# Idea — AI Agent Control Surface for Hearth

- **박제 일자**: 2026-04-26
- **스코프 후보**: 1.1+ (1.0 출시 sprint에서 제외)
- **기원**: D 스펙 §3 스크린샷 작업 중 사용자 발산 — "claudecode/codex 안에서 스킬을 통해 cli 조작해서 자동으로 실시간 반영되는 INSANE 기능"

## 핵심 아이디어

**Hearth는 AI workspace이고, 그 AI workspace를 AI agent가 직접 조작할 수 있다.**

CC(Claude Code) / Codex CLI 세션 안에서 자연어 요청 → Hearth 스킬이 hearth-cli 호출 → Hearth UI에 실시간 반영.

### 사용 시나리오 예시

```
사용자 (CC 세션 안에서):
  "이 PR(/Users/genie/dev/foo/pr-123) 작업 중 떠오른 메모들 다 모아서
   '결제 모듈 리팩토링'이라는 새 프로젝트로 묶고, 내일 오후 3시
   리뷰 회고 일정 잡아줘"

CC가 hearth 스킬 호출:
  → hearth-cli search "PR #123" --type memo --output json
  → hearth-cli project create "결제 모듈 리팩토링" --output json
  → hearth-cli memo move <memo_ids> --to-project <new_project_id>
  → hearth-cli schedule create "리뷰 회고" --datetime "2026-04-27T15:00:00"
  → 변경사항 Hearth 본체 UI에 즉시 반영 (현재 활성 탭 자동 새로고침은 이미
    구현됨 — commit 9ff38f3 "Refresh active Hearth tabs after external DB writes")
```

### 차별점 (다른 productivity 앱이 못 따라하는 위치)

- 다른 노트/할일 앱: AI는 앱 안에서만 동작 (Notion AI, Mem AI, Reflect)
- Hearth: AI agent가 **앱 외부에서** 앱을 조작. CC/Codex 세션이 곧 super-command-palette
- 양방향 컨텍스트:
  - Hearth → CC: 현재 활성 프로젝트·메모 컨텍스트를 CC 세션에 주입
  - CC → Hearth: 현재 cwd/git 브랜치/PR을 Hearth가 자동 인지해서 매칭

### 기존 인프라 (이미 있는 부품)

- ✅ `hearth-cli` (서브프로젝트 C 결정 D4 — public OSS 유지) — CLI surface 존재
- ✅ `skills/hearth/` 스킬 디렉토리 — 자연어 라우터 스켈레톤 존재 (이전 세션 흔적)
- ✅ External DB write 시 active tab 자동 refresh — `commit 9ff38f3`
- ✅ tool-calling AI 명령 팔레트 — `⌘K` 안에서 이미 동작 중

### 1.0 → 1.1+ 마이그레이션 전략

**1.0 출시 시점** (2026-05-17):
- hearth-cli 그대로 유지
- 마케팅 (서브프로젝트 G 캠페인)에 "1.1 preview: AI agents control your workspace" 티저 한 줄
- 변경 코드 0

**1.1 구상** (출시 후 4-8주):
- 본격 브레인스토밍 세션 별도 진행
- 결정 필요 항목:
  - hearth-cli output JSON schema 표준화
  - skill 자연어 → CLI 매핑 카탈로그
  - CC 세션 → Hearth 양방향 컨텍스트 채널 (file watch? IPC? unix socket?)
  - 권한 모델 (CC가 destructive op 할 때 Hearth 측 confirmation?)
  - 라이선스 게이트 (read-only 모드에서 CLI 조작은? — B 스펙 정책 cross-check)

### 마케팅 시그널

- "AI workspace **that AI can drive**" — slogan candidate
- HN/Reddit Show 포스트의 hook
- Productivity twitter 인플루언서 어필 포인트

### 위험·제약

- ⚠️ 1.0 출시 sprint 일정상 1.0 스코프 진입 금지. **deadline 보호 절대 우선**
- ⚠️ 라이선스 게이트(B) 정책과 cross-check 필요 — read-only 상태에서 hearth-cli mutation 차단 vs 허용
- ⚠️ MAS 샌드박스에서 외부 프로세스(CC)와 Hearth 본체 간 IPC 가능성 검증 필요 — `com.apple.security.app-sandbox` 제약

## Action

- [ ] 1.0 출시 D+30일 시점에 별도 브레인스토밍 세션 예약
- [ ] 그동안 사용자 피드백 수집 — 1.0 출시 후 "이 기능 있으면 좋겠다" 의견 트래킹
- [ ] G 캠페인에 1.1 preview 티저 한 줄 포함 검토 (G 스펙 작성 시)
