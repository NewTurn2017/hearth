import "@testing-library/jest-dom";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ShortcutRecorder } from "../ShortcutRecorder";

describe("ShortcutRecorder", () => {
  it("captures a modifier+letter combo and emits normalized string", async () => {
    const onSave = vi.fn();
    render(<ShortcutRecorder onSave={onSave} onCancel={() => {}} />);
    const area = screen.getByRole("button", { name: /녹화/ });
    area.focus();
    await userEvent.keyboard("{Meta>}{Shift>}h{/Shift}{/Meta}");
    await userEvent.click(screen.getByRole("button", { name: /확인/ }));
    expect(onSave).toHaveBeenCalledWith("Cmd+Shift+H");
  });

  it("ignores modifier-only input", async () => {
    const onSave = vi.fn();
    render(<ShortcutRecorder onSave={onSave} onCancel={() => {}} />);
    const area = screen.getByRole("button", { name: /녹화/ });
    area.focus();
    await userEvent.keyboard("{Shift}");
    const save = screen.getByRole("button", { name: /확인/ });
    expect(save).toBeDisabled();
  });
});
