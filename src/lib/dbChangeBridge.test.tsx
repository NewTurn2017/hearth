import { render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DB_CHANGE_EVENTS, useTauriDbChangeBridge } from "./dbChangeBridge";

const handlers = new Map<string, () => void>();
const unlisteners = new Map<string, ReturnType<typeof vi.fn>>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, handler: () => void) => {
    handlers.set(eventName, handler);
    const unlisten = vi.fn();
    unlisteners.set(eventName, unlisten);
    return Promise.resolve(unlisten);
  }),
}));

function Harness() {
  useTauriDbChangeBridge();
  return null;
}

afterEach(() => {
  handlers.clear();
  unlisteners.clear();
  vi.clearAllMocks();
});

describe("useTauriDbChangeBridge", () => {
  it("redispatches Tauri database change events as DOM events", async () => {
    const onMemosChanged = vi.fn();
    window.addEventListener("memos:changed", onMemosChanged);

    const view = render(<Harness />);
    await waitFor(() => {
      expect(handlers.size).toBe(DB_CHANGE_EVENTS.length);
    });

    handlers.get("memos:changed")?.();

    expect(onMemosChanged).toHaveBeenCalledTimes(1);

    view.unmount();
    await waitFor(() => {
      expect(unlisteners.get("memos:changed")).toHaveBeenCalledTimes(1);
    });

    window.removeEventListener("memos:changed", onMemosChanged);
  });
});
