import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Button } from "../ui/Button";
import * as api from "../api";

type Stage = "intro" | "choosing" | "restartPrompt" | "error";

// First-launch blocking modal driven by the Rust `migration:required`
// event (emitted from lib.rs::setup() when no bookmark exists and the user
// has not explicitly dismissed). Settings → 일반 → "데이터 폴더" handles the
// re-entry flow inline; this component is for the boot-time prompt only.
export function MigrationWizard() {
  const [open, setOpen] = useState(false);
  const [stage, setStage] = useState<Stage>("intro");
  const [error, setError] = useState<string | null>(null);
  const [resolvedPath, setResolvedPath] = useState<string | null>(null);
  // Guard against React StrictMode's double-effect — `cancelled` flips so
  // the late-resolving listen() promise unsubscribes itself.
  const onceRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    function reveal() {
      if (cancelled) return;
      if (onceRef.current) return;
      onceRef.current = true;
      setOpen(true);
      setStage("intro");
    }

    (async () => {
      // Belt-and-suspenders: if the listener registers after Rust emits
      // `migration:required` (the 1500ms delay in setup() vs. React mount
      // timing), we still need to surface the wizard. Query the status
      // explicitly and reveal if the user has neither a bookmark nor an
      // explicit dismiss marker.
      try {
        const status = await api.getDataFolderStatus();
        if (!status.hasBookmark && !status.dismissed) {
          reveal();
        }
      } catch {
        // Non-Tauri context (tests) — ignore.
      }

      const off = await listen("migration:required", reveal);
      if (cancelled) off();
      else unlisten = off;
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  if (!open) return null;

  async function onChoose() {
    setStage("choosing");
    setError(null);
    try {
      const { resolvedPath } = await api.chooseDataFolder();
      setResolvedPath(resolvedPath);
      setStage("restartPrompt");
    } catch (e) {
      const msg = String(e);
      // NSOpenPanel cancel → return to intro silently. Real errors surface.
      if (msg.includes("user_cancelled")) {
        setStage("intro");
      } else {
        setError(msg);
        setStage("error");
      }
    }
  }

  async function onDismiss() {
    try {
      await api.dismissMigration();
    } catch (e) {
      console.error("dismiss_migration failed:", e);
    }
    setOpen(false);
  }

  async function onRestart() {
    try {
      await api.restartApp();
    } catch (e) {
      console.error("restart_app failed:", e);
      setError(String(e));
      setStage("error");
    }
  }

  return (
    // Note: no backdrop-dismiss — the user must click one of the buttons.
    // This is intentional for the first-launch flow. Settings re-entry uses
    // a different inline path that the user can ignore.
    <div className="fixed inset-0 z-[200] flex items-center justify-center p-6 bg-black/60">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="migration-wizard-title"
        className="w-full max-w-lg rounded-[var(--radius-xl)] bg-[var(--color-surface-1)] border border-[var(--color-border)] shadow-[var(--shadow-e3)] p-6 outline-none"
      >
        <h2
          id="migration-wizard-title"
          className="text-[16px] font-semibold text-[var(--color-text-hi)] mb-3"
        >
          데이터 폴더 연결
        </h2>

        {stage === "intro" && (
          <>
            <p className="text-[13px] text-[var(--color-text)] leading-relaxed mb-2">
              Hearth는 <code>~/Library/Application Support/com.codewithgenie.hearth/</code>
              에 데이터를 보관합니다. 이 위치 접근 권한을 한 번 허용해 주세요.
            </p>
            <p className="text-[12px] text-[var(--color-text-muted)] leading-relaxed mb-5">
              CLI 및 AI agent와 같은 데이터를 공유하기 위함입니다. "나중에"를
              선택하면 기능 축소 모드로 동작하며 설정에서 언제든 다시 연결할
              수 있습니다.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={onDismiss}>
                나중에
              </Button>
              <Button onClick={onChoose}>폴더 선택</Button>
            </div>
          </>
        )}

        {stage === "choosing" && (
          <p className="text-[13px] text-[var(--color-text-muted)]">
            폴더 선택 창을 여는 중...
          </p>
        )}

        {stage === "restartPrompt" && (
          <>
            <p className="text-[13px] text-[var(--color-text)] leading-relaxed mb-2">
              데이터 폴더가 연결되었습니다.
            </p>
            {resolvedPath && (
              <p className="text-[11px] text-[var(--color-text-muted)] font-mono break-all mb-4">
                {resolvedPath}
              </p>
            )}
            <p className="text-[12px] text-[var(--color-text-muted)] leading-relaxed mb-5">
              새 위치를 적용하려면 Hearth를 재시작해 주세요.
            </p>
            <div className="flex justify-end gap-2">
              <Button onClick={onRestart}>지금 재시작</Button>
            </div>
          </>
        )}

        {stage === "error" && (
          <>
            <p className="text-[13px] text-red-400 leading-relaxed mb-2">
              오류가 발생했습니다.
            </p>
            <p className="text-[11px] text-[var(--color-text-muted)] font-mono break-all mb-5">
              {error}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setStage("intro")}>
                다시 시도
              </Button>
              <Button variant="secondary" onClick={onDismiss}>
                닫기
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
