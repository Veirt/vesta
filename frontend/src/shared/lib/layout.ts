import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import type { WidgetDescriptor } from "@/shared/schemas/core";

export type TileSize = { width: number; height: number };

export const DEFAULT_GROUP_SPAN = 6;
const MIN_GROUP_SPAN = 3;

const UNKNOWN_WIDGET_MIN_SIZE: TileSize = { width: 3, height: 2 };
const HIDDEN_WIDGET_MIN_SIZES: Record<string, TileSize> = {
  SystemStats: { width: 4, height: 4 },
};
export const MAX_GRID_SIZE = 12;

export function serviceMinSize(groupSpan: number | null | undefined): TileSize {
  const span = clampGroupSpan(groupSpan);
  if (span >= 10) return { width: 1, height: 1 };
  if (span >= 7) return { width: 2, height: 1 };
  return { width: 3, height: 1 };
}

export function widgetMinSize(widgets: WidgetDescriptor[], name: string): TileSize {
  const descriptor = widgets.find((widget) => widget.name === name);
  const fallback = HIDDEN_WIDGET_MIN_SIZES[name] ?? UNKNOWN_WIDGET_MIN_SIZE;
  return {
    width: Math.max(1, Math.floor(descriptor?.min_width ?? fallback.width)),
    height: Math.max(1, Math.floor(descriptor?.min_height ?? fallback.height)),
  };
}

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

export type GroupableItem = { group: string | null; x: number | null; y: number | null };
export type GroupMetadata = {
  id: string;
  title: string;
  description?: string | null;
  icon?: string | null;
  width?: number | null;
};
export type DashboardGroupSection<T> = {
  id: string;
  label: string | null;
  description: string | null;
  icon: string | null;
  width: number;
  items: T[];
};
export type EditSection = DashboardGroupSection<EditableDashboardItem>;

export function clampGroupSpan(width: number | null | undefined): number {
  return Math.max(MIN_GROUP_SPAN, Math.min(MAX_GRID_SIZE, Math.round(width ?? DEFAULT_GROUP_SPAN)));
}

export const TILE_GRID_GAP = 12;
export const TILE_ROW_HEIGHT = 72;
export const TILE_MAX_SPAN = 12;

export function clampResizeUnits(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(Math.max(1, max), Math.round(value)));
}

export function gridUnitsFor(widthPx: number, gap: number, cellPx: number): number {
  return (widthPx + gap) / (cellPx + gap);
}

export type TileResizeState = {
  editorId: string;
  startX: number;
  startY: number;
  baseWidth: number;
  baseHeight: number;
  x: number;
  minWidth: number;
  minHeight: number;
  columnWidth: number;
  width: number;
  height: number;
};

export function groupDashboardItems<T extends GroupableItem>(
  items: T[],
  groups: GroupMetadata[] = [],
  includeEmptyGroups = false,
): DashboardGroupSection<T>[] {
  const ordered = [...items].sort(
    (left, right) => (left.y ?? 0) - (right.y ?? 0) || (left.x ?? 0) - (right.x ?? 0),
  );
  const metadata = new Map(groups.map((group) => [group.id, group]));
  const grouped = new Map<string, T[]>();
  for (const item of ordered) {
    const key = item.group?.trim() ?? "";
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  const sectionOrder: string[] = metadata.has("") ? [] : [""];
  const orderedIds = new Set(sectionOrder);
  for (const group of groups) {
    if (!orderedIds.has(group.id)) {
      orderedIds.add(group.id);
      sectionOrder.push(group.id);
    }
  }
  const seen = orderedIds;
  for (const key of grouped.keys()) {
    if (!seen.has(key)) sectionOrder.push(key);
  }
  const sections = sectionOrder.flatMap((id) => {
    const groupItems = grouped.get(id) ?? [];
    if (groupItems.length === 0 && !includeEmptyGroups) return [];
    const group = metadata.get(id);
    return [
      {
        id,
        label: id ? (group?.title ?? id) : null,
        description: id ? (group?.description ?? null) : null,
        icon: id ? (group?.icon ?? null) : null,
        width: clampGroupSpan(group?.width),
        items: groupItems,
      },
    ];
  });
  if (sections.length === 1 && sections[0].id === "") {
    return [{ ...sections[0], width: 12 }];
  }
  return sections;
}

export function eventClientY(event: Event): number | null {
  if ("clientY" in event && typeof event.clientY === "number") return event.clientY;
  if ("touches" in event) {
    const touchEvent = event as TouchEvent;
    return touchEvent.touches[0]?.clientY ?? touchEvent.changedTouches[0]?.clientY ?? null;
  }
  return null;
}
