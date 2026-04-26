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
const successMock = vi.fn();
const errorMock = vi.fn();

vi.mock("../../ui/Toast", () => ({
  useToast: () => ({
    success: successMock,
    error: errorMock,
    info: infoMock,
  }),
}));

import { useAppUpdater } from "../useAppUpdater";

beforeEach(() => {
  vi.useFakeTimers();
  checkMock.mockReset();
  relaunchMock.mockReset();
  infoMock.mockReset();
  successMock.mockReset();
  errorMock.mockReset();
  capturedInfoArgs = [];
  localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAppUpdater", () => {
  it("shows toast when an update is available", async () => {
    checkMock.mockResolvedValue({
      version: "0.3.0",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
      close: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(infoMock).toHaveBeenCalledTimes(1);
    const [message, opts] = capturedInfoArgs as [
      string,
      { sticky: boolean; actions: { label: string }[] },
    ];
    expect(message).toContain("0.3.0");
    expect(opts.sticky).toBe(true);
    expect(opts.actions.map((a) => a.label)).toEqual(["지금 재시작", "나중에"]);
  });

  it("does not toast when check() returns null (no update)", async () => {
    checkMock.mockResolvedValue(null);
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("does not toast when dismissedVersion matches, and releases the update handle", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    const close = vi.fn().mockResolvedValue(undefined);
    checkMock.mockResolvedValue({ version: "0.3.0", close });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    expect(infoMock).not.toHaveBeenCalled();
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("toasts again for a newer version than dismissed", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({
      version: "0.3.1",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
      close: vi.fn().mockResolvedValue(undefined),
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

  it("manual check reports when already up to date", async () => {
    checkMock.mockResolvedValue(null);
    const { result } = renderHook(() => useAppUpdater());
    await act(async () => {
      await result.current.checkNow();
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(successMock).toHaveBeenCalledWith("현재 최신 버전입니다.");
  });

  it("manual check reports failures", async () => {
    checkMock.mockRejectedValue(new Error("offline"));
    const { result } = renderHook(() => useAppUpdater());
    await act(async () => {
      await result.current.checkNow();
    });
    expect(errorMock).toHaveBeenCalledWith(
      "업데이트 확인 실패: Error: offline",
    );
  });

  it("manual check ignores a dismissed version and makes it pending again", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({
      version: "0.3.0",
      downloadAndInstall: vi.fn(),
      close: vi.fn().mockResolvedValue(undefined),
    });
    const { result } = renderHook(() => useAppUpdater());
    await act(async () => {
      await result.current.checkNow();
    });
    expect(result.current.pending?.version).toBe("0.3.0");
    expect(localStorage.getItem("updater.dismissedVersion")).toBeNull();
    expect(infoMock).toHaveBeenCalledTimes(1);
  });

  it("confirm action downloads + relaunches", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    checkMock.mockResolvedValue({
      version: "0.3.0",
      downloadAndInstall,
      close: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    const [, opts] = capturedInfoArgs as [
      string,
      { actions: { label: string; run: () => Promise<void> }[] },
    ];
    const confirm = opts.actions.find((a) => a.label === "지금 재시작")!;
    await act(async () => {
      await confirm.run();
    });
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(relaunchMock).toHaveBeenCalledTimes(1);
  });

  it("dismiss action persists the version + releases the update handle", async () => {
    const close = vi.fn().mockResolvedValue(undefined);
    checkMock.mockResolvedValue({
      version: "0.3.0",
      downloadAndInstall: vi.fn(),
      close,
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_001);
    });
    const [, opts] = capturedInfoArgs as [
      string,
      { actions: { label: string; run: () => Promise<void> }[] },
    ];
    const dismiss = opts.actions.find((a) => a.label === "나중에")!;
    await act(async () => {
      await dismiss.run();
    });
    expect(localStorage.getItem("updater.dismissedVersion")).toBe("0.3.0");
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("re-checks after 24h interval", async () => {
    checkMock.mockResolvedValue(null);
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
