import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsGeneralSection } from "../SettingsGeneralSection";

vi.mock("../../api", () => ({
  getAutostart: vi.fn().mockResolvedValue(false),
  setAutostart: vi.fn().mockResolvedValue(undefined),
  notificationsPermission: vi.fn().mockResolvedValue("unknown"),
  notificationsRequest: vi.fn().mockResolvedValue("granted"),
  getQuickCaptureShortcut: vi.fn().mockResolvedValue("CommandOrControl+Shift+H"),
  getQuickCaptureShortcutError: vi.fn().mockResolvedValue(""),
  rebindQuickCaptureShortcut: vi.fn().mockResolvedValue("CommandOrControl+Shift+H"),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import * as api from "../../api";

describe("SettingsGeneralSection", () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it("reads initial autostart state and toggles via set_autostart", async () => {
    render(<SettingsGeneralSection active />);
    const toggle = await screen.findByLabelText("로그인 시 Hearth 자동 실행");
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(api.setAutostart).toHaveBeenCalledWith(true)
    );
  });

  it("fires notifications_request when the permission button is clicked", async () => {
    render(<SettingsGeneralSection active />);
    const btn = await screen.findByRole("button", { name: "권한 요청" });
    fireEvent.click(btn);
    await waitFor(() =>
      expect(api.notificationsRequest).toHaveBeenCalled()
    );
  });
});
