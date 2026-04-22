// Settings → 테마 tab content. Two preset grids (dark + light) plus a
// custom-brand picker with 300ms-debounced live preview and explicit save.

import { useEffect, useMemo, useRef, useState } from "react";
import { Check } from "lucide-react";
import { cn } from "../lib/cn";
import { Button } from "../ui/Button";
import { useTheme } from "../theme/ThemeContext";
import { DARK_PRESETS, LIGHT_PRESETS, type PresetId } from "../theme/types";
import { PRESETS, PRESET_META } from "../theme/presets";
import { applyTheme } from "../theme/applyTheme";

const HEX_RE = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

function PresetCard({
  id,
  active,
  onSelect,
}: {
  id: PresetId;
  active: boolean;
  onSelect: () => void;
}) {
  const p = PRESETS[id];
  const meta = PRESET_META[id];
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onSelect}
      className={cn(
        "relative flex flex-col gap-2 rounded-[var(--radius-lg)] p-3 text-left transition-colors",
        "border-2",
        active
          ? "border-[var(--color-brand)]"
          : "border-[var(--color-border)] hover:border-[var(--color-border-strong)]",
      )}
      style={{ background: p["--color-surface-1"] }}
    >
      <div className="flex gap-1">
        <span className="h-5 w-5 rounded" style={{ background: p["--color-surface-0"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-surface-2"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-brand"] }} />
        <span className="h-5 w-5 rounded" style={{ background: p["--color-text-hi"] }} />
      </div>
      <div className="flex items-center justify-between">
        <span className="text-[13px] font-medium" style={{ color: p["--color-text-hi"] }}>
          {meta.label}
        </span>
        {active && (
          <Check size={14} style={{ color: p["--color-brand"] }} aria-hidden />
        )}
      </div>
    </button>
  );
}

export function SettingsThemeSection() {
  const { theme, setTheme } = useTheme();

  const activePresetId: PresetId | null =
    theme.kind === "preset" ? theme.id : null;

  // Remember the last preset the user was on so the "revert" link has
  // somewhere to go back to.
  const lastPresetRef = useRef<PresetId>(
    theme.kind === "preset" ? theme.id : "warm-paper",
  );
  useEffect(() => {
    if (theme.kind === "preset") lastPresetRef.current = theme.id;
  }, [theme]);

  // Custom picker state. Seeded from the current theme so switching to the
  // custom tab mid-session shows the current values.
  const [baseMode, setBaseMode] = useState<"light" | "dark">(
    theme.kind === "custom" ? theme.baseMode : "dark",
  );
  const [hexInput, setHexInput] = useState<string>(
    theme.kind === "custom" ? theme.brandHex : "#d97706",
  );

  const normalizedHex = useMemo(() => {
    const v = hexInput.trim();
    if (!HEX_RE.test(v)) return null;
    return v.startsWith("#") ? v : `#${v}`;
  }, [hexInput]);

  // Debounced preview: re-apply but do NOT persist until the user clicks 저장.
  // Apply visually only — the context's setTheme would persist, which we
  // only want on explicit save.
  useEffect(() => {
    if (!normalizedHex) return;
    const t = setTimeout(() => {
      applyTheme({ kind: "custom", baseMode, brandHex: normalizedHex });
    }, 300);
    return () => clearTimeout(t);
  }, [normalizedHex, baseMode]);

  const onSavePreset = async (id: PresetId) => {
    await setTheme({ kind: "preset", id });
  };

  const onSaveCustom = async () => {
    if (!normalizedHex) return;
    await setTheme({ kind: "custom", baseMode, brandHex: normalizedHex });
  };

  const onRevert = async () => {
    await setTheme({ kind: "preset", id: lastPresetRef.current });
  };

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">다크</h3>
        <div className="grid grid-cols-3 gap-2">
          {DARK_PRESETS.map((id) => (
            <PresetCard
              key={id}
              id={id}
              active={activePresetId === id}
              onSelect={() => void onSavePreset(id)}
            />
          ))}
        </div>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">라이트</h3>
        <div className="grid grid-cols-3 gap-2">
          {LIGHT_PRESETS.map((id) => (
            <PresetCard
              key={id}
              id={id}
              active={activePresetId === id}
              onSelect={() => void onSavePreset(id)}
            />
          ))}
        </div>
      </section>

      <section
        className={cn(
          "rounded-[var(--radius-lg)] border-2 p-3 flex flex-col gap-3",
          theme.kind === "custom"
            ? "border-[var(--color-brand)]"
            : "border-[var(--color-border)]",
        )}
      >
        <h3 className="text-[13px] text-[var(--color-text-hi)]">커스텀</h3>

        <div className="flex items-center gap-4 text-[13px]">
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name="theme-base-mode"
              checked={baseMode === "dark"}
              onChange={() => setBaseMode("dark")}
            />
            다크
          </label>
          <label className="flex items-center gap-1.5">
            <input
              type="radio"
              name="theme-base-mode"
              checked={baseMode === "light"}
              onChange={() => setBaseMode("light")}
            />
            라이트
          </label>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="color"
            aria-label="강조색 색상 선택"
            value={normalizedHex ?? "#d97706"}
            onChange={(e) => setHexInput(e.target.value)}
            className="h-8 w-10 cursor-pointer rounded border border-[var(--color-border)]"
          />
          <input
            type="text"
            aria-label="강조색 HEX"
            value={hexInput}
            onChange={(e) => setHexInput(e.target.value)}
            placeholder="#rrggbb"
            className="font-mono rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-[12px]"
          />
          <Button size="sm" onClick={() => void onSaveCustom()} disabled={!normalizedHex}>
            저장
          </Button>
        </div>

        {!normalizedHex && (
          <p className="text-[11px] text-red-400">
            올바른 HEX 값을 입력하세요 (예: #ff8000)
          </p>
        )}

        <button
          type="button"
          onClick={() => void onRevert()}
          className="self-start text-[11px] text-[var(--color-text-muted)] hover:text-[var(--color-text)] underline"
        >
          프리셋으로 되돌리기
        </button>
      </section>
    </div>
  );
}
