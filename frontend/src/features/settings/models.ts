import type {
  GroupSettings,
  NodeSettings,
  ServiceSettings,
  SettingsPayload,
  WidgetDescriptor,
} from "@/features/settings/schema";

type EditableNode = NodeSettings & { editor_id: string };

type EditableService = {
  editor_id: string;
  title: string;
  href: string;
  icon: string;
  x: number;
  y: number;
  width: number;
  height: number;
  ping_url: string;
  widget_name: string;
  widget_config: Record<string, unknown>;
};

type EditableGroup = Omit<GroupSettings, "services"> & {
  editor_id: string;
  services: EditableService[];
};

function createEditorId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function toEditableService(service: ServiceSettings, editorId = createEditorId()): EditableService {
  const widgetConfig = service.widget?.config ?? {};
  return {
    editor_id: editorId,
    title: service.title,
    href: service.href ?? "",
    icon: service.icon ?? "",
    x: service.x ?? 0,
    y: service.y ?? 0,
    width: service.width ?? 4,
    height: service.height ?? (service.widget ? 3 : 1),
    ping_url: service.ping?.url ?? "",
    widget_name: service.widget?.name ?? "",
    widget_config: widgetConfig,
  };
}

function createSettingsPayload(
  nodes: EditableNode[],
  groups: EditableGroup[],
  app: SettingsPayload["app"],
  widgetDescriptors: WidgetDescriptor[],
): SettingsPayload {
  return {
    dashboard: {
      nodes: nodes.map((node) => ({
        id: node.id,
        name: node.name,
        hostname: node.hostname?.trim() || null,
        device_name: node.device_name?.trim() || null,
        icon: node.icon?.trim() || null,
        endpoint: node.endpoint,
        interval_secs: node.interval_secs,
      })),
      groups: groups.map((group) => ({
        id: group.id.trim(),
        name: group.name,
        services: group.services.map((service) => {
          let widget: SettingsPayload["dashboard"]["groups"][number]["services"][number]["widget"] =
            null;
          if (service.widget_name) {
            const descriptor = widgetDescriptors.find(({ name }) => name === service.widget_name);
            const acceptedKeys = new Set(descriptor?.fields.map(({ key }) => key) ?? []);
            const configValue = Object.fromEntries(
              Object.entries(service.widget_config).filter(([key]) => acceptedKeys.has(key)),
            );
            widget = {
              name: service.widget_name,
              config: Object.keys(configValue).length === 0 ? null : configValue,
            };
          }
          return {
            title: service.title,
            href: service.href.trim() || null,
            icon: service.icon.trim() || null,
            x: service.x,
            y: service.y,
            width: service.width,
            height: service.height,
            ping: service.ping_url.trim() ? { url: service.ping_url.trim() } : null,
            widget,
          };
        }),
      })),
    },
    app,
  };
}

function slug(value: string, fallback: string) {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return normalized || fallback;
}

export { createEditorId, createSettingsPayload, slug, toEditableService };
export type { EditableGroup, EditableNode, EditableService };
