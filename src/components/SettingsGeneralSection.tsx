import { useEffect, useState } from "react";
import { Button } from "../ui/Button";
import * as api from "../api";
import type { NotificationPermission } from "../api";
import { useQuickCaptureShortcut } from "../hooks/useQuickCaptureShortcut";
import { ShortcutRecorder } from "./settings/ShortcutRecorder";

export function SettingsGeneralSection({
  active,
}: {
  active: boolean;
}) {
  const [perm, setPerm] = useState<NotificationPermission>("unknown");
  const [busy, setBusy] = useState(false);
  const {
    combo,
    display,
    error: shortcutError,
    rebind,
  } = useQuickCaptureShortcut();
  const [recording, setRecording] = useState(false);
  const [rebindError, setRebindError] = useState<string | null>(null);

  async function refresh() {
    try {
      const p = await api.notificationsPermission();
      setPerm(p);
    } catch (e) {
      console.error("general settings load failed:", e);
    }
  }

  useEffect(() => {
    if (active) refresh();
  }, [active]);

  async function requestPerm() {
    setBusy(true);
    try {
      const p = await api.notificationsRequest();
      setPerm(p);
    } catch (e) {
      console.error("notifications_request failed:", e);
    } finally {
      setBusy(false);
    }
  }

  const permLabel = {
    granted: "허용됨",
    denied: "차단됨",
    unknown: "미요청",
  }[perm];

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          로그인 시 자동 실행
        </h3>
        <div className="rounded-md border border-[var(--color-border)] p-3 text-[12px] text-[var(--color-text-muted)] leading-relaxed">
          Mac App Store 정책 호환을 위해 1.1에서 지원될 예정입니다.
          <br />
          그동안은 macOS 시스템 설정 → 일반 → 로그인 항목에 Hearth를 추가해
          주세요.
        </div>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">알림</h3>
        <div className="flex items-center gap-3 text-[13px]">
          <span>상태: {permLabel}</span>
          {perm !== "granted" && (
            <Button
              size="sm"
              variant="secondary"
              onClick={requestPerm}
              disabled={busy}
            >
              권한 요청
            </Button>
          )}
        </div>
        {perm === "denied" && (
          <p className="text-[11px] text-[var(--color-text-muted)] mt-2">
            macOS 시스템 설정 → 알림 → Hearth 에서 허용으로 변경해 주세요.
          </p>
        )}
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          Quick Capture
        </h3>
        <div className="flex items-center gap-3 text-[13px]">
          <span className="font-mono rounded bg-black/40 px-2 py-1">
            {display || "—"}
          </span>
          {!recording && (
            <Button size="sm" onClick={() => setRecording(true)}>
              변경
            </Button>
          )}
        </div>
        {recording && (
          <div className="mt-3">
            <ShortcutRecorder
              onCancel={() => {
                setRecording(false);
                setRebindError(null);
              }}
              onSave={async (next) => {
                try {
                  await rebind(next);
                  setRecording(false);
                  setRebindError(null);
                } catch (e) {
                  setRebindError(String(e));
                }
              }}
            />
          </div>
        )}
        {(shortcutError || rebindError) && (
          <p className="mt-2 text-xs text-red-400">
            {rebindError ?? `단축키 등록 실패: ${shortcutError}`}
          </p>
        )}
        <p className="mt-2 text-[11px] text-[var(--color-text-muted)]">
          어느 앱에서든 이 단축키로 한 줄 메모를 남길 수 있어요. Hearth가 완전히
          종료되면 작동하지 않습니다. 저장된 메모는 기본 노란색으로 메모 탭
          상단에 쌓입니다. (combo: <code>{combo}</code>)
        </p>
      </section>
    </div>
  );
}
