import { IconPencil } from "@tabler/icons-react";
import { ServiceCard } from "@/features/widgets/service-card";
import { WidgetCard } from "@/features/widgets/widget-card";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import type { DashboardService, DashboardWidget } from "@/shared/schemas/core";

function asService(item: Extract<EditableDashboardItem, { kind: "service" }>): DashboardService {
  return {
    kind: "service",
    id: item.id,
    title: item.title,
    group: item.group,
    icon: item.icon || null,
    href: item.href || null,
    show_destination: item.show_destination,
    x: item.x,
    y: item.y,
    width: item.width,
    height: item.height,
    has_ping: Boolean(item.ping_url.trim()),
  };
}

function asWidget(item: Extract<EditableDashboardItem, { kind: "widget" }>): DashboardWidget {
  return {
    kind: "widget",
    id: item.id,
    title: item.title,
    group: item.group,
    name: item.name,
    x: item.x,
    y: item.y,
    width: item.width,
    height: item.height,
    refresh_interval_secs: null,
  };
}

export function EditTile({ item, onEdit }: { item: EditableDashboardItem; onEdit: () => void }) {
  return (
    <div className="dashboard-edit-tile">
      <button
        className="dashboard-tile-edit-button"
        type="button"
        aria-label={`Edit ${item.title}`}
        onClick={onEdit}
      >
        <IconPencil size={16} aria-hidden="true" />
      </button>
      {item.kind === "service" ? (
        <ServiceCard service={asService(item)} editing />
      ) : (
        <WidgetCard widget={asWidget(item)} editing />
      )}
    </div>
  );
}
