import { Bars3Icon } from "@heroicons/react/24/outline";
import { useEffect, useRef, useState, type PointerEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

type Drag = { id: string | number; x: number; y: number; startX: number; startY: number; offsetX: number; offsetY: number; width: number; active: boolean };

// Match the reference's handle-only fallback drag: one full row above the app,
// a dim placeholder in the list, and no browser-generated drag image.
export function SortableList<T>({ items, itemKey, renderItem, onChange, onEnd, disabled, className = "" }: {
  items: T[];
  itemKey: (item: T) => string | number;
  renderItem: (item: T, handle: ReactNode) => ReactNode;
  onChange: (items: T[]) => void;
  onEnd?: (items: T[]) => void;
  disabled?: boolean;
  className?: string;
}) {
  const root = useRef<HTMLDivElement>(null);
  const elements = useRef(new Map<string | number, HTMLDivElement>());
  const ordered = useRef(items);
  ordered.current = items;
  const dragState = useRef<Drag | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const dragged = drag?.active ? items.find(item => itemKey(item) === drag.id) : undefined;
  const reorder = (y: number) => {
    const pointer = dragState.current;
    if (!pointer?.active) return;
    const from = ordered.current.findIndex(item => itemKey(item) === pointer.id);
    const to = ordered.current.findIndex(item => { const rect = elements.current.get(itemKey(item))!.getBoundingClientRect(); return y >= rect.top && y <= rect.bottom; });
    if (to < 0 || to === from) return;
    const rect = elements.current.get(itemKey(ordered.current[to]))!.getBoundingClientRect();
    if (from < to ? y < rect.top + rect.height * .25 : y > rect.top + rect.height * .75) return;
    const old = new Map([...elements.current].map(([key, el]) => [key, el.getBoundingClientRect().top]));
    const next = [...ordered.current];
    next.splice(to, 0, next.splice(from, 1)[0]);
    ordered.current = next;
    onChange(next);
    requestAnimationFrame(() => {
      if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      elements.current.forEach((el, key) => {
        el.getAnimations().forEach(animation => animation.cancel());
        const delta = old.get(key)! - el.getBoundingClientRect().top;
        if (Math.abs(delta) > .5) el.animate([{ transform: `translateY(${delta}px)` }, { transform: "translateY(0)" }], { duration: 150, easing: "ease" });
      });
    });
  };
  const begin = (event: PointerEvent<SVGSVGElement>, item: T) => {
    if (disabled || event.button !== 0) return;
    event.preventDefault();
    const id = itemKey(item), rect = elements.current.get(id)!.getBoundingClientRect();
    root.current!.setPointerCapture(event.pointerId);
    dragState.current = { id, x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY, offsetX: event.clientX - rect.left, offsetY: event.clientY - rect.top, width: rect.width, active: false };
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    const pointer = dragState.current;
    if (!pointer) return;
    const active = pointer.active || Math.hypot(event.clientX - pointer.startX, event.clientY - pointer.startY) >= 3;
    dragState.current = { ...pointer, x: event.clientX, y: event.clientY, active };
    if (active) { setDrag(dragState.current); reorder(event.clientY); }
  };
  const finish = (event: PointerEvent<HTMLDivElement>) => {
    const active = dragState.current?.active;
    dragState.current = null;
    setDrag(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (active) onEnd?.(ordered.current);
  };
  useEffect(() => {
    if (!drag?.active) return;
    const scroller = root.current!.closest<HTMLElement>(".ob-modal-body, .settings-content")!;
    let frame: number;
    const scroll = () => {
      const pointer = dragState.current;
      if (!pointer) return;
      const rect = scroller.getBoundingClientRect(), before = scroller.scrollTop;
      if (pointer.y < rect.top + 24) scroller.scrollTop -= 6;
      else if (pointer.y > rect.bottom - 24) scroller.scrollTop += 6;
      if (before !== scroller.scrollTop) reorder(pointer.y);
      frame = requestAnimationFrame(scroll);
    };
    frame = requestAnimationFrame(scroll);
    return () => cancelAnimationFrame(frame);
  }, [drag?.active]);
  return <>
    <div ref={root} className={`sortable-list ${className}`} onPointerMove={move} onPointerUp={finish} onPointerCancel={finish} onLostPointerCapture={finish} onDragStart={event => event.preventDefault()}>
      {items.map(item => <div key={itemKey(item)} ref={el => { if (el) elements.current.set(itemKey(item), el); else elements.current.delete(itemKey(item)); }} className={`sortable-row${drag?.active && drag.id === itemKey(item) ? " sortable-placeholder" : ""}`}>{renderItem(item, <Bars3Icon className="sortable-handle" aria-label="拖拽排序" onPointerDown={event => begin(event, item)} />)}</div>)}
    </div>
    {dragged !== undefined && drag && createPortal(<div aria-hidden="true" className={`sortable-preview ${className}`} style={{ left: drag.x - drag.offsetX, top: drag.y - drag.offsetY, width: drag.width }}>{renderItem(dragged, <Bars3Icon className="sortable-handle" />)}</div>, document.querySelector(".openbox-app")!)}
  </>;
}
