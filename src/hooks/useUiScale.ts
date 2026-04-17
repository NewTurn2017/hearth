import { useCallback, useEffect, useState } from "react";
import * as api from "../api";

const STEPS = [0.85, 1.0, 1.15, 1.3] as const;
const DEFAULT: number = 1.0;

export const __STEPS_FOR_TEST = STEPS;

export function useUiScale() {
  const [scale, setScale] = useState<number>(DEFAULT);

  const apply = useCallback((next: number) => {
    document.documentElement.style.zoom = String(next);
    setScale(next);
    api.setUiScale(next).catch(() => {});
  }, []);

  const bump = useCallback((dir: 1 | -1) => {
    setScale((current) => {
      const idx = STEPS.indexOf(current as (typeof STEPS)[number]);
      const base =
        idx === -1 ? STEPS.indexOf(DEFAULT as (typeof STEPS)[number]) : idx;
      const nextIdx = Math.max(0, Math.min(STEPS.length - 1, base + dir));
      const next = STEPS[nextIdx];
      document.documentElement.style.zoom = String(next);
      api.setUiScale(next).catch(() => {});
      return next;
    });
  }, []);

  const reset = useCallback(() => apply(DEFAULT), [apply]);

  useEffect(() => {
    api
      .getUiScale()
      .then((v) => apply(Number.isFinite(v) && v > 0 ? v : DEFAULT))
      .catch(() => apply(DEFAULT));
  }, [apply]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        bump(1);
      } else if (e.key === "-") {
        e.preventDefault();
        bump(-1);
      } else if (e.key === "0") {
        e.preventDefault();
        reset();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bump, reset]);

  return { scale, bump, reset };
}
