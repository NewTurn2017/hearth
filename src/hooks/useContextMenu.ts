import { useCallback, useState, type MouseEvent } from "react";

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
}

export function useContextMenu() {
  const [menu, setMenu] = useState<ContextMenuState>({
    open: false,
    x: 0,
    y: 0,
  });

  const open = useCallback((e: MouseEvent) => {
    // `preventDefault` suppresses the native WebKit menu; `stopPropagation`
    // prevents the global blocker effect in Layout (which also calls
    // preventDefault) from hiding our own menu by bubbling.
    e.preventDefault();
    e.stopPropagation();
    setMenu({ open: true, x: e.clientX, y: e.clientY });
  }, []);

  const close = useCallback(() => {
    setMenu((prev) => ({ ...prev, open: false }));
  }, []);

  return { menu, open, close };
}
