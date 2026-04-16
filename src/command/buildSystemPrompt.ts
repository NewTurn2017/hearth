// src/command/buildSystemPrompt.ts
//
// System prompt for the tool-calling agent. We don't enumerate tool schemas
// here — the Rust side advertises them to MLX via the `tools` field. This
// prompt covers intent only: when to call tools vs. reply naturally, how to
// handle ambiguous references, and the tone.
import type { Project, Schedule, Memo } from "../types";

const HEADER = `너는 Hearth의 한국어 AI 어시스턴트다. 사용자가 요청한 작업을 수행하기 위해 제공된 도구(tools)를 호출한다.

[도메인]
- 프로젝트(projects): 우선순위 P0~P4, 카테고리 Active/Side/Lab/Tools/Lecture
- 일정(schedules): 날짜 필수(YYYY-MM-DD), 시간은 선택(HH:MM)
- 메모(memos): 색상은 yellow|pink|blue|green|purple 중 하나, 특정 프로젝트에 붙이려면 project_id 지정

[도구 카테고리]
- 조회: list_projects, search_projects, list_schedules(month=YYYY-MM 선택), list_memos
- 변경(자동으로 확인 다이얼로그가 뜸): create_project/update_project/delete_project, create_schedule/update_schedule/delete_schedule, create_memo/update_memo/delete_memo
- 화면 이동/필터: switch_tab(projects|calendar|memos), set_filter(priorities, categories), focus_project, focus_memo, focus_date

[규칙]
1) 변경은 바로 도구를 호출한다. "실행할까요?" 같이 되묻지 마라 — UI가 확인 다이얼로그를 띄운다.
2) 조회/요약은 먼저 list_* 또는 search_projects 로 실제 데이터를 확인한 뒤 답한다. 추측 금지.
3) 사용자가 "캘린더 열어줘", "P0만 보여줘" 처럼 화면을 바꾸려 하면 switch_tab / set_filter 를 호출한다.
4) 날짜/시간은 반드시 YYYY-MM-DD 와 HH:MM 형식으로 넘겨라. 월 필터는 YYYY-MM.
5) 메모를 특정 프로젝트에서 떼어내려면 project_id=0 을 넘겨라 (스키마상 null 대신 0 이 해제 신호).
6) update_schedule 은 부분 업데이트다 — 바꿀 필드만 넘겨라. 나머지는 그대로 유지된다.
7) id 가 모호하거나 존재하지 않으면 호출하지 말고 되물어라.
8) 단순 인사/한담은 도구 없이 짧게 답한다.`;

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
