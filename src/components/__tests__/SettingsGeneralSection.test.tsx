import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsGeneralSection } from "../SettingsGeneralSection";

vi.mock("../../api", () => ({
  notificationsPermission: vi.fn().mockResolvedValue("unknown"),
  notificationsRequest: vi.fn().mockResolvedValue("granted"),
  getQuickCaptureShortcut: vi
    .fn()
    .mockResolvedValue("CommandOrControl+Shift+H"),
  getQuickCaptureShortcutError: vi.fn().mockResolvedValue(""),
  rebindQuickCaptureShortcut: vi
    .fn()
    .mockResolvedValue("CommandOrControl+Shift+H"),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../../ui/Toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  }),
}));

import * as api from "../../api";

describe("SettingsGeneralSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the autostart 1.1 informational card instead of a toggle", async () => {
    render(<SettingsGeneralSection active />);
    expect(
      await screen.findByText(/Mac App Store 정책 호환을 위해 1\.1에서/),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("로그인 시 Hearth 자동 실행"),
    ).not.toBeInTheDocument();
  });

  it("fires notifications_request when the permission button is clicked", async () => {
    render(<SettingsGeneralSection active />);
    const btn = await screen.findByRole("button", { name: "권한 요청" });
    fireEvent.click(btn);
    await waitFor(() => expect(api.notificationsRequest).toHaveBeenCalled());
  });
});
