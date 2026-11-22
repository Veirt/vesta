import { IconArrowsHorizontal } from "@tabler/icons-react";
import { useRef, type PointerEvent as ReactPointerEvent } from "react";

export function GroupResizeHandle({
  label,
  width,
  onResize,
}: {
  label: string;
  width: number;
  onResize: (width: number) => void;
}) {
  const dragStart = useRef<{ clientX: number; span: number; groupWidth: number } | null>(null);

  function resizeFromPointer(event: ReactPointerEvent<HTMLDivElement>) {
    const start = dragStart.current;
    if (!start || start.groupWidth <= 0) return;
    const columnWidth = start.groupWidth / start.span;
    onResize(start.span + (event.clientX - start.clientX) / columnWidth);
  }

  return (
    <div
      className="dashboard-group-resize-handle"
      role="slider"
      aria-label={`Resize ${label} width`}
      aria-orientation="vertical"
      aria-valuemin={3}
      aria-valuemax={12}
      aria-valuenow={width}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") onResize(width - 1);
        if (event.key === "ArrowRight") onResize(width + 1);
      }}
      onPointerDown={(event) => {
        const groupWidth =
          event.currentTarget.closest("section")?.getBoundingClientRect().width ?? 0;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        dragStart.current = { clientX: event.clientX, span: width, groupWidth };
      }}
      onPointerMove={resizeFromPointer}
      onPointerUp={(event) => {
        dragStart.current = null;
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onPointerCancel={() => {
        dragStart.current = null;
      }}
    >
      <IconArrowsHorizontal size={12} aria-hidden="true" />
    </div>
  );
}
