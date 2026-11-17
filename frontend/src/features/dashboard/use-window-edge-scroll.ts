import { useEffect, useRef } from "react";
import { eventClientY } from "@/features/dashboard/tile-grid";

/**
 * While a tile drag is in progress, auto-scroll the window when the pointer
 * nears the top or bottom edge. The animation frame loop is stopped on unmount.
 */
function useWindowEdgeScroll() {
  const direction = useRef(0);
  const step = useRef(0);
  const frame = useRef<number | null>(null);

  function stopWindowEdgeScroll() {
    direction.current = 0;
    if (frame.current !== null) {
      cancelAnimationFrame(frame.current);
      frame.current = null;
    }
  }

  function handleWindowEdgeScroll(event: Event) {
    const clientY = eventClientY(event);
    if (clientY === null) return;
    const edgeSize = 72;
    const distanceToEdge = Math.min(clientY, window.innerHeight - clientY);
    if (distanceToEdge >= edgeSize) {
      direction.current = 0;
      return;
    }

    direction.current = clientY < edgeSize ? -1 : 1;
    step.current = Math.max(2, Math.ceil(((edgeSize - distanceToEdge) / edgeSize) * 10));
    if (frame.current !== null) return;

    const scroll = () => {
      const currentDirection = direction.current;
      if (!currentDirection) {
        frame.current = null;
        return;
      }

      const maxScroll = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
      const canScroll = currentDirection < 0 ? window.scrollY > 0 : window.scrollY < maxScroll;
      if (!canScroll) {
        frame.current = null;
        return;
      }

      window.scrollBy(0, currentDirection * step.current);
      frame.current = requestAnimationFrame(scroll);
    };
    frame.current = requestAnimationFrame(scroll);
  }

  useEffect(
    () => () => {
      direction.current = 0;
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    },
    [],
  );

  return { handleWindowEdgeScroll, stopWindowEdgeScroll };
}

export { useWindowEdgeScroll };
