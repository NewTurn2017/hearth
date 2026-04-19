// Backup location + manual backup + restore list.
//
// Backup directory is stored under the `backup.dir` key in the settings KV.
// An empty value falls back to `$APP_DATA/backups` — the behavior pre-dating
// this feature. Restore is gated behind `ask(...)` because it overwrites
// `data.db` and the app only fully settles after the next launch.

import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Download,
  FolderCog,
  FolderOpen,
  RotateCcw,
  Save,
  Trash2,
} from "lucide-react";
import { ask, open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import type { BackupInfo } from "../types";
import * as api from "../api";

export function SettingsBackupSection({ active }: { active: boolean }) {
  const toast = useToast();
  const [dir, setDir] = useState<string>("");
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const [d, list] = await Promise.all([
        api.getBackupDir(),
        api.listBackups(),
      ]);
      setDir(d);
      setBackups(list);
    } catch (e) {
      toast.error(`백업 정보 불러오기 실패: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (active) void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  // Refetch whenever any code path dispatches `backup:changed` (including our
  // own mutation paths below). Consolidates what used to be three inline
  // `refresh()` calls.
  useEffect(() => {
    const onChanged = () => {
      void refresh();
    };
    window.addEventListener("backup:changed", onChanged);
    return () => window.removeEventListener("backup:changed", onChanged);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handlePickDir = async () => {
    const picked = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: dir || undefined,
    });
    if (!picked) return;
    const next = Array.isArray(picked) ? picked[0] : picked;
    setBusy(true);
    try {
      const canonical = await api.setBackupDir(next);
      setDir(canonical);
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success("백업 위치 변경됨");
    } catch (e) {
      toast.error(`변경 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleRevealDir = async () => {
    if (!dir) return;
    try {
      await api.openInFinder(dir);
    } catch (e) {
      toast.error(`폴더 열기 실패: ${e}`);
    }
  };

  const handleImport = async () => {
    const file = await openDialog({
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
      defaultPath: dir || undefined,
    });
    if (!file) return;
    const clearExisting = await ask(
      "기존 데이터를 삭제하고 새로 가져오시겠습니까?",
      { title: "Excel 가져오기", kind: "warning" }
    );
    setBusy(true);
    try {
      const filePath = Array.isArray(file) ? file[0] : file;
      const result = await api.importExcel(filePath, clearExisting);
      toast.success(`${result.projects_imported}개 프로젝트 가져왔습니다`);
      setTimeout(() => window.location.reload(), 800);
    } catch (e) {
      toast.error(`가져오기 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleBackupNow = async () => {
    setBusy(true);
    try {
      const path = await api.backupDb();
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success(`백업 완료: ${path}`);
    } catch (e) {
      toast.error(`백업 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleRestore = async (info: BackupInfo) => {
    const ok = await ask(
      `${info.filename} 을(를) 복원하시겠습니까? 현재 DB가 덮어쓰기됩니다.`,
      { title: "백업 복원", kind: "warning" }
    );
    if (!ok) return;
    setBusy(true);
    try {
      await api.restoreDb(info.path);
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success("복원 완료 — 앱을 다시 시작하세요");
    } catch (e) {
      toast.error(`복원 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleReset = async () => {
    const ok = await ask(
      "모든 프로젝트 · 메모 · 일정 · 클라이언트가 삭제됩니다.\n" +
        "카테고리 · AI 설정 · 백업 경로 · UI 스케일은 유지됩니다.\n\n" +
        "초기화 직전 스냅샷이 백업 폴더에 저장되므로 '최근 백업' 에서 복원할 수 있습니다.\n\n" +
        "계속하시겠습니까?",
      { title: "데이터 초기화", kind: "warning" }
    );
    if (!ok) return;
    setBusy(true);
    try {
      const snapshot = await api.resetData();
      window.dispatchEvent(new CustomEvent("backup:changed"));
      window.dispatchEvent(new CustomEvent("projects:changed"));
      window.dispatchEvent(new CustomEvent("memos:changed"));
      window.dispatchEvent(new CustomEvent("schedules:changed"));
      toast.success(`초기화 완료 — 스냅샷: ${snapshot}`);
    } catch (e) {
      toast.error(`초기화 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <div>
        <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
          백업 위치
        </label>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleRevealDir}
            disabled={busy || !dir}
            title={dir ? `Finder에서 열기 — ${dir}` : dir}
            className={cn(
              "flex-1 h-9 px-3 inline-flex items-center text-[12px] font-mono text-left",
              "rounded-[var(--radius-md)] bg-[var(--color-surface-2)]",
              "border border-[var(--color-border)] text-[var(--color-text)] truncate",
              "hover:border-[var(--color-brand-hi)] hover:text-[var(--color-brand-hi)]",
              "disabled:cursor-not-allowed disabled:hover:border-[var(--color-border)]",
              "disabled:hover:text-[var(--color-text)]"
            )}
          >
            {loading ? "불러오는 중…" : dir || "(설정되지 않음)"}
          </button>
          <Button
            variant="ghost"
            size="sm"
            leftIcon={FolderOpen}
            onClick={handleRevealDir}
            disabled={busy || !dir}
          >
            열기
          </Button>
          <Button
            variant="secondary"
            size="sm"
            leftIcon={FolderCog}
            onClick={handlePickDir}
            disabled={busy}
          >
            변경…
          </Button>
        </div>
      </div>

      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-[12px] font-medium text-[var(--color-text)]">
            최근 백업
          </label>
          <Button
            variant="primary"
            size="sm"
            leftIcon={Save}
            onClick={handleBackupNow}
            disabled={busy}
          >
            지금 백업
          </Button>
        </div>
        {backups.length === 0 ? (
          <p className="text-[12px] text-[var(--color-text-dim)]">
            아직 백업이 없습니다
          </p>
        ) : (
          <ul className="flex flex-col divide-y divide-[var(--color-border)] rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-surface-2)]">
            {backups.slice(0, 5).map((b) => (
              <li key={b.path} className="flex items-center gap-2 px-3 h-9">
                <span
                  className="flex-1 truncate text-[12px] font-mono text-[var(--color-text)]"
                  title={b.path}
                >
                  {b.filename}
                </span>
                <span className="text-[11px] text-[var(--color-text-dim)] shrink-0">
                  {b.created}
                </span>
                <button
                  type="button"
                  onClick={() => handleRestore(b)}
                  disabled={busy}
                  className={cn(
                    "inline-flex items-center gap-1 h-7 px-2 rounded-[var(--radius-sm)]",
                    "text-[11px] text-[var(--color-text-muted)]",
                    "hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-3)]",
                    "disabled:opacity-50 disabled:cursor-not-allowed"
                  )}
                >
                  <Icon icon={RotateCcw} size={14} />
                  복원
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-[12px] font-medium text-[var(--color-text)]">
            Excel 가져오기
          </label>
          <Button
            variant="secondary"
            size="sm"
            leftIcon={Download}
            onClick={handleImport}
            disabled={busy}
          >
            파일 선택…
          </Button>
        </div>
        <p className="text-[11px] text-[var(--color-text-dim)] leading-relaxed">
          백업 위치에서 .xlsx/.xls 파일을 선택해 프로젝트를 가져옵니다. 기존 데이터를 유지하거나 초기화한 뒤 가져올지 선택할 수 있어요.
        </p>
      </div>

      <div
        className={cn(
          "rounded-[var(--radius-md)] border border-[var(--color-danger)]/60",
          "bg-[var(--color-danger)]/5 p-3 flex flex-col gap-2"
        )}
      >
        <div className="flex items-center gap-1.5 text-[var(--color-danger)]">
          <Icon icon={AlertTriangle} size={14} />
          <span className="text-[12px] font-medium">위험 구역</span>
        </div>
        <p className="text-[11px] text-[var(--color-text-dim)] leading-relaxed">
          프로젝트 · 메모 · 일정 · 클라이언트를 모두 삭제합니다. 카테고리 · AI 설정 · 백업 경로는 유지돼요. 초기화 직전 스냅샷이 <span className="font-mono">pre-reset-…</span> 이름으로 백업 폴더에 저장되므로 '최근 백업' 에서 되돌릴 수 있습니다.
        </p>
        <div className="flex justify-end">
          <Button
            variant="secondary"
            size="sm"
            leftIcon={Trash2}
            onClick={handleReset}
            disabled={busy}
            className="!text-[var(--color-danger)] !border-[var(--color-danger)]/40 hover:!bg-[var(--color-danger)]/10"
          >
            데이터 초기화
          </Button>
        </div>
      </div>
    </div>
  );
}
