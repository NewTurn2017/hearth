import "@testing-library/jest-dom";
import { describe, it, expect, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToastProvider, useToast } from "../Toast";

function Trigger({ onReady }: { onReady: (api: ReturnType<typeof useToast>) => void }) {
  const api = useToast();
  onReady(api);
  return null;
}

function setup() {
  let captured: ReturnType<typeof useToast> | null = null;
  render(
    <ToastProvider>
      <Trigger onReady={(a) => { captured = a; }} />
    </ToastProvider>
  );
  return () => captured!;
}

describe("ToastProvider.info", () => {
  it("renders message with multiple action buttons", () => {
    const getApi = setup();
    const run1 = vi.fn();
    const run2 = vi.fn();
    act(() => {
      getApi().info("새 버전 v0.3.0 준비됨", {
        sticky: true,
        actions: [
          { label: "지금 재시작", run: run1 },
          { label: "나중에", run: run2 },
        ],
      });
    });
    expect(screen.getByText("새 버전 v0.3.0 준비됨")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "지금 재시작" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "나중에" })).toBeInTheDocument();
  });

  it("invokes the action's run on click and dismisses the toast", async () => {
    const user = userEvent.setup();
    const getApi = setup();
    const run = vi.fn();
    act(() => {
      getApi().info("hello", {
        sticky: true,
        actions: [{ label: "ok", run }],
      });
    });
    await user.click(screen.getByRole("button", { name: "ok" }));
    expect(run).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("hello")).not.toBeInTheDocument();
  });

  it("sticky toast does not auto-dismiss", () => {
    vi.useFakeTimers();
    const getApi = setup();
    act(() => {
      getApi().info("stays", { sticky: true, actions: [] });
    });
    act(() => { vi.advanceTimersByTime(10_000); });
    expect(screen.getByText("stays")).toBeInTheDocument();
    vi.useRealTimers();
  });
});
