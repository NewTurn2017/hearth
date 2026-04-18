import "@testing-library/jest-dom";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ScheduleModal } from "../ScheduleModal";

describe("ScheduleModal notify toggle", () => {
  it("hides the time picker when notify is off", () => {
    render(
      <ScheduleModal onSave={vi.fn()} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    expect(screen.queryByLabelText("시간")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("5분 전")).not.toBeInTheDocument();
  });

  it("reveals time picker + checkboxes when notify is turned on", () => {
    render(
      <ScheduleModal onSave={vi.fn()} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByLabelText("알림 받기"));
    expect(screen.getByLabelText("시간")).toBeInTheDocument();
    expect(screen.getByLabelText("5분 전")).toBeChecked();
    expect(screen.getByLabelText("정각")).not.toBeChecked();
  });

  it("emits notify fields on save when toggle is on", () => {
    const onSave = vi.fn();
    render(
      <ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByLabelText("알림 받기"));
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        date: "2026-04-20",
        time: "09:00",
        remind_before_5min: true,
        remind_at_start: false,
      })
    );
  });

  it("omits time + flags when toggle stays off", () => {
    const onSave = vi.fn();
    render(
      <ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        date: "2026-04-20",
        time: undefined,
        remind_before_5min: false,
        remind_at_start: false,
      })
    );
  });

  it("hydrates notify=true when editing a schedule with time", () => {
    render(
      <ScheduleModal
        onSave={vi.fn()}
        onClose={vi.fn()}
        schedule={{
          id: 1,
          date: "2026-04-20",
          time: "10:00",
          location: null,
          description: null,
          notes: null,
          remind_before_5min: true,
          remind_at_start: false,
          created_at: "",
          updated_at: "",
        }}
      />
    );
    expect(screen.getByLabelText("알림 받기")).toBeChecked();
    expect(screen.getByLabelText("시간")).toHaveValue("10:00");
    expect(screen.getByLabelText("5분 전")).toBeChecked();
  });
});

describe("ScheduleModal IME-safe Enter", () => {
  it("submits on Enter when composition is not active", () => {
    const onSave = vi.fn();
    render(<ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />);
    const location = screen.getByLabelText("장소");
    // fireEvent.keyDown exposes the synthetic KeyboardEvent; jsdom default
    // isComposing=false / keyCode=13 for Enter.
    fireEvent.keyDown(location, { key: "Enter", code: "Enter" });
    expect(onSave).toHaveBeenCalled();
  });

  it("does NOT submit on Enter during IME composition", () => {
    const onSave = vi.fn();
    render(<ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />);
    const location = screen.getByLabelText("장소");
    fireEvent.keyDown(location, {
      key: "Enter",
      code: "Enter",
      keyCode: 229, // WebKit legacy marker for IME-in-progress
      isComposing: true,
    });
    expect(onSave).not.toHaveBeenCalled();
  });
});
