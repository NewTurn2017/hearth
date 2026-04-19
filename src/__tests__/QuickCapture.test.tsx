import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("../api", () => ({
  createMemo: vi.fn().mockResolvedValue({ id: 42 }),
  hideQuickCaptureWindow: vi.fn().mockResolvedValue(undefined),
  resizeQuickCaptureWindow: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { QuickCapture } from "../windows/QuickCapture";
import * as api from "../api";

describe("QuickCapture", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("saves on Enter and hides", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "hello world{Enter}");
    expect(api.createMemo).toHaveBeenCalledWith({
      content: "hello world",
      color: "yellow",
    });
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("Esc closes without save", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "abc");
    await userEvent.keyboard("{Escape}");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("empty Enter is no-op save but hides", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "   {Enter}");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("Shift+Enter inserts newline and does not submit", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox") as HTMLTextAreaElement;
    await userEvent.type(input, "line1{Shift>}{Enter}{/Shift}line2");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(input.value).toBe("line1\nline2");
  });
});
