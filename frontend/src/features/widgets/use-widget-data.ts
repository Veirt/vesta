import { useEffect, useState } from "react";
import { fetchWidgetData } from "@/shared/api/client";
import type { WidgetData } from "@/features/widgets/widget-schemas";

export type WidgetDataState =
  | { status: "loading"; data: null; error: "" }
  | { status: "ready"; data: WidgetData; error: "" }
  | { status: "error"; data: WidgetData | null; error: string };

export function useWidgetData(
  widgetName: string,
  itemId: string,
  refreshInterval: number | null,
): WidgetDataState {
  const [state, setState] = useState<WidgetDataState>({
    status: "loading",
    data: null,
    error: "",
  });

  useEffect(() => {
    const controller = new AbortController();

    const refresh = () => {
      void fetchWidgetData(widgetName, itemId, controller.signal)
        .then((widget) => {
          if (!controller.signal.aborted) {
            setState({ status: "ready", data: widget, error: "" });
          }
        })
        .catch((cause: unknown) => {
          if (!controller.signal.aborted) {
            setState((current) => ({
              status: "error",
              data: current.data,
              error: cause instanceof Error ? cause.message : "Widget data could not be loaded",
            }));
          }
        });
    };

    void refresh();
    let interval: number | undefined;
    if (refreshInterval && refreshInterval > 0) {
      interval = window.setInterval(refresh, refreshInterval * 1_000);
    }

    return () => {
      controller.abort();
      clearInterval(interval);
    };
  }, [widgetName, itemId, refreshInterval]);

  return state;
}
