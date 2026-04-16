// src/command/CommandEmpty.tsx
export function CommandEmpty({ text }: { text: string }) {
  return (
    <div className="px-4 py-8 text-center text-[12px] text-[var(--color-text-dim)]">
      {text}
    </div>
  );
}
