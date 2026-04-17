import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { NewMemoDialog } from "../NewMemoDialog";
import { ToastProvider } from "../../ui/Toast";

vi.mock("../../api", () => ({
  createMemo: vi.fn().mockResolvedValue({
    id: 1,
    content: "hello",
    color: "pink",
    project_id: null,
    sort_order: 0,
    created_at: "",
    updated_at: "",
  }),
}));

vi.mock("../../hooks/useProjects", () => ({
  useProjects: () => ({ projects: [], loading: false }),
}));

import * as api from "../../api";

const renderIt = (open = true, onClose = () => {}) =>
  render(
    <ToastProvider>
      <NewMemoDialog open={open} onClose={onClose} />
    </ToastProvider>
  );

describe("NewMemoDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("disables the 추가 button when content is empty", () => {
    renderIt();
    const submit = screen.getByRole("button", { name: "추가" });
    expect(submit).toBeDisabled();
  });

  it("calls createMemo with content + chosen color on submit", async () => {
    renderIt();
    const textarea = screen.getByRole("textbox");
    fireEvent.change(textarea, { target: { value: "hello" } });
    const pinkBtn = screen.getByLabelText("색상: pink");
    fireEvent.click(pinkBtn);
    const submit = screen.getByRole("button", { name: "추가" });
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    await waitFor(() => {
      expect(api.createMemo).toHaveBeenCalledWith({
        content: "hello",
        color: "pink",
        project_id: undefined,
      });
    });
  });
});
