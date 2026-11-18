import type {
  DashboardGroupSettings,
  DashboardItemSettings,
  NodeSettings,
  SettingsPayload,
  WidgetDescriptor,
} from "@/shared/schemas/core";

export type EditableNode = Omit<NodeSettings, "hostname" | "device_name" | "icon"> & {
  hostname: string;
  device_name: string;
  icon: string;
  editor_id: string;
  id_touched: boolean;
};

export type EditableService = {
  kind: "service";
  editor_id: string;
  id: string;
  title: string;
  group: string | null;
  href: string;
  show_destination: boolean;
  icon: string;
  x: number;
  y: number;
  width: number;
  height: number;
  ping_url: string;
};

export type EditableWidget = {
  kind: "widget";
  editor_id: string;
  id: string;
  title: string;
  group: string | null;
  name: string;
  config: Record<string, unknown>;
  x: number;
  y: number;
  width: number;
  height: number;
};

export type EditableDashboardItem = EditableService | EditableWidget;

export type EditableDashboardGroup = {
  id: string;
  title: string;
  description: string;
  icon: string;
  width: number;
};

export function createEditorId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function createItemId(value: string, fallback: string) {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  return `${normalized || fallback}-${Math.random().toString(36).slice(2, 8)}`;
}

export function toEditableItem(item: DashboardItemSettings): EditableDashboardItem {
  const editorId = createEditorId();
  const layout = {
    x: item.x ?? 0,
    y: item.y ?? 0,
    width: item.width ?? 4,
    height: item.height ?? (item.kind === "service" ? 1 : 2),
  };
  if (item.kind === "service") {
    return {
      kind: "service",
      editor_id: editorId,
      id: item.id,
      title: item.title,
      group: item.group ?? null,
      href: item.href ?? "",
      show_destination: item.show_destination ?? true,
      icon: item.icon ?? "",
      ping_url: item.ping?.url ?? "",
      ...layout,
    };
  }
  return {
    kind: "widget",
    editor_id: editorId,
    id: item.id,
    title: item.title,
    group: item.group ?? null,
    name: item.name,
    config: item.config ?? {},
    ...layout,
  };
}

const DEFAULT_GROUP_WIDTH = 6;

export function toEditableGroup(group: DashboardGroupSettings): EditableDashboardGroup {
  const width = group.width ?? DEFAULT_GROUP_WIDTH;
  return {
    id: group.id,
    title: group.title,
    description: group.description ?? "",
    icon: group.icon ?? "",
    width: Math.max(1, Math.min(12, Math.round(width))),
  };
}

export function serializeDashboardItems(
  items: EditableDashboardItem[],
  widgetDescriptors: WidgetDescriptor[],
): DashboardItemSettings[] {
  return items.map((item) => {
    if (item.kind === "service") {
      return {
        kind: "service",
        id: item.id.trim(),
        title: item.title.trim(),
        group: item.group?.trim() || null,
        href: item.href.trim() || null,
        show_destination: item.show_destination,
        icon: item.icon.trim() || null,
        x: item.x,
        y: item.y,
        width: item.width,
        height: item.height,
        ping: item.ping_url.trim() ? { url: item.ping_url.trim() } : null,
      };
    }
    const descriptor = widgetDescriptors.find(({ name }) => name === item.name);
    const acceptedKeys = new Set(descriptor?.fields.map(({ key }) => key) ?? []);
    const config = Object.fromEntries(
      Object.entries(item.config).filter(([key]) => acceptedKeys.has(key)),
    );
    return {
      kind: "widget",
      id: item.id.trim(),
      title: item.title.trim(),
      group: item.group?.trim() || null,
      name: item.name,
      config: Object.keys(config).length === 0 ? null : config,
      x: item.x,
      y: item.y,
      width: item.width,
      height: item.height,
    };
  });
}

export function serializeDashboardGroups(
  groups: EditableDashboardGroup[],
): DashboardGroupSettings[] {
  return groups.map((group) => ({
    id: group.id,
    title: group.title.trim(),
    description: group.description.trim() || null,
    icon: group.icon.trim() || null,
    width: Math.max(1, Math.min(12, Math.round(group.width))),
  }));
}

export function createSettingsPayload(
  nodes: EditableNode[],
  app: SettingsPayload["app"],
): SettingsPayload {
  return {
    dashboard: {
      nodes: nodes.map((node) => ({
        id: node.id.trim(),
        name: node.name.trim(),
        hostname: node.hostname?.trim() || null,
        device_name: node.device_name?.trim() || null,
        icon: node.icon?.trim() || null,
        endpoint: node.endpoint.trim(),
        interval_secs: node.interval_secs,
      })),
    },
    app,
  };
}

export function slug(value: string, fallback: string) {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return normalized || fallback;
}
