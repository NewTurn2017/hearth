// src/components/__tests__/SettingsThemeSection.test.tsx
import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import type { ReactNode } from "react";
import { SettingsThemeSection } from "../SettingsThemeSection";
import { ThemeProvider } from "../../theme/ThemeContext";

vi.mock("../../api", () => ({
  getTheme: vi.fn().mockResolvedValue(null),
  setTheme: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import * as api from "../../api";

function wrap(ui: ReactNode) {
  return render(<ThemeProvider>{ui}</ThemeProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
});

describe("SettingsThemeSection", () => {
  it("renders all 10 preset cards grouped into dark + light", () => {
    wrap(<SettingsThemeSection />);
    expect(screen.getByRole("button", { name: /Warm Paper/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Midnight/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Forest/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Plum/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Carbon/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Cream/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Linen/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Mint/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Blush/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Arctic/ })).toBeInTheDocument();
  });

  it("clicking a preset card calls setTheme and sets data-theme", async () => {
    wrap(<SettingsThemeSection />);
    fireEvent.click(screen.getByRole("button", { name: /Midnight/ }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenCalledWith({ kind: "preset", id: "midnight" });
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("midnight");
  });

  it("marks the active preset card with aria-pressed", async () => {
    wrap(<SettingsThemeSection />);
    const carbon = screen.getByRole("button", { name: /Carbon/ });
    fireEvent.click(carbon);
    await waitFor(() => expect(carbon).toHaveAttribute("aria-pressed", "true"));
    expect(screen.getByRole("button", { name: /Midnight/ })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("debounces custom color input and previews live before save", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    const hexInput = screen.getByLabelText("강조색 HEX") as HTMLInputElement;
    fireEvent.change(hexInput, { target: { value: "#ff8000" } });
    // Before 300ms: still on previous theme
    expect(document.documentElement.getAttribute("data-theme")).not.toBe("custom");
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("custom");
    // Preview should NOT persist until "저장" click
    expect(api.setTheme).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("rejects invalid HEX with a visible error", async () => {
    wrap(<SettingsThemeSection />);
    const hexInput = screen.getByLabelText("강조색 HEX");
    fireEvent.change(hexInput, { target: { value: "nope" } });
    expect(await screen.findByText(/올바른 HEX/)).toBeInTheDocument();
  });

  it("save button persists the currently-previewed custom theme", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    fireEvent.change(screen.getByLabelText("강조색 HEX"), {
      target: { value: "#10b981" },
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    vi.useRealTimers();
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenCalledWith({
        kind: "custom",
        baseMode: "dark",
        brandHex: "#10b981",
      });
    });
  });

  it("revert link goes back to warm-paper when no prior preset", async () => {
    vi.useFakeTimers();
    wrap(<SettingsThemeSection />);
    fireEvent.change(screen.getByLabelText("강조색 HEX"), {
      target: { value: "#ff8000" },
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    vi.useRealTimers();
    fireEvent.click(screen.getByRole("button", { name: /프리셋으로 되돌리기/ }));
    await waitFor(() => {
      expect(api.setTheme).toHaveBeenLastCalledWith({
        kind: "preset",
        id: "warm-paper",
      });
    });
  });
});
