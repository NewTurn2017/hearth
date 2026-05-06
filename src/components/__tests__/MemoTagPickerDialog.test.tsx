import "@testing-library/jest-dom/vitest";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { Memo, MemoTag } from "../../types";
import { ToastProvider } from "../../ui/Toast";
import { MemoTagPickerDialog } from "../MemoTagPickerDialog";

const tag = (id: number, name: string): MemoTag => ({
  id,
  name,
  color: "#999999",
  sort_order: id,
  usage_count: 1,
  created_at: "",
  updated_at: "",
});

const memo = (overrides: Partial<Memo> = {}): Memo => ({
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

const renderDialog = ({
  onApply = vi.fn(),
  onCreateTag = vi.fn(),
  onClose = vi.fn(),
}: {
  onApply?: (tagNames: string[]) => void | Promise<void>;
  onCreateTag?: (name: string) => Promise<MemoTag>;
  onClose?: () => void;
}) => {
  render(
    <ToastProvider>
      <MemoTagPickerDialog
        open
        memo={memo()}
        tags={[tag(1, "UI")]}
        onClose={onClose}
        onApply={onApply}
        onCreateTag={onCreateTag}
      />
    </ToastProvider>,
  );
  return { onApply, onCreateTag, onClose };
};

describe("MemoTagPickerDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps the dialog open and reports apply failures", async () => {
    const onClose = vi.fn();
    const onApply = vi.fn().mockRejectedValue(new Error("db offline"));
    renderDialog({ onApply, onClose });

    fireEvent.click(screen.getByRole("button", { name: "#UI" }));
    fireEvent.click(screen.getByRole("button", { name: "적용" }));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(
        "태그 저장 실패: db offline",
      );
    });
    expect(onApply).toHaveBeenCalledWith(["UI"]);
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("keeps the dialog open and reports create failures", async () => {
    const onCreateTag = vi.fn().mockRejectedValue(new Error("duplicate"));
    renderDialog({ onCreateTag });

    fireEvent.change(screen.getByPlaceholderText("새 태그 이름"), {
      target: { value: "중요" },
    });
    fireEvent.click(screen.getByRole("button", { name: "추가" }));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(
        "태그 생성 실패: duplicate",
      );
    });
    expect(onCreateTag).toHaveBeenCalledWith("중요");
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
