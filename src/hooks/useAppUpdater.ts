import { useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useToast } from "../ui/Toast";

const STARTUP_DELAY_MS = 30_000;
const PERIODIC_MS = 24 * 60 * 60 * 1000;
const DISMISS_KEY = "updater.dismissedVersion";

export function useAppUpdater(): void {
  const toast = useToast();

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

      const pending = update;
      toast.info(`새 버전 ${pending.version} 준비됨`, {
        sticky: true,
        actions: [
          {
            label: "지금 재시작",
            run: async () => {
              await pending.downloadAndInstall();
              await relaunch();
            },
          },
          {
            label: "나중에",
            run: async () => {
              localStorage.setItem(DISMISS_KEY, pending.version);
              await pending.close();
            },
          },
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
}
