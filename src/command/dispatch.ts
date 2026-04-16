// src/command/dispatch.ts
import {
  FolderPlus,
  CalendarPlus,
  StickyNote,
  Save,
  Download,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir, join } from "@tauri-apps/api/path";
import * as api from "../api";
import type { LocalCommand } from "./types";

const EXCEL_FORMAT_HINT =
  "Excel 파일을 선택하면 기존 프로젝트/메모/일정/고객사 데이터를 모두 삭제하고 새로 가져옵니다.\n\n" +
  "예상 형식:\n" +
  "• 시트 이름: 'Projects'\n" +
  "• A열: 우선순위 (P0 / P1 / P2 / P3 / P4)\n" +
  "• B열: 번호 (정수, 선택)\n" +
  "• C열: 프로젝트명 (필수)\n" +
  "• D열: 카테고리 (Active / Side / Lab / Tools / Lecture)\n" +
  "• E열: 경로 (선택)\n" +
  "• F열: 평가/메모 (선택)\n\n" +
  "시드 예시: ~/dev/projects.xlsx";

async function defaultSeedPath(): Promise<string | undefined> {
  try {
    const home = await homeDir();
    return await join(home, "dev", "projects.xlsx");
  } catch {
    return undefined;
  }
}

export interface DispatchDeps {
  openNewProject: () => void;
  openNewSchedule: () => void;
  openNewMemo: () => void;
}

export function buildLocalCommands(deps: DispatchDeps): LocalCommand[] {
  return [
    {
      id: "new-project",
      label: "새 프로젝트",
      hint: "프로젝트 추가",
      icon: FolderPlus,
      run: async () => {
        deps.openNewProject();
      },
    },
    {
      id: "new-schedule",
      label: "새 일정",
      hint: "일정 추가",
      icon: CalendarPlus,
      run: async () => {
        deps.openNewSchedule();
      },
    },
    {
      id: "new-memo",
      label: "새 메모",
      hint: "메모 추가",
      icon: StickyNote,
      run: async () => {
        deps.openNewMemo();
      },
    },
    {
      id: "backup",
      label: "백업 생성",
      hint: "DB 스냅샷 저장",
      icon: Save,
      mutation: true,
      confirmMessage: "현재 DB를 백업하시겠습니까?",
      run: async () => {
        await api.backupDb();
      },
    },
    {
      id: "import-excel",
      label: "Excel 가져오기",
      hint: "Projects 시트, A=우선순위 / C=이름 / E=경로",
      icon: Download,
      mutation: true,
      confirmMessage: EXCEL_FORMAT_HINT,
      run: async () => {
        const file = await open({
          defaultPath: await defaultSeedPath(),
          filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
        });
        if (!file) return;
        const filePath = Array.isArray(file) ? file[0] : file;
        await api.importExcel(filePath, true);
        window.dispatchEvent(new CustomEvent("projects:changed"));
        // Full reload so schedules/memos/clients repopulate together.
        setTimeout(() => window.location.reload(), 600);
      },
    },
  ];
}
