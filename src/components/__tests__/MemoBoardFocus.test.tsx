import "@testing-library/jest-dom/vitest";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { CategoryRow, Memo, MemoTag, Project } from "../../types";
import { MemoBoard } from "../MemoBoard";
import { ToastProvider } from "../../ui/Toast";

const update = vi.fn();
const remove = vi.fn();
const reload = vi.fn();
const createTag = vi.fn();

let mockMemos: Memo[] = [];
let mockProjects: Project[] = [];
let mockCategories: CategoryRow[] = [];
let mockTags: MemoTag[] = [];

vi.mock("../../hooks/useMemos", () => ({
  useMemos: () => ({
    memos: mockMemos,
    loading: false,
    update,
    remove,
    reload,
  }),
}));

vi.mock("../../hooks/useProjects", () => ({
  useProjects: () => ({ projects: mockProjects, loading: false }),
}));

vi.mock("../../hooks/useCategories", () => ({
  useCategories: () => ({ categories: mockCategories, loading: false }),
}));

vi.mock("../../hooks/useMemoTags", () => ({
  useMemoTags: () => ({
    tags: mockTags,
    loading: false,
    create: createTag,
  }),
}));

vi.mock("../../api", () => ({
  updateMemo: vi.fn(),
  reorderMemos: vi.fn(),
}));

const tag = (id: number, name: string): MemoTag => ({
  id,
  name,
  color: "#999999",
  sort_order: id,
  usage_count: 1,
  created_at: "",
  updated_at: "",
});

const project = (overrides: Partial<Project>): Project => ({
  id: 1,
  priority: "P2",
  number: null,
  name: "Project",
  category: null,
  path: null,
  evaluation: null,
  sort_order: 0,
  created_at: "",
  updated_at: "",
  ...overrides,
});

const category = (id: number, name: string): CategoryRow => ({
  id,
  name,
  color: "#999999",
  sort_order: id,
  usage_count: 1,
  created_at: "",
  updated_at: "",
});

const memo = (overrides: Partial<Memo>): Memo => ({
  id: 1,
  content: "memo",
  color: "yellow",
  project_id: null,
  sort_order: 0,
  font_size: "normal",
  is_bold: false,
  focus_x: null,
  focus_y: null,
  tags: [],
  created_at: "",
  updated_at: "",
  ...overrides,
});

const renderBoard = () =>
  render(
    <ToastProvider>
      <MemoBoard />
    </ToastProvider>,
  );

describe("MemoBoard Focus view", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    mockTags = [tag(1, "중요"), tag(2, "UI"), tag(3, "Ops")];
    mockCategories = [category(1, "Tools"), category(2, "Lab")];
    mockProjects = [
      project({ id: 10, name: "Tool Project", category: "Tools" }),
      project({ id: 11, name: "Lab Project", category: "Lab" }),
    ];
    mockMemos = [
      memo({ id: 1, content: "plain memo", sort_order: 0 }),
      memo({ id: 2, content: "bold memo", sort_order: 1, is_bold: true }),
      memo({ id: 3, content: "large memo", sort_order: 2, font_size: "large" }),
      memo({
        id: 4,
        content: "important tag memo",
        sort_order: 3,
        tags: [mockTags[0]],
      }),
      memo({
        id: 5,
        content: "tools ui memo",
        sort_order: 4,
        project_id: 10,
        tags: [mockTags[1]],
      }),
      memo({
        id: 6,
        content: "lab ui memo",
        sort_order: 5,
        project_id: 11,
        tags: [mockTags[1]],
      }),
    ];
  });

  it("persists and restores the Focus tab", () => {
    const { unmount } = renderBoard();
    fireEvent.click(screen.getByRole("tab", { name: /포커스/ }));

    expect(localStorage.getItem("hearth.memoboard.view")).toBe("focus");
    expect(screen.getByTestId("focus-board-surface")).toBeInTheDocument();

    unmount();
    renderBoard();

    expect(screen.getByRole("tab", { name: /포커스/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByTestId("focus-board-surface")).toBeInTheDocument();
  });

  it("filters Focus notes to important memos", () => {
    renderBoard();
    fireEvent.click(screen.getByRole("tab", { name: /포커스/ }));
    fireEvent.click(screen.getAllByRole("button", { name: "중요" })[0]);

    expect(screen.queryByText("plain memo")).not.toBeInTheDocument();
    expect(screen.getByText("bold memo")).toBeInTheDocument();
    expect(screen.getByText("large memo")).toBeInTheDocument();
    expect(screen.getByText("important tag memo")).toBeInTheDocument();
  });

  it("combines category and tag rail filters", () => {
    renderBoard();
    fireEvent.click(screen.getByRole("tab", { name: /포커스/ }));
    fireEvent.click(screen.getByRole("button", { name: "Tools" }));
    fireEvent.click(screen.getByRole("button", { name: "#UI" }));

    expect(screen.getByText("tools ui memo")).toBeInTheDocument();
    expect(screen.queryByText("lab ui memo")).not.toBeInTheDocument();
    expect(screen.queryByText("plain memo")).not.toBeInTheDocument();
  });

  it("renders large bold Focus notes with emphasis classes", () => {
    mockMemos = [
      memo({
        id: 7,
        content: "styled memo",
        is_bold: true,
        font_size: "large",
      }),
    ];
    renderBoard();
    fireEvent.click(screen.getByRole("tab", { name: /포커스/ }));

    expect(screen.getByText("styled memo")).toHaveClass("font-semibold");
    expect(screen.getByText("styled memo")).toHaveClass(
      "text-[16px] leading-relaxed",
    );
  });
});
