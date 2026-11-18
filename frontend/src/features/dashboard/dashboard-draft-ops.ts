import {
  createItemId,
  type EditableDashboardGroup,
  type EditableDashboardItem,
  type EditableService,
  type EditableWidget,
} from "@/shared/lib/draft-models";
import {
  clampTileSize,
  DEFAULT_GROUP_SPAN,
  groupSpanOf,
  MAX_GRID_SIZE,
  MIN_GROUP_SPAN,
  minSizeFor,
  widgetMinSize,
} from "@/shared/lib/layout";
import type { WidgetDescriptor } from "@/shared/schemas/core";

export type LayoutEntry = { i: string; x: number; y: number; w: number; h: number };

export function occupies(
  a: { x: number; y: number; width: number; height: number },
  b: { x: number; y: number; width: number; height: number },
) {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}

/**
 * Matches react-grid-layout's `verticalCompactor`: keeps each tile's column but
 * pulls it up into the topmost free row, so saved layouts never carry empty
 * gaps and the rendered grid agrees with the stored baseline.
 */
export function compactY(items: EditableDashboardItem[]) {
  const byGroup = new Map<string, EditableDashboardItem[]>();
  for (const item of items) {
    const key = item.group ?? "";
    byGroup.set(key, [...(byGroup.get(key) ?? []), item]);
  }
  const positions = new Map<string, EditableDashboardItem>();
  for (const groupItems of byGroup.values()) {
    const placed: EditableDashboardItem[] = [];
    const sorted = [...groupItems].sort((left, right) => left.y - right.y || left.x - right.x);
    for (const item of sorted) {
      let y = item.y;
      const candidate = { ...item, y };
      while (y > 0 && !placed.some((other) => occupies({ ...candidate, y: y - 1 }, other))) {
        y -= 1;
      }
      for (;;) {
        const hit = placed.find((other) => occupies({ ...candidate, y }, other));
        if (!hit) break;
        y = hit.y + hit.height;
      }
      const settled = { ...item, y };
      placed.push(settled);
      positions.set(item.editor_id, settled);
    }
  }
  return items.map((item) => positions.get(item.editor_id) ?? item);
}

export function sortItems(items: EditableDashboardItem[]) {
  return [...items].sort((left, right) => left.y - right.y || left.x - right.x);
}

export function reflowGroup(items: EditableDashboardItem[]) {
  let x = 0;
  let y = 0;
  let rowHeight = 0;
  return items.map((item) => {
    if (x + item.width > 12) {
      x = 0;
      y += rowHeight;
      rowHeight = 0;
    }
    const placed = { ...item, x, y };
    x += item.width;
    rowHeight = Math.max(rowHeight, item.height);
    return placed;
  });
}

function sameGroup(left: EditableDashboardItem, right: EditableDashboardItem) {
  return (left.group ?? "") === (right.group ?? "");
}

function nextId(base: string, items: EditableDashboardItem[]) {
  const candidate = createItemId(base, "tile");
  const ids = new Set(items.map((item) => item.id));
  if (!ids.has(candidate)) return candidate;
  let suffix = 2;
  while (ids.has(`${candidate}-${suffix}`)) suffix += 1;
  return `${candidate}-${suffix}`;
}

function defaultConfig(descriptor: WidgetDescriptor) {
  return Object.fromEntries(
    descriptor.fields
      .filter((field) => field.default !== undefined)
      .map((field) => [field.key, field.default]),
  );
}

function nextPosition(items: EditableDashboardItem[], group: string | null) {
  const bottom = items
    .filter((item) => (item.group ?? "") === (group ?? ""))
    .reduce((value, item) => Math.max(value, item.y + item.height), 0);
  return { x: 0, y: bottom };
}

/**
 * Largest size the tile can grow to at its current position without touching a
 * neighbour. Growing further would force react-grid-layout's vertical
 * compactor to evict the blocking tile into a lower row (and shuffle the rest
 * of the group), so resizes stop at this boundary instead.
 */
