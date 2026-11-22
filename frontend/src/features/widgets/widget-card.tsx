import type { ReactNode } from "react";
import type { DashboardWidget } from "@/shared/schemas/core";
import { WidgetError } from "@/features/widgets/builtins";
import { useWidgetData } from "@/features/widgets/use-widget-data";
import { WidgetContent } from "@/features/widgets/widget-content";
import { WidgetPreviewBody } from "@/features/widgets/widget-preview";
import { layoutStyle } from "@/features/widgets/widget-schemas";

function WidgetFrame({
  widget,
  editing,
  children,
}: {
  widget: DashboardWidget;
  editing: boolean;
  children: ReactNode;
}) {
  // The Jellyfin tile overlays the series title on its artwork, so the
  // dashboard heading would duplicate it.
  const showTitle = widget.name !== "JellyfinRandomShow";
  return (
    <article
      className={`widget-frame${editing ? " h-full" : " service-layout-item"}`}
      style={editing ? undefined : layoutStyle(widget)}
      aria-label={`${widget.title} widget`}
    >
      {showTitle && <h3 className="widget-card-title">{widget.title}</h3>}
      <div className="widget-card-content">{children}</div>
    </article>
  );
}

function LiveWidgetCard({ widget }: { widget: DashboardWidget }) {
  const state = useWidgetData(widget.name, widget.id, widget.refresh_interval_secs);
  return (
    <WidgetFrame widget={widget} editing={false}>
      {state.status === "error" ? (
        <WidgetError message={state.error} />
      ) : state.status === "loading" ? (
        <div className="widget-loading">Loading…</div>
      ) : (
        <WidgetContent name={widget.name} data={state.data} title={widget.title} />
      )}
    </WidgetFrame>
  );
}

export function WidgetCard({
  widget,
  editing = false,
}: {
  widget: DashboardWidget;
  editing?: boolean;
}) {
  if (editing) {
    return (
      <WidgetFrame widget={widget} editing>
        <WidgetPreviewBody name={widget.name} title={widget.title} />
      </WidgetFrame>
    );
  }
  return <LiveWidgetCard widget={widget} />;
}
