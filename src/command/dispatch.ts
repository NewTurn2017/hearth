// src/command/dispatch.ts
import {
  FolderPlus,
  CalendarPlus,
  StickyNote,
  Save,
  Download,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { LocalCommand } from "./types";

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
      hint: "기존 데이터 덮어쓰기 여부 확인",
      icon: Download,
      mutation: true,
      confirmMessage: "Excel 파일을 선택하고 가져오시겠습니까?",
      run: async () => {
        const file = await open({
          filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
        });
        if (!file) return;
        const filePath = Array.isArray(file) ? file[0] : file;
        await api.importExcel(filePath, true);
      },
    },
  ];
}
