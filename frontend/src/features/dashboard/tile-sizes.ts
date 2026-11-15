import type { EditableDashboardItem } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

export type TileSize = { width: number; height: number };

/** Keep these in sync with `SERVICE_MIN_SIZE` in `src/config_manager.rs`. */
/** Also kept server-side; see `DashboardGroup` in `src/config.rs`. */
export const DEFAULT_GROUP_SPAN = 6;
export const MIN_GROUP_SPAN = 3;

const SERVICE_MIN_SIZE: TileSize = { width: 3, height: 1 };
const UNKNOWN_WIDGET_MIN_SIZE: TileSize = { width: 3, height: 2 };
const MAX_GRID_SIZE = 12;

export function widgetMinSize(widgets: WidgetDescriptor[], name: string): TileSize {
  const descriptor = widgets.find((widget) => widget.name === name);
  return {
    width: Math.max(1, Math.floor(descriptor?.min_width ?? UNKNOWN_WIDGET_MIN_SIZE.width)),
    height: Math.max(1, Math.floor(descriptor?.min_height ?? UNKNOWN_WIDGET_MIN_SIZE.height)),
  };
}

export function minSizeFor(item: EditableDashboardItem, widgets: WidgetDescriptor[]): TileSize {
  return item.kind === "service" ? SERVICE_MIN_SIZE : widgetMinSize(widgets, item.name);
}

export function clampTileSize(width: number, height: number, min: TileSize): TileSize {
  return {
    width: Math.max(min.width, Math.min(MAX_GRID_SIZE, Math.round(width))),
    height: Math.max(min.height, Math.min(MAX_GRID_SIZE, Math.round(height))),
  };
}
