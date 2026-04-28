// About / Version section. Per the MAS readiness spec (sub-project A, D1
// decision), Hearth 1.0 ships through the Mac App Store only — the
// in-app auto-updater is removed. Version updates flow through
// App Store → Updates, which we surface with a one-click link.

import { useEffect, useState } from "react";
import { getVersion, getTauriVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { Button } from "../ui/Button";

const APP_STORE_UPDATES_URL = "macappstore://showUpdatesPage";

export function SettingsAboutSection({ active }: { active: boolean }) {
  const [version, setVersion] = useState<string>("");
  const [tauri, setTauri] = useState<string>("");

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    void getVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {
        /* non-Tauri context (tests) */
      });
    void getTauriVersion()
      .then((v) => {
        if (!cancelled) setTauri(v);
      })
      .catch(() => {
        /* non-Tauri context */
      });
    return () => {
      cancelled = true;
    };
  }, [active]);

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          버전 정보
        </h3>
        <div className="rounded-md border border-[var(--color-border)] p-4 text-[12px]">
          <div className="flex items-center justify-between py-1">
            <span className="text-[var(--color-text-muted)]">Hearth</span>
            <span className="font-mono text-[var(--color-text-hi)]">
              {version || "—"}
            </span>
          </div>
          <div className="flex items-center justify-between py-1">
            <span className="text-[var(--color-text-muted)]">번들 ID</span>
            <span className="font-mono text-[var(--color-text)]">
              com.codewithgenie.hearth
            </span>
          </div>
          {tauri && (
            <div className="flex items-center justify-between py-1">
              <span className="text-[var(--color-text-muted)]">Tauri</span>
              <span className="font-mono text-[var(--color-text)]">
                {tauri}
              </span>
            </div>
          )}
        </div>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          업데이트
        </h3>
        <p className="text-[12px] text-[var(--color-text-muted)] mb-3">
          Hearth 업데이트는 Mac App Store가 자동으로 처리합니다. 직접
          확인하려면 App Store의 업데이트 페이지를 여세요.
        </p>
        <Button
          size="sm"
          variant="secondary"
          rightIcon={ExternalLink}
          onClick={() =>
            void openUrl(APP_STORE_UPDATES_URL).catch((e) =>
              console.error("openUrl failed:", e),
            )
          }
        >
          App Store에서 업데이트 확인
        </Button>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">제작</h3>
        <p className="text-[12px] text-[var(--color-text-muted)]">
          © 2026 위드지니 · 장재현
          <br />
          문의: genie@codewithgenie.com
        </p>
      </section>
    </div>
  );
}
