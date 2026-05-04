import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUiScale, __STEPS_FOR_TEST } from "../useUiScale";

vi.mock("../../api", () => ({
  getUiScale: vi.fn().mockResolvedValue(1.0),
  setUiScale: vi.fn().mockResolvedValue(undefined),
}));

describe("useUiScale", () => {
  beforeEach(() => {
    document.documentElement.style.zoom = "";
    document.documentElement.style.removeProperty("--ui-scale");
  });

  it("uses the DEFAULT step when none is persisted", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    expect(result.current.scale).toBe(1.0);
  });

  it("bump(+1) moves one step up, clamped at the max", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => {
      result.current.bump(1);
    });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[2]);
    act(() => {
      result.current.bump(1);
    });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[3]);
    act(() => {
      result.current.bump(1);
    });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[3]);
  });

  it("bump(-1) moves one step down, clamped at the min", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => {
      result.current.bump(-1);
    });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[0]);
    act(() => {
      result.current.bump(-1);
    });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[0]);
  });

  it("reset() returns to DEFAULT", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => {
      result.current.bump(1);
    });
    act(() => {
      result.current.reset();
    });
    expect(result.current.scale).toBe(1.0);
  });

  it("syncs the --ui-scale CSS variable on every step so layout containers can compensate", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    expect(
      document.documentElement.style.getPropertyValue("--ui-scale"),
    ).toBe("1");
    act(() => {
      result.current.bump(-1);
    });
    expect(
      document.documentElement.style.getPropertyValue("--ui-scale"),
    ).toBe(String(__STEPS_FOR_TEST[0]));
    act(() => {
      result.current.reset();
    });
    expect(
      document.documentElement.style.getPropertyValue("--ui-scale"),
    ).toBe("1");
  });
});
