import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useContextMenu } from "../useContextMenu";

describe("useContextMenu", () => {
  it("open(e) calls preventDefault + stopPropagation and stores coords", () => {
    const { result } = renderHook(() => useContextMenu());
    const preventDefault = vi.fn();
    const stopPropagation = vi.fn();
    const fakeEvent = {
      clientX: 120,
      clientY: 80,
      preventDefault,
      stopPropagation,
    } as unknown as React.MouseEvent;

    act(() => {
      result.current.open(fakeEvent);
    });

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(stopPropagation).toHaveBeenCalledOnce();
    expect(result.current.menu.open).toBe(true);
    expect(result.current.menu.x).toBe(120);
    expect(result.current.menu.y).toBe(80);
  });

  it("close() resets open to false", () => {
    const { result } = renderHook(() => useContextMenu());
    const fakeEvent = {
      clientX: 1,
      clientY: 1,
      preventDefault: () => {},
      stopPropagation: () => {},
    } as unknown as React.MouseEvent;

    act(() => {
      result.current.open(fakeEvent);
    });
    expect(result.current.menu.open).toBe(true);

    act(() => {
      result.current.close();
    });
    expect(result.current.menu.open).toBe(false);
  });
});
