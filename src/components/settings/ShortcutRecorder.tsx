import { useCallback, useState } from "react";
import { Button } from "../../ui/Button";

type Captured = {
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
  shift: boolean;
  key: string | null;
};

const EMPTY: Captured = {
  ctrl: false,
  meta: false,
  alt: false,
  shift: false,
  key: null,
};

function format(c: Captured): string | null {
  if (!c.key) return null;
  const parts: string[] = [];
  if (c.meta) parts.push("Cmd");
  else if (c.ctrl) parts.push("Ctrl");
  if (c.alt) parts.push("Alt");
  if (c.shift) parts.push("Shift");
  parts.push(c.key);
  return parts.join("+");
}

export function ShortcutRecorder({
  onSave,
  onCancel,
}: {
  onSave: (combo: string) => void;
  onCancel: () => void;
}) {
  const [cap, setCap] = useState<Captured>(EMPTY);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLButtonElement>) => {
      e.preventDefault();
      const k = e.key;
      if (k === "Escape") {
        onCancel();
        return;
      }
      const mods = {
        ctrl: e.ctrlKey,
        meta: e.metaKey,
        alt: e.altKey,
        shift: e.shiftKey,
      };
      if (["Control", "Meta", "Alt", "Shift"].includes(k)) {
        setCap((prev) => ({ ...prev, ...mods }));
        return;
      }
      let key: string | null = null;
      if (k.length === 1 && /[a-zA-Z0-9]/.test(k)) key = k.toUpperCase();
      else if (/^F\d{1,2}$/.test(k)) key = k;
      else if (k === " ") key = "Space";
      else if (
        [
          "Enter",
          "Tab",
          "Backspace",
          "Delete",
          "ArrowUp",
          "ArrowDown",
          "ArrowLeft",
          "ArrowRight",
        ].includes(k)
      )
        key = k;
      if (!key) return;
      setCap({ ...mods, key });
    },
    [onCancel]
  );

  const display = format(cap) ?? "키를 눌러주세요...";
  const canSave = !!format(cap);

  return (
    <div className="flex flex-col gap-3">
      <button
        type="button"
        aria-label="단축키 녹화 영역 — 키를 누르세요"
        onKeyDown={onKeyDown}
        className="rounded border border-white/15 px-4 py-3 text-center text-sm bg-black/30 focus:outline-none focus:ring-2 focus:ring-amber-400"
      >
        {display}
      </button>
      <div className="flex gap-2 justify-end">
        <Button variant="secondary" size="sm" onClick={onCancel}>
          취소
        </Button>
        <Button
          size="sm"
          disabled={!canSave}
          onClick={() => {
            const c = format(cap);
            if (c) onSave(c);
          }}
        >
          확인
        </Button>
      </div>
    </div>
  );
}
