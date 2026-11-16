import type { EditableDashboardItem } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

export type TileSize = { width: number; height: number };

/** Keep in sync with the group span clamps in `src/config_manager.rs`. */
export const DEFAULT_GROUP_SPAN = 6;
export const MIN_GROUP_SPAN = 3;

const UNKNOWN_WIDGET_MIN_SIZE: TileSize = { width: 3, height: 2 };
const MAX_GRID_SIZE = 12;

/**
 * Service links can shrink as their group grows: a tile of width w always
 * spans w/12 of the group's box, so in a small group 3 units keeps tiles
 * usable while a full-width group can host narrow link tiles.
 * Keep in sync with `service_min_size` in `src/config_manager.rs`.
 */
export function serviceMinSize(groupSpan: number | null | undefined): TileSize {
  const span = Math.max(
    MIN_GROUP_SPAN,
    Math.min(MAX_GRID_SIZE, Math.round(groupSpan ?? DEFAULT_GROUP_SPAN)),
  );
  if (span >= 10) return { width: 1, height: 1 };
  if (span >= 7) return { width: 2, height: 1 };
  return { width: 3, height: 1 };
}

export function widgetMinSize(widgets: WidgetDescriptor[], name: string): TileSize {
  const descriptor = widgets.find((widget) => widget.name === name);
  return {
    width: Math.max(1, Math.floor(descriptor?.min_width ?? UNKNOWN_WIDGET_MIN_SIZE.width)),
    height: Math.max(1, Math.floor(descriptor?.min_height ?? UNKNOWN_WIDGET_MIN_SIZE.height)),
  };
}

/** Column span of the group an item belongs to; ungrouped falls back to the default. */
export function groupSpanOf(
  groups: ReadonlyArray<{ id: string; width?: number | null }>,
  groupId: string | null | undefined,
): number {
  return groups.find((group) => group.id === (groupId ?? ""))?.width ?? DEFAULT_GROUP_SPAN;
}

export function minSizeFor(
  item: EditableDashboardItem,
  widgets: WidgetDescriptor[],
  groupSpan?: number | null,
): TileSize {
  return item.kind === "service" ? serviceMinSize(groupSpan) : widgetMinSize(widgets, item.name);
}

export function clampTileSize(width: number, height: number, min: TileSize): TileSize {
  return {
    width: Math.max(min.width, Math.min(MAX_GRID_SIZE, Math.round(width))),
    height: Math.max(min.height, Math.min(MAX_GRID_SIZE, Math.round(height))),
  };
}
