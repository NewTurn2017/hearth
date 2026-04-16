// src/command/buildSystemPrompt.ts
import type { Project, Schedule, Memo } from "../types";

const HEADER = `너는 Project Genie 의 AI 어시스턴트다. 한국어로 답한다.
사용자 요청에 JSON 으로 응답한다. "reply" 는 자연어, "actions" 는 수행할 액션 배열 (없으면 빈 배열).

사용 가능한 command:
  create_project(name, priority, category?, path?)
  update_project(id, fields)
  delete_project(id)
  create_schedule(date, time?, location?, description?, notes?)
  update_schedule(id, fields)
  delete_schedule(id)
  create_memo(content, color?, project_id?)
  update_memo(id, fields)
  delete_memo(id)
  set_filter(priorities?, categories?)
  focus_project(id)

규칙:
1) 생성/수정/삭제 command (create_*, update_*, delete_*) 는 모두 type: mutation — 실행은 사용자가 UI 에서 확인한다.
2) set_filter, focus_project 는 type: navigation — 확인 없이 즉시 실행.
3) 단순 조회/요약은 reply 에만 서술, actions 는 빈 배열.
4) 존재하지 않는 프로젝트/일정/메모 는 추측하지 않고 사용자에게 되물어본다.`;

export function buildSystemPrompt(snapshot: {
  projects: Project[];
  schedules: Schedule[];
  memos: Memo[];
}): string {
  const { projects, schedules, memos } = snapshot;
  const byPri = (p: string) => projects.filter((pr) => pr.priority === p).length;
  const stats = `현재 상태:
- 프로젝트 ${projects.length}개 (P0 ${byPri("P0")}, P1 ${byPri("P1")}, P2 ${byPri("P2")}, P3 ${byPri("P3")}, P4 ${byPri("P4")})
- 일정 ${schedules.length}개
- 메모 ${memos.length}개`;

  const projectList =
    "[프로젝트 목록]\n" +
    projects
      .slice(0, 50)
      .map((p) => `#${p.id} [${p.priority}] ${p.name}${p.category ? ` (${p.category})` : ""}`)
      .join("\n");

  const scheduleList =
    "[이번 달 일정]\n" +
    schedules
      .slice(0, 30)
      .map((s) => `#${s.id} ${s.date}${s.time ? ` ${s.time}` : ""} ${s.description ?? ""}${s.location ? ` @ ${s.location}` : ""}`)
      .join("\n");

  const memoList =
    "[최근 메모 10개]\n" +
    memos
      .slice(0, 10)
      .map((m) => `#${m.id} ${m.content.slice(0, 80)}`)
      .join("\n");

  return [HEADER, stats, projectList, scheduleList, memoList].join("\n\n");
}
