import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

const checkMock = vi.fn();
const relaunchMock = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: (...args: unknown[]) => checkMock(...args),
}));
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: (...args: unknown[]) => relaunchMock(...args),
}));

let capturedInfoArgs: unknown[] = [];
const infoMock = vi.fn((...args: unknown[]) => {
  capturedInfoArgs = args;
});

vi.mock("../../ui/Toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: infoMock,
  }),
}));

import { useAppUpdater } from "../useAppUpdater";

beforeEach(() => {
  vi.useFakeTimers();
  checkMock.mockReset();
  relaunchMock.mockReset();
  infoMock.mockReset();
  capturedInfoArgs = [];
  localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAppUpdater", () => {
  it("shows toast when an update is available", async () => {
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(infoMock).toHaveBeenCalledTimes(1);
    const [message, opts] = capturedInfoArgs as [string, { sticky: boolean; actions: { label: string }[] }];
    expect(message).toContain("0.3.0");
    expect(opts.sticky).toBe(true);
    expect(opts.actions.map((a) => a.label)).toEqual(["지금 재시작", "나중에"]);
  });

  it("does not toast when update.available is false", async () => {
    checkMock.mockResolvedValue({ available: false });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("does not toast when dismissedVersion matches", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({ available: true, version: "0.3.0" });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("toasts again for a newer version than dismissed", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.1",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).toHaveBeenCalledTimes(1);
  });

  it("swallows errors from check() (offline)", async () => {
    checkMock.mockRejectedValue(new Error("offline"));
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("confirm action downloads + relaunches", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall,
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    const [, opts] = capturedInfoArgs as [string, { actions: { label: string; run: () => Promise<void> }[] }];
    const confirm = opts.actions.find((a) => a.label === "지금 재시작")!;
    await act(async () => {
      await confirm.run();
    });
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(relaunchMock).toHaveBeenCalledTimes(1);
  });

  it("dismiss action persists the version to localStorage", async () => {
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall: vi.fn(),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    const [, opts] = capturedInfoArgs as [string, { actions: { label: string; run: () => void }[] }];
    const dismiss = opts.actions.find((a) => a.label === "나중에")!;
    act(() => {
      dismiss.run();
    });
    expect(localStorage.getItem("updater.dismissedVersion")).toBe("0.3.0");
  });

  it("re-checks after 24h interval", async () => {
    checkMock.mockResolvedValue({ available: false });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(24 * 60 * 60 * 1000);
    });
    expect(checkMock).toHaveBeenCalledTimes(2);
  });
});
