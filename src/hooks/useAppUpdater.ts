import { useEffect, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useToast } from "../ui/Toast";

const STARTUP_DELAY_MS = 30_000;
const PERIODIC_MS = 24 * 60 * 60 * 1000;
const DISMISS_KEY = "updater.dismissedVersion";

export interface PendingUpdate {
  version: string;
  install: () => Promise<void>;
  dismiss: () => Promise<void>;
}

/**
 * Background updater. Shows a sticky toast whenever a new version is available
 * and also exposes the pending update so the shell can render a persistent
 * UI affordance (e.g. an "업데이트" button in the TopBar). The toast fires once
 * per availability; the button stays visible until the user dismisses or
 * installs, so they can trigger the upgrade even after closing the toast.
 */
export function useAppUpdater(): PendingUpdate | null {
  const toast = useToast();
  const [pending, setPending] = useState<PendingUpdate | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function runCheck() {
      if (cancelled) return;
      let update: Awaited<ReturnType<typeof check>> = null;
      try {
        update = await check();
      } catch {
        return;
      }
      // `update.available` is @deprecated in @tauri-apps/plugin-updater v2 —
      // check() returns null when there's nothing new.
      if (cancelled || update === null) return;
      if (localStorage.getItem(DISMISS_KEY) === update.version) {
        // Release the Rust-side Resource so we don't leak handles on each tick.
        await update.close();
        return;
      }

      const ref = update;
      const install = async () => {
        await ref.downloadAndInstall();
        await relaunch();
      };
      const dismiss = async () => {
        localStorage.setItem(DISMISS_KEY, ref.version);
        await ref.close();
        if (!cancelled) setPending(null);
      };

      if (!cancelled) setPending({ version: ref.version, install, dismiss });

      toast.info(`새 버전 ${ref.version} 준비됨`, {
        sticky: true,
        actions: [
          { label: "지금 재시작", run: install },
          { label: "나중에", run: dismiss },
        ],
      });
    }

    const startTimer = setTimeout(runCheck, STARTUP_DELAY_MS);
    const intervalTimer = setInterval(runCheck, PERIODIC_MS);

    return () => {
      cancelled = true;
      clearTimeout(startTimer);
      clearInterval(intervalTimer);
    };
    // toast is stable from context; react-hooks lint will accept [toast]
  }, [toast]);

  return pending;
}
