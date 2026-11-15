import { useEffect, useState } from "react";
import {
  createItemId,
  serializeDashboardGroups,
  serializeDashboardItems,
  toEditableGroup,
  toEditableItem,
} from "@/features/settings/models";
import type {
  EditableDashboardGroup,
  EditableDashboardItem,
  EditableService,
  EditableWidget,
} from "@/features/settings/models";
import {
  clampTileSize,
  DEFAULT_GROUP_SPAN,
  MIN_GROUP_SPAN,
  minSizeFor,
  widgetMinSize,
} from "@/features/dashboard/tile-sizes";
import { fetchSettings, saveDashboardItems, uploadIcon } from "@/features/settings/api";
import type { NodeSettings, WidgetDescriptor } from "@/features/settings/schema";

type LayoutEntry = { i: string; x: number; y: number; w: number; h: number };

function occupies(
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
function compactY(items: EditableDashboardItem[]): EditableDashboardItem[] {
  const placed: EditableDashboardItem[] = [];
  const sorted = [...items].sort((left, right) => left.y - right.y || left.x - right.x);
  for (const item of sorted) {
    let y = item.y;
    let candidate = { ...item, y };
    while (y > 0 && !placed.some((other) => occupies({ ...candidate, y: y - 1 }, other))) {
      y -= 1;
    }
    for (;;) {
      const hit = placed.find((other) => occupies({ ...candidate, y }, other));
      if (!hit) break;
      y = hit.y + hit.height;
    }
    placed.push({ ...item, y });
  }
  const positions = new Map(placed.map((item) => [item.editor_id, item]));
  return items.map((item) => positions.get(item.editor_id) ?? item);
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

function sortItems(items: EditableDashboardItem[]) {
  return [...items].sort((left, right) => left.y - right.y || left.x - right.x);
}

function reflowGroup(items: EditableDashboardItem[]) {
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

function useDashboardEditor(enabled: boolean) {
  const [items, setItems] = useState<EditableDashboardItem[]>([]);
  const [groups, setGroups] = useState<EditableDashboardGroup[]>([]);
  const [nodes, setNodes] = useState<NodeSettings[]>([]);
  const [widgets, setWidgets] = useState<WidgetDescriptor[]>([]);
  const [baseline, setBaseline] = useState("");
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState("");
  const [loadRevision, setLoadRevision] = useState(0);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  useEffect(() => {
    if (!enabled) {
      setBaseline("");
      return;
    }
    let active = true;
    setLoading(true);
    setLoadError("");
    fetchSettings()
      .then((data) => {
        if (!active) return;
        const editableItems = data.dashboard.items.map(toEditableItem);
        const editableGroups = data.dashboard.groups.map(toEditableGroup);
        const knownGroups = new Set(editableGroups.map((group) => group.id));
        for (const item of editableItems) {
          if (item.group && !knownGroups.has(item.group)) {
            knownGroups.add(item.group);
            editableGroups.push({
              id: item.group,
              title: item.group,
              description: "",
              icon: "",
              width: DEFAULT_GROUP_SPAN,
            });
          }
        }
        const compactedItems = compactY(editableItems);
        setItems(compactedItems);
        setGroups(editableGroups);
        setNodes(data.dashboard.nodes);
        setWidgets(data.widgets);
        setBaseline(
          JSON.stringify({
            items: serializeDashboardItems(compactedItems, data.widgets),
            groups: serializeDashboardGroups(editableGroups),
          }),
        );
      })
      .catch((cause: unknown) => {
        if (active) {
          setLoadError(
            cause instanceof Error ? cause.message : "Dashboard settings could not be loaded.",
          );
        }
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [enabled, loadRevision]);

  const serializedItems = serializeDashboardItems(items, widgets);
  const serializedGroups = serializeDashboardGroups(groups);
  const serializedDraft = JSON.stringify({ items: serializedItems, groups: serializedGroups });
  const dirty = baseline !== "" && serializedDraft !== baseline;

  function addGroup(requestedTitle?: string) {
    const requested = requestedTitle?.trim();
    if (requested) {
      const existing = groups.find(
        (group) => group.title.trim().toLocaleLowerCase() === requested.toLocaleLowerCase(),
      );
      if (existing) return existing.id;
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
    setGroups((current) => [
      ...current,
      { id, title, description: "", icon: "", width: DEFAULT_GROUP_SPAN },
    ]);
    setNotice("");
    return id;
  }

  function updateGroup(id: string, update: Partial<EditableDashboardGroup>) {
    setGroups((current) =>
      current.map((group) => (group.id === id ? { ...group, ...update } : group)),
    );
    setNotice("");
  }

  function removeGroup(id: string) {
    setGroups((current) => current.filter((group) => group.id !== id));
    setItems((current) => {
      const moving = current
        .filter((item) => item.group === id)
        .map((item) => ({ ...item, group: null }));
      if (moving.length === 0) return current;
      const remaining = current.filter((item) => item.group !== id);
      const ungrouped = reflowGroup(
        sortItems([...remaining.filter((item) => !item.group), ...moving]),
      );
      const positions = new Map(ungrouped.map((item) => [item.editor_id, item]));
      return remaining.map((item) => positions.get(item.editor_id) ?? item);
    });
    setNotice("");
  }

  function addService() {
    const position = nextPosition(items, null);
    const service: EditableService = {
      kind: "service",
      editor_id: createItemId("editor-service", "service"),
      id: nextId("service", items),
      title: "New service",
      group: null,
      href: "",
      show_destination: true,
      icon: "",
      ping_url: "",
      width: 4,
      height: 1,
      ...position,
    };
    setItems((current) => [...current, service]);
    setNotice("");
    return service.editor_id;
  }

  function addWidget(name: string) {
    const descriptor = widgets.find((widget) => widget.name === name);
    if (!descriptor) return null;
    const min = widgetMinSize(widgets, name);
    const position = nextPosition(items, null);
    const widget: EditableWidget = {
      kind: "widget",
      editor_id: createItemId("editor-widget", "widget"),
      id: nextId(name, items),
      title: name.replace(/([a-z])([A-Z])/g, "$1 $2"),
      group: null,
      name,
      config: defaultConfig(descriptor),
      width: Math.max(min.width, descriptor.default_width),
      height: Math.max(min.height, descriptor.default_height),
      ...position,
    };
    setItems((current) => [...current, widget]);
    setNotice("");
    return widget.editor_id;
  }

  function updateItem(editorId: string, update: object) {
    setItems((current) =>
      current.map((item) =>
        item.editor_id === editorId ? ({ ...item, ...update } as EditableDashboardItem) : item,
      ),
    );
    setNotice("");
  }

  function removeItem(editorId: string) {
    setItems((current) => current.filter((item) => item.editor_id !== editorId));
    setNotice("");
  }

  function moveItem(editorId: string, direction: -1 | 1) {
    setItems((current) => {
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
    });
    setNotice("");
  }

  function resizeItem(editorId: string, width: number, height: number) {
    setItems((current) => {
      const selected = current.find((item) => item.editor_id === editorId);
      if (!selected) return current;
      const min = minSizeFor(selected, widgets);
      const fixed = clampTileSize(width, height, min);
      const updated = current.map((item) =>
        item.editor_id === editorId
          ? {
              ...item,
              width: fixed.width,
              height: fixed.height,
              x: Math.min(item.x, 12 - fixed.width),
            }
          : item,
      );
      const reflowed = reflowGroup(sortItems(updated.filter((item) => sameGroup(item, selected))));
      const positions = new Map(reflowed.map((item) => [item.editor_id, item]));
      return updated.map((item) => positions.get(item.editor_id) ?? item);
    });
    setNotice("");
  }

  function moveGroup(id: string, direction: -1 | 1) {
    setGroups((current) => {
      const from = current.findIndex((group) => group.id === id);
      const to = from + direction;
      if (from < 0 || to < 0 || to >= current.length) return current;
      const next = [...current];
      [next[from], next[to]] = [next[to], next[from]];
      return next;
    });
    setNotice("");
  }

  function moveGroupTo(id: string, targetId: string, placement: "before" | "after") {
    setGroups((current) => {
      const from = current.findIndex((group) => group.id === id);
      if (from < 0) return current;
      const next = [...current];
      const [moved] = next.splice(from, 1);
      let to = next.findIndex((group) => group.id === targetId);
      if (to < 0) return current.map((group, index) => (index === from ? moved : group));
      if (placement === "after") to += 1;
      next.splice(to, 0, moved);
      return next;
    });
    setNotice("");
  }

  /**
   * The Ungrouped section is implicit (tiles with `group: null`), so ordering and
   * width need a metadata slot; this id-less entry carries them and is persisted
   * with the rest of the groups.
   */
  function ensureUngroupedGroup() {
    setGroups((current) =>
      current.some((group) => group.id === "")
        ? current
        : [{ id: "", title: "", description: "", icon: "", width: DEFAULT_GROUP_SPAN }, ...current],
    );
  }

  function setGroupWidth(id: string, width: number) {
    setGroups((current) =>
      current.map((group) =>
        group.id === id
          ? { ...group, width: Math.max(MIN_GROUP_SPAN, Math.min(12, Math.round(width))) }
          : group,
      ),
    );
    setNotice("");
  }

  function moveItemToGroup(editorId: string, group: string | null) {
    if (group) {
      setGroups((current) =>
        current.some((entry) => entry.id === group)
          ? current
          : [
              ...current,
              { id: group, title: group, description: "", icon: "", width: DEFAULT_GROUP_SPAN },
            ],
      );
    }
    setItems((current) => {
      const selected = current.find((item) => item.editor_id === editorId);
      if (!selected || (selected.group ?? "") === (group ?? "")) return current;
      const position = nextPosition(current, group);
      const updated = current.map((item) =>
        item.editor_id === editorId ? { ...item, group, ...position } : item,
      );
      const source = reflowGroup(
        sortItems(
          updated.filter((item) => item.editor_id !== editorId && sameGroup(item, selected)),
        ),
      );
      const destination = reflowGroup(
        sortItems(updated.filter((item) => sameGroup(item, { ...selected, group }))),
      );
      const positions = new Map([...source, ...destination].map((item) => [item.editor_id, item]));
      return updated.map((item) => positions.get(item.editor_id) ?? item);
    });
    setNotice("");
  }

  function updateLayout(layout: LayoutEntry[], cols: number) {
    if (cols === 1) return;
    const factor = 12 / cols;
    const byId = new Map(layout.map((entry) => [entry.i, entry]));
    setItems((current) => {
      let changed = false;
      const next = current.map((item) => {
        const entry = byId.get(item.editor_id);
        if (!entry) return item;
        const min = minSizeFor(item, widgets);
        const x = Math.round(entry.x * factor);
        const y = entry.y;
        const size = clampTileSize(entry.w * factor, entry.h, min);
        if (
          item.x === x &&
          item.y === y &&
          item.width === size.width &&
          item.height === size.height
        )
          return item;
        changed = true;
        return { ...item, x, y, width: size.width, height: size.height };
      });
      return changed ? next : current;
    });
    setNotice("");
  }

  async function uploadImage(file: File): Promise<string | null> {
    setUploading(true);
    setError("");
    try {
      const uploaded = await uploadIcon(file);
      setNotice(`${uploaded.name} uploaded. Save the dashboard to use it.`);
      return uploaded.url;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The image could not be uploaded.");
      return null;
    } finally {
      setUploading(false);
    }
  }

  async function save() {
    setSaving(true);
    setError("");
    setNotice("");
    try {
      await saveDashboardItems(serializedItems, serializedGroups);
      setBaseline(serializedDraft);
      setNotice("Dashboard saved.");
      return true;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Dashboard could not be saved.");
      return false;
    } finally {
      setSaving(false);
    }
  }

  function retryLoad() {
    setLoading(true);
    setLoadError("");
    setLoadRevision((revision) => revision + 1);
  }

  return {
    dirty,
    error,
    items,
    groups,
    loadError,
    loading,
    ready: baseline !== "",
    nodes,
    notice,
    saving,
    uploading,
    widgets,
    addService,
    addGroup,
    addWidget,
    ensureUngroupedGroup,
    moveGroup,
    moveGroupTo,
    moveItem,
    moveItemToGroup,
    removeItem,
    removeGroup,
    resizeItem,
    retryLoad,
    save,
    setGroupWidth,
    updateItem,
    updateGroup,
    updateLayout,
    uploadImage,
  };
}

export { useDashboardEditor, sortItems };
export type { LayoutEntry };
