import { useEffect, useRef, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import {
  createMemo,
  hideQuickCaptureWindow,
  resizeQuickCaptureWindow,
} from "../api";

export function QuickCapture() {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const taRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    taRef.current?.focus();
  }, []);

  useEffect(() => {
    const onBlur = () => {
      if (!busy) void hideQuickCaptureWindow();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [busy]);

  useEffect(() => {
    const el = taRef.current;
    if (!el) return;
    el.style.height = "auto";
    const h = Math.min(Math.max(el.scrollHeight + 16, 80), 200);
    void resizeQuickCaptureWindow(h);
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  async function submit() {
    const content = value.trim();
    if (!content) {
      await hideQuickCaptureWindow();
      return;
    }
    setBusy(true);
    try {
      const memo = await createMemo({ content, color: "yellow" });
      await emit("memo:quick-captured", { memoId: memo.id });
      setValue("");
      await hideQuickCaptureWindow();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      void hideQuickCaptureWindow();
    }
  }

  return (
    <div className="h-full w-full p-3 bg-[var(--color-surface,_#1b1b1b)]/95 backdrop-blur rounded-xl shadow-2xl border border-white/10">
      <textarea
        ref={taRef}
        role="textbox"
        value={value}
        rows={1}
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder="뇌에 있는 거 한 줄..."
        aria-label="Quick Capture"
        className="w-full resize-none bg-transparent outline-none text-[15px] text-white placeholder-white/40"
      />
      {error && (
        <div className="mt-2 text-xs text-red-400">저장 실패 — {error}</div>
      )}
    </div>
  );
}

export default QuickCapture;
