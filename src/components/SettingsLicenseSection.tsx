// License section. Per B spec, the license state machine has three terminal
// states the UI cares about: Trial { days_left } / Purchased / TrialExpired
// (read-only). The `useLicense` hook lives on the `feat/iap-license`
// worktree (not yet on main). Until B merges, we render a stub that
// compiles cleanly and matches the final layout so swap-in is mechanical.
//
// TODO(B-merge): replace the stub with `useLicense()` from `src/license/`
// once feat/iap-license merges to main. The hook returns:
//   { status, daysLeft, isReadOnly, requestPurchase, restorePurchase }
// per docs/superpowers/specs/2026-04-26-iap-license-design.md §1.

import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { Button } from "../ui/Button";

// TODO(B-merge): import { useLicense } from "../license/useLicense";
type StubLicense =
  | { status: "trial"; daysLeft: number }
  | { status: "purchased" }
  | { status: "trial_expired" }
  | { status: "unknown" };

function useLicenseStub(): StubLicense {
  // While the real hook is on a worktree, we render in "unknown" state so
  // the section is visible but doesn't lie about entitlement.
  return { status: "unknown" };
}

export function SettingsLicenseSection() {
  const license = useLicenseStub();

  let label: string;
  let detail: string;
  switch (license.status) {
    case "trial":
      label = "체험판";
      detail = `평가 기간이 ${license.daysLeft}일 남았습니다.`;
      break;
    case "purchased":
      label = "Hearth Pro";
      detail = "구매가 완료되어 모든 기능이 영구적으로 활성화되었습니다.";
      break;
    case "trial_expired":
      label = "읽기 전용";
      detail =
        "14일 평가 기간이 끝났습니다. Hearth Pro를 구매하면 모든 기능이 다시 열립니다.";
      break;
    default:
      label = "확인 중";
      detail = "라이선스 상태를 불러오는 중입니다.";
  }

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          라이선스
        </h3>
        <div className="rounded-md border border-[var(--color-border)] p-4">
          <div className="flex items-baseline justify-between mb-1">
            <span className="text-[14px] text-[var(--color-text-hi)]">
              {label}
            </span>
            <span className="text-[11px] font-mono text-[var(--color-text-muted)]">
              io.hearth.app.pro
            </span>
          </div>
          <p className="text-[12px] text-[var(--color-text-muted)]">{detail}</p>

          {/* TODO(B-merge): wire these up to requestPurchase / restorePurchase */}
          <div className="flex items-center gap-2 mt-4">
            <Button size="sm" variant="primary" disabled>
              Hearth Pro 구매
            </Button>
            <Button size="sm" variant="secondary" disabled>
              구매 복원
            </Button>
          </div>
          <p className="text-[11px] text-[var(--color-text-muted)] mt-3">
            결제는 Apple App Store가 처리하며, 가족 공유를 지원합니다. Hearth
            서버는 존재하지 않으므로 결제 정보는 Apple 외부로 전송되지 않습니다.
          </p>
        </div>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          개인정보 처리방침
        </h3>
        <p className="text-[12px] text-[var(--color-text-muted)] mb-3">
          Hearth는 어떠한 개인 데이터도 수집하지 않습니다. 모든 데이터는 사용자
          Mac의 SQLite 파일에만 저장됩니다.
        </p>
        <Button
          size="sm"
          variant="secondary"
          rightIcon={ExternalLink}
          onClick={() =>
            void openUrl("https://hearth.codewithgenie.com/privacy").catch(
              (e) => console.error("openUrl failed:", e),
            )
          }
        >
          전체 정책 보기
        </Button>
      </section>
    </div>
  );
}
