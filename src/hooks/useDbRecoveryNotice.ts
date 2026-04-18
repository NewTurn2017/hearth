import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useToast } from "../ui/Toast";

export function useDbRecoveryNotice(): void {
  const toast = useToast();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    (async () => {
      const off = await listen<string>("db:recovered", () => {
        if (cancelled) return;
        toast.info(
          "데이터베이스 파일이 손상되어 빈 상태로 복구되었습니다. Settings → 백업 → 복원에서 최근 백업으로 되돌릴 수 있습니다.",
          { sticky: true }
        );
      });
      if (cancelled) {
        off();
      } else {
        unlisten = off;
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [toast]);
}
