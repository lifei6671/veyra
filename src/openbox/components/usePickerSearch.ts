import { useEffect, useRef, useState } from "react";

export function usePickerSearch(open: boolean) {
  const [container, setContainer] = useState<HTMLElement>();
  const input = useRef<HTMLInputElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  useEffect(() => { setContainer(document.querySelector<HTMLElement>(".openbox-app")!); }, []);
  useEffect(() => {
    if (!open) return;
    const frame = requestAnimationFrame(() => {
      popup.current?.querySelector('[data-state="checked"]')?.scrollIntoView({ block: "nearest" });
      input.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [open]);
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") { event.preventDefault(); popup.current?.querySelector<HTMLElement>('[role="option"]:not([data-disabled])')?.focus(); }
    if (event.key !== "Escape" && event.key !== "Tab") event.stopPropagation();
  };
  return { container, input, popup, onKeyDown };
}