function maxFreeSize(
  item: EditableDashboardItem,
  neighbours: EditableDashboardItem[],
): { width: number; height: number } {
  let width = MAX_GRID_SIZE - item.x;
  let height = MAX_GRID_SIZE;
  for (const other of neighbours) {
    const rowsOverlap = other.y < item.y + item.height && other.y + other.height > item.y;
    if (rowsOverlap && other.x >= item.x + item.width) {
      width = Math.min(width, other.x - item.x);
    }
    const colsOverlap = other.x < item.x + item.width && other.x + other.width > item.x;
    if (colsOverlap && other.y >= item.y + item.height) {
      height = Math.min(height, other.y - item.y);
    }
  }
  return { width, height };
}

export function ensureKnownGroups(
  editableItems: EditableDashboardItem[],
  editableGroups: EditableDashboardGroup[],
): EditableDashboardGroup[] {
  const knownGroups = new Set(editableGroups.map((group) => group.id));
  const result = [...editableGroups];
  for (const item of editableItems) {
    if (item.group && !knownGroups.has(item.group)) {
      knownGroups.add(item.group);
      result.push({
        id: item.group,
        title: item.group,
        description: "",
        icon: "",
        width: DEFAULT_GROUP_SPAN,
      });
    }
  }
  return result;
}

export function createGroupEntry(
  groups: EditableDashboardGroup[],
  requestedTitle?: string,
): { id: string; nextGroups: EditableDashboardGroup[] } {
  const requested = requestedTitle?.trim();
  if (requested) {
    const existing = groups.find(
      (group) => group.title.trim().toLocaleLowerCase() === requested.toLocaleLowerCase(),
    );
    if (existing) return { id: existing.id, nextGroups: groups };
  }
  const existingTitles = new Set(groups.map((group) => group.title.trim().toLocaleLowerCase()));
  let title = requested || "New group";
  let number = 2;
  while (!requested && existingTitles.has(title.toLocaleLowerCase())) {
    title = `New group ${number}`;
    number += 1;
  }
  const existingIds = new Set(groups.map((group) => group.id));
  let id = createItemId(title, "group");
  while (existingIds.has(id)) id = createItemId(title, "group");
  return {
    id,
    nextGroups: [...groups, { id, title, description: "", icon: "", width: DEFAULT_GROUP_SPAN }],
  };
}

export function createServiceDraft(
  items: EditableDashboardItem[],
  group?: string | null,
): EditableService {
  const position = nextPosition(items, group ?? null);
  return {
    kind: "service",
    editor_id: createItemId("editor-service", "service"),
    id: nextId("service", items),
    title: "New service",
    group: group ?? null,
    href: "",
    show_destination: true,
    icon: "",
    ping_url: "",
    width: 4,
    height: 1,
    ...position,
  };
}

export function createWidgetDraft(
  items: EditableDashboardItem[],
  widgets: WidgetDescriptor[],
  name: string,
  group?: string | null,
): EditableWidget | null {
  const descriptor = widgets.find((widget) => widget.name === name);
  if (!descriptor) return null;
  const min = widgetMinSize(widgets, name);
  const position = nextPosition(items, group ?? null);
  return {
    kind: "widget",
    editor_id: createItemId("editor-widget", "widget"),
    id: nextId(name, items),
    title: descriptor.title || name,
    group: group ?? null,
    name,
    config: defaultConfig(descriptor),
    width: Math.max(min.width, descriptor.default_width),
    height: Math.max(min.height, descriptor.default_height),
    ...position,
  };
}

export function removeGroupFromItems(
  current: EditableDashboardItem[],
  id: string,
): EditableDashboardItem[] {
  const moving = current
    .filter((item) => item.group === id)
    .map((item) => ({ ...item, group: null }));
  if (moving.length === 0) return current;
  const remaining = current.filter((item) => item.group !== id);
  const ungrouped = reflowGroup(sortItems([...remaining.filter((item) => !item.group), ...moving]));
  const positions = new Map(ungrouped.map((item) => [item.editor_id, item]));
  return [...remaining, ...moving].map((item) => positions.get(item.editor_id) ?? item);
}

