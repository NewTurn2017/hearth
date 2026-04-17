import { describe, it, expect } from "vitest";
import { deriveTarget } from "../dragTargets";
import type { Project } from "../../types";

const mk = (id: number, priority: Project["priority"]): Project => ({
  id,
  priority,
  number: null,
  name: `p${id}`,
  category: null,
  path: null,
  evaluation: null,
  sort_order: 0,
  created_at: "",
  updated_at: "",
});

const projects: Project[] = [mk(1, "P0"), mk(2, "P0"), mk(3, "P2")];

describe("deriveTarget", () => {
  it("resolves a card id to its priority", () => {
    expect(deriveTarget(3, projects)).toEqual({ priority: "P2", overId: 3 });
  });

  it("resolves an empty-zone id to its priority with null overId", () => {
    expect(deriveTarget("priority-P4-empty", projects)).toEqual({
      priority: "P4",
      overId: null,
    });
  });

  it("returns null for an unknown id", () => {
    expect(deriveTarget(999, projects)).toBeNull();
    expect(deriveTarget("priority-PX-empty", projects)).toBeNull();
  });
});
