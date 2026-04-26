# AI Agent Control Surface — 1.0 출시 + 1.1+ 발전 방향

- **재정리 일자**: 2026-04-26
- **상태**: **이미 1.0에 출시 완료된 핵심 차별점** (이전 박제 분류 정정)
- **D 스펙 반영**: Q6 결정으로 description, subtitle, screenshot #4, App Preview hero moment 모두 이 시그널 중심으로 재설계

## 1.0에 이미 들어가는 것 (출시 product)

| 부품 | 위치 | 역할 |
|------|------|------|
| `hearth-cli` 바이너리 | `src-tauri/cli/src/cmd/` (category, export, import, log, memo, project, schedule, search, views) | 거의 전 entity CRUD + 검색 + 로그 CLI surface |
| `hearth` 통합 라우터 스킬 | `skills/hearth/SKILL.md` | 자연어 요청 → hearth-cli 호출 매핑 |
| 설치 스크립트 | `scripts/install-skills.sh` | `~/.claude/skills/hearth`, `~/.codex/skills/hearth`로 심링크 |
| 외부 DB write 자동 새로고침 | commit `9ff38f3` "Refresh active Hearth tabs after external DB writes" | hearth-cli가 DB 변경 → Hearth 본체 활성 탭 즉시 새로고침 |
| 사용자 문서 | README §55, §202, §244-255 + `docs/install-ko.md` + `docs/hearth-cli-ko.md` | 한국어 설치·사용 가이드 |

**즉, 1.0 출시 시점에 사용자는 다음이 가능:**

```
$ ./scripts/install-skills.sh --into ~/.claude/skills

# Claude Code 세션 안에서:
> 오늘 PR #123 작업한 내용 정리해서 "결제 모듈 리팩토링" 프로젝트로 묶고
> 내일 오후 3시 리뷰 회의 잡아줘

# CC가 hearth 스킬 호출 → hearth-cli 다중 호출:
#   hearth project create "결제 모듈 리팩토링"
#   hearth memo move ... --to-project ...
#   hearth schedule create "리뷰 회의" --datetime 2026-04-27T15:00:00
# → Hearth 앱 활성 탭이 자동 새로고침되어 변경사항 즉시 노출
```

이 흐름 전체가 **1.0에 작동**.

## 1.0 마케팅 전략 (D 스펙 Q6 반영)

이 차별점이 무명 1.0에 가장 강한 hook이라는 판단:
- **다른 productivity 앱**: AI는 앱 안에서만 동작 (Notion AI, Mem AI, Reflect, Bear) — closed-world AI
- **Hearth 1.0**: AI agent가 **앱 외부에서** 앱을 조작 — open-world AI agent surface

D 스펙 반영 위치:
- Subtitle: `Local-first AI agent workspace` (en) / `로컬 AI 에이전트 워크스페이스` (ko)
- Description: 별도 단락 "Drive Hearth from your AI agent" 추가
- Keywords: `cli`, `automation` (en) / `자동화`, `개발자도구`, `바이브코딩` (ko)
- Screenshot #4: CC 터미널 + Hearth 앱 분할 컷
- App Preview: 0:18-0:25 hero moment (7초)
- Settings: Integrations 섹션에 `hearth` skill 카드
- Review Notes §5 Optional Features에 명시
- G 캠페인 strap line 후보: "AI workspace **that AI can drive**"

## 1.1+ 발전 방향 (1.0 출시 후 별도 브레인스토밍 대상)

다음은 **1.0에 없는** advanced 기능. 출시 후 별도 세션에서 깊이 파기:

### A) 양방향 컨텍스트
- 현재(1.0): CC → Hearth 단방향 (CC가 hearth-cli 호출)
- 1.1+: Hearth → CC도 가능. 현재 활성 프로젝트·메모를 CC 세션 컨텍스트에 자동 주입
- 결정 필요: 채널(file watch? IPC? unix socket? MCP server?)

### B) CWD 자동 매칭
- CC 세션의 `cwd` / `git rev-parse --show-toplevel` / 현재 PR 번호를 Hearth가 자동 인지
- 그 정보를 기반으로 "이 작업과 관련된 Hearth 프로젝트"를 자동 매칭·제안

### C) 권한 모델
- 1.0: hearth-cli는 OS user 권한으로 실행, mutation 무제한
- 1.1+: destructive op (delete project, bulk update) 시 Hearth UI에서 confirmation prompt
- 라이선스 게이트(B 스펙) 정책과 cross-check 필요 — read-only 모드에서 hearth-cli mutation 차단 vs 허용

### D) MAS 샌드박스 IPC 검증
- MAS production 빌드는 `com.apple.security.app-sandbox` 활성
- hearth-cli는 OSS 별도 바이너리 → MAS 본체와 직접 IPC 어려움
- 현재 1.0 동작 원리: 양쪽이 같은 SQLite 파일을 읽고/쓰고, 변경 감지로 새로고침
- 1.1+에서 더 풍부한 통신(예: 실시간 진행 상황 보고)이 필요해지면 IPC 채널 설계 재검토

### E) `hearth` 스킬 자체의 발전
- 1.0: 자연어 → 단일 hearth-cli 호출 매핑
- 1.1+: 다단계 작업(여러 cli 호출 + 결과 검증 + 사용자 확인)을 스킬 안에서 오케스트레이션
- ChatGPT 함수 호출 / Anthropic tool use 패턴 활용

## 1.0 출시 후 액션 항목

- [ ] 1.0 출시 D+1주: 사용자 피드백 수집 — `hearth` skill 사용 빈도·패턴 트래킹 (자체 텔레메트리 없으니 GitHub issue·이메일·Twitter 멘션 모니터링)
- [ ] 1.0 출시 D+30일: 1.1+ 별도 브레인스토밍 세션 예약 (위 A-E 항목)
- [ ] G 캠페인(런치 캠페인)에 핵심 hook으로 활용 — Show HN / Reddit r/macapps / 한국 IT 커뮤니티 발신문에 "AI workspace that AI can drive" 강조

## 박제 → 전략 자산 reframe 사유

이전(같은 일자 17:30 작성) 박제는 이 기능을 "1.1+ 후보"로 잘못 분류했음. 사용자 피드백("이미 구현 완료되었는데 출시 안 하나?")으로 정정. 1.0 product surface area의 일부이며, 무명 1.0의 가장 강한 마케팅 시그널로 D 스펙 Q6 결정에 반영됨.
