import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import { ContextMenu } from "../ContextMenu";

describe("ContextMenu", () => {
  it("renders nothing when closed", () => {
    const { container } = render(
      <ContextMenu
        open={false}
        x={10}
        y={10}
        items={[{ id: "a", label: "A", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders items when open", () => {
    render(
      <ContextMenu
        open
        x={10}
        y={10}
        items={[{ id: "a", label: "Alpha", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    expect(screen.getByText("Alpha")).toBeInTheDocument();
  });

  it("clamps x so the panel does not overflow the right edge", () => {
    Object.defineProperty(window, "innerWidth", { value: 800, writable: true });
    render(
      <ContextMenu
        open
        x={790}
        y={10}
        items={[{ id: "a", label: "A", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    const panel = screen.getByRole("menu");
    const left = parseFloat((panel as HTMLElement).style.left);
    // Panel width is 208px (min-w-[208px]); clamp leaves at least 8px margin.
    // Panel must not start later than innerWidth - panelW - margin.
    expect(left).toBeLessThanOrEqual(800 - 208 - 8 + 0.5);
  });
});
