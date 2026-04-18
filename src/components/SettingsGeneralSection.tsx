import { useEffect, useState } from "react";
import { Button } from "../ui/Button";
import * as api from "../api";
import type { NotificationPermission } from "../api";

export function SettingsGeneralSection({ active }: { active: boolean }) {
  const [autostart, setAutostartState] = useState<boolean>(false);
  const [perm, setPerm] = useState<NotificationPermission>("unknown");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    try {
      const [a, p] = await Promise.all([
        api.getAutostart(),
        api.notificationsPermission(),
      ]);
      setAutostartState(a);
      setPerm(p);
    } catch (e) {
      console.error("general settings load failed:", e);
    }
  }

  useEffect(() => {
    if (active) refresh();
  }, [active]);

  async function toggleAutostart(next: boolean) {
    setBusy(true);
    try {
      await api.setAutostart(next);
      setAutostartState(next);
    } catch (e) {
      console.error("set_autostart failed:", e);
    } finally {
      setBusy(false);
    }
  }

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
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">자동 시작</h3>
        <label className="flex items-center gap-2 text-[13px]">
          <input
            type="checkbox"
            checked={autostart}
            disabled={busy}
            onChange={(e) => toggleAutostart(e.target.checked)}
            aria-label="로그인 시 Hearth 자동 실행"
          />
          <span>로그인 시 Hearth 자동 실행 (백그라운드에서 조용히 시작)</span>
        </label>
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
    </div>
  );
}
