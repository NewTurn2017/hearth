// Integrations section. Surfaces Hearth's "AI agent driveable workspace"
// positioning per D spec Q6 — the bundled `hearth` skill (Claude Code /
// Codex) and the `hearth-cli` binary that lets external AI agents drive
// the app's database, with running tabs auto-refreshing in real time.
//
// This is intentionally the first item users see in this tab — it is the
// 1.0 differentiator, not a buried footnote.

import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, Sparkles, Terminal } from "lucide-react";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";

const SKILL_DOCS_URL = "https://hearth.codewithgenie.com/docs/hearth-skill";
const CLI_DOCS_URL = "https://hearth.codewithgenie.com/docs/hearth-cli";

function open(url: string) {
  void openUrl(url).catch((e) => console.error("openUrl failed:", e));
}

export function SettingsIntegrationsSection() {
  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          AI 에이전트로 Hearth 조작하기
        </h3>
        <p className="text-[12px] text-[var(--color-text-muted)] mb-3">
          Hearth는 외부 AI 에이전트가 직접 조작할 수 있는 워크스페이스입니다.
          아래 두 도구를 함께 쓰면 Claude Code · Codex 같은 에이전트가 자연어로
          프로젝트·메모·일정을 만들고, 열려 있는 Hearth 탭이 실시간으로
          새로고침됩니다.
        </p>
      </section>

      <section className="rounded-md border border-[var(--color-border)] p-4">
        <div className="flex items-start gap-3">
          <Icon icon={Sparkles} size={18} className="mt-0.5" />
          <div className="flex-1">
            <h4 className="text-[13px] text-[var(--color-text-hi)] mb-1">
              Hearth skill (Claude Code · Codex)
            </h4>
            <p className="text-[12px] text-[var(--color-text-muted)] mb-3">
              Claude Code와 Codex CLI에 설치되는 라우터 스킬. "오늘 PR 정리해서
              새 프로젝트로 묶고 내일 오후 3시 리뷰 회의 잡아줘" 같은 자연어
              요청을 분류해 hearth-cli 호출로 변환합니다. 모든 변경은
              propose → approve → apply 게이트를 통과한 뒤 실행됩니다.
            </p>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                rightIcon={ExternalLink}
                onClick={() => open(SKILL_DOCS_URL)}
              >
                설치 안내 보기
              </Button>
            </div>
          </div>
        </div>
      </section>

      <section className="rounded-md border border-[var(--color-border)] p-4">
        <div className="flex items-start gap-3">
          <Icon icon={Terminal} size={18} className="mt-0.5" />
          <div className="flex-1">
            <h4 className="text-[13px] text-[var(--color-text-hi)] mb-1">
              hearth-cli
            </h4>
            <p className="text-[12px] text-[var(--color-text-muted)] mb-3">
              Hearth 데이터베이스를 명령줄에서 직접 조작하는 오픈소스 CLI.
              스킬이 내부적으로 호출하지만, 셸 스크립트 · 자동화 워크플로에서
              직접 사용해도 됩니다. Homebrew로 한 줄 설치:
            </p>
            <pre className="text-[11px] font-mono rounded bg-black/40 px-2 py-1 mb-3 select-all">
              brew install withgenie/hearth/hearth-cli
            </pre>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                rightIcon={ExternalLink}
                onClick={() => open(CLI_DOCS_URL)}
              >
                문서 열기
              </Button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