export function moveItemDraft(
  current: EditableDashboardItem[],
  editorId: string,
  direction: -1 | 1,
): EditableDashboardItem[] {
  const selected = current.find((item) => item.editor_id === editorId);
  if (!selected) return current;
  const ordered = sortItems(current.filter((item) => sameGroup(item, selected)));
  const index = ordered.findIndex((item) => item.editor_id === editorId);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= ordered.length) return current;
  [ordered[index], ordered[target]] = [ordered[target], ordered[index]];
  const reflowed = reflowGroup(ordered);
  const positions = new Map(reflowed.map((item) => [item.editor_id, item]));
  return current.map((item) => positions.get(item.editor_id) ?? item);
}

export function resizeItemDraft(
  current: EditableDashboardItem[],
  groups: EditableDashboardGroup[],
  widgets: WidgetDescriptor[],
  editorId: string,
  width: number,
  height: number,
): EditableDashboardItem[] {
  const selected = current.find((item) => item.editor_id === editorId);
  if (!selected) return current;
  const min = minSizeFor(selected, widgets, groupSpanOf(groups, selected.group));
  const neighbours = current.filter(
    (item) => item.editor_id !== editorId && sameGroup(item, selected),
  );
  const free = maxFreeSize(selected, neighbours);
  const fixed = clampTileSize(Math.min(width, free.width), Math.min(height, free.height), min);
  if (selected.width === fixed.width && selected.height === fixed.height) return current;
  return current.map((item) =>
    item.editor_id === editorId
      ? {
          ...item,
          width: fixed.width,
          height: fixed.height,
          x: Math.min(item.x, 12 - fixed.width),
        }
      : item,
  );
}

export function moveGroupDraft(
  current: EditableDashboardGroup[],
  id: string,
  direction: -1 | 1,
): EditableDashboardGroup[] {
  const from = current.findIndex((group) => group.id === id);
  const to = from + direction;
  if (from < 0 || to < 0 || to >= current.length) return current;
  const next = [...current];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

export function moveGroupToDraft(
  current: EditableDashboardGroup[],
  id: string,
  targetId: string,
  placement: "before" | "after",
): EditableDashboardGroup[] {
  const from = current.findIndex((group) => group.id === id);
  if (from < 0) return current;
  const next = [...current];
  const [moved] = next.splice(from, 1);
  let to = next.findIndex((group) => group.id === targetId);
  if (to < 0) return current.map((group, index) => (index === from ? moved : group));
  if (placement === "after") to += 1;
  next.splice(to, 0, moved);
  return next;
}

export function moveItemToGroupDraft(
  current: EditableDashboardItem[],
  editorId: string,
  group: string | null,
): EditableDashboardItem[] {
  const selected = current.find((item) => item.editor_id === editorId);
  if (!selected || (selected.group ?? "") === (group ?? "")) return current;
  const position = nextPosition(current, group);
  const updated = current.map((item) =>
    item.editor_id === editorId ? { ...item, group, ...position } : item,
  );
  const source = reflowGroup(
    sortItems(updated.filter((item) => item.editor_id !== editorId && sameGroup(item, selected))),
  );
  const destination = reflowGroup(
    sortItems(updated.filter((item) => sameGroup(item, { ...selected, group }))),
  );
  const positions = new Map([...source, ...destination].map((item) => [item.editor_id, item]));
  return updated.map((item) => positions.get(item.editor_id) ?? item);
}

export function applyLayoutDraft(
  current: EditableDashboardItem[],
  groups: EditableDashboardGroup[],
  widgets: WidgetDescriptor[],
  layout: LayoutEntry[],
  cols: number,
): EditableDashboardItem[] {
  if (cols === 1) return current;
  const factor = 12 / cols;
  const byId = new Map(layout.map((entry) => [entry.i, entry]));
  let changed = false;
  const next = current.map((item) => {
    const entry = byId.get(item.editor_id);
    if (!entry) return item;
    const min = minSizeFor(item, widgets, groupSpanOf(groups, item.group));
    const x = Math.round(entry.x * factor);
    const y = entry.y;
    const size = clampTileSize(entry.w * factor, entry.h, min);
    if (item.x === x && item.y === y && item.width === size.width && item.height === size.height)
      return item;
    changed = true;
    return { ...item, x, y, width: size.width, height: size.height };
  });
  return changed ? next : current;
}

export function clampGroupSpanUpdate(width: number): number {
  return Math.max(MIN_GROUP_SPAN, Math.min(12, Math.round(width)));
}
