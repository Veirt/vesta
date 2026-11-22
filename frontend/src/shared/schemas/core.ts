import * as v from "valibot";

export const ThemeSchema = v.picklist(["system", "light", "dark"]);

export const AppSettingsSchema = v.object({
  dashboard_title: v.string(),
  theme: ThemeSchema,
});

export const LoadSchema = v.object({
  one: v.number(),
  five: v.number(),
  fifteen: v.number(),
});

export const StatsSchema = v.object({
  cpu_usage_percent: v.number(),
  memory: v.object({
    used_bytes: v.number(),
    total_bytes: v.number(),
    usage_percent: v.number(),
  }),
  disk: v.object({
    mount: v.string(),
    used_bytes: v.number(),
    total_bytes: v.number(),
    usage_percent: v.number(),
  }),
  load: v.nullable(LoadSchema),
  uptime_secs: v.number(),
  collected_age_secs: v.number(),
});

export const NodeSchema = v.object({
  id: v.string(),
  name: v.string(),
  hostname: v.nullable(v.string()),
  device_name: v.nullable(v.string()),
  icon: v.nullable(v.string()),
  source: v.string(),
  interval_secs: v.number(),
  scrape_failed: v.boolean(),
  stats: v.nullable(StatsSchema),
});

export const NodeSettingsSchema = v.object({
  id: v.string(),
  name: v.string(),
  hostname: v.optional(v.nullable(v.string())),
  device_name: v.optional(v.nullable(v.string())),
  icon: v.optional(v.nullable(v.string())),
  endpoint: v.string(),
  interval_secs: v.number(),
});

export const ServiceSchema = v.object({
  kind: v.literal("service"),
  id: v.string(),
  title: v.string(),
  group: v.nullable(v.string()),
  icon: v.nullable(v.string()),
  href: v.nullable(v.string()),
  show_destination: v.optional(v.boolean(), true),
  x: v.nullable(v.number()),
  y: v.nullable(v.number()),
  width: v.nullable(v.number()),
  height: v.nullable(v.number()),
  has_ping: v.boolean(),
});

export const WidgetSchema = v.object({
  kind: v.literal("widget"),
  id: v.string(),
  title: v.string(),
  group: v.nullable(v.string()),
  name: v.string(),
  x: v.nullable(v.number()),
  y: v.nullable(v.number()),
  width: v.nullable(v.number()),
  height: v.nullable(v.number()),
  refresh_interval_secs: v.nullable(v.number()),
});

export const DashboardItemSchema = v.union([ServiceSchema, WidgetSchema]);

export const DashboardGroupSchema = v.object({
  id: v.string(),
  title: v.string(),
  description: v.optional(v.nullable(v.string())),
  icon: v.optional(v.nullable(v.string())),
  width: v.optional(v.nullable(v.number())),
});

export const DashboardDataSchema = v.object({
  nodes: v.array(NodeSchema),
  items: v.array(DashboardItemSchema),
  groups: v.optional(v.array(DashboardGroupSchema), []),
  app: AppSettingsSchema,
});

export const NodeEventSchema = v.object({
  id: v.string(),
  stats: v.nullable(StatsSchema),
  scrape_failed: v.boolean(),
});

export const ServiceSettingsSchema = v.object({
  kind: v.literal("service"),
  id: v.string(),
  title: v.string(),
  group: v.optional(v.nullable(v.string())),
  href: v.nullable(v.string()),
  show_destination: v.optional(v.boolean(), true),
  icon: v.optional(v.nullable(v.string())),
  x: v.optional(v.nullable(v.number())),
  y: v.optional(v.nullable(v.number())),
  width: v.optional(v.nullable(v.number())),
  height: v.optional(v.nullable(v.number())),
  ping: v.optional(v.nullable(v.object({ url: v.string() }))),
});

export const WidgetInstanceSettingsSchema = v.object({
  kind: v.literal("widget"),
  id: v.string(),
  title: v.string(),
  group: v.optional(v.nullable(v.string())),
  name: v.string(),
  config: v.optional(v.nullable(v.record(v.string(), v.unknown()))),
  x: v.optional(v.nullable(v.number())),
  y: v.optional(v.nullable(v.number())),
  width: v.optional(v.nullable(v.number())),
  height: v.optional(v.nullable(v.number())),
});

export const DashboardItemSettingsSchema = v.union([
  ServiceSettingsSchema,
  WidgetInstanceSettingsSchema,
]);

export const UploadedIconSchema = v.object({
  id: v.number(),
  name: v.string(),
  content_type: v.string(),
  url: v.string(),
});

export const WidgetOptionSchema = v.object({ value: v.string(), label: v.string() });

export const WidgetFieldSchema = v.object({
  key: v.string(),
  label: v.string(),
  kind: v.string(),
  required: v.boolean(),
  description: v.string(),
  default: v.optional(v.unknown()),
  options: v.optional(v.array(WidgetOptionSchema)),
  min: v.optional(v.number()),
  max: v.optional(v.number()),
});

export const WidgetDescriptorSchema = v.object({
  name: v.string(),
  title: v.string(),
  description: v.string(),
  default_width: v.number(),
  default_height: v.number(),
  min_width: v.number(),
  min_height: v.number(),
  fields: v.array(WidgetFieldSchema),
});

export const SettingsDataSchema = v.object({
  dashboard: v.object({
    nodes: v.array(NodeSettingsSchema),
    items: v.array(DashboardItemSettingsSchema),
    groups: v.optional(v.array(DashboardGroupSchema), []),
  }),
  app: AppSettingsSchema,
  widgets: v.array(WidgetDescriptorSchema),
});

export const PluginWidgetManifestSchema = v.object({
  id: v.string(),
  title: v.string(),
  description: v.optional(v.nullable(v.string())),
  entry: v.string(),
  width: v.number(),
  height: v.number(),
  refresh_interval_secs: v.optional(v.nullable(v.number())),
  fields: v.array(
    v.object({
      key: v.string(),
      label: v.string(),
      kind: v.string(),
      required: v.boolean(),
      description: v.optional(v.nullable(v.string())),
      default: v.optional(v.unknown()),
      options: v.optional(v.array(WidgetOptionSchema)),
    }),
  ),
});

export const PluginSchema = v.object({
  id: v.string(),
  name: v.string(),
  version: v.optional(v.nullable(v.string())),
  description: v.optional(v.nullable(v.string())),
  source_url: v.string(),
  commit: v.string(),
  installed_at: v.string(),
  widgets: v.array(PluginWidgetManifestSchema),
});

export type Theme = v.InferOutput<typeof ThemeSchema>;
export type AppSettings = v.InferOutput<typeof AppSettingsSchema>;
export type DashboardAppSettings = AppSettings;
export type Stats = v.InferOutput<typeof StatsSchema>;
export type DashboardNode = v.InferOutput<typeof NodeSchema> & { receivedAt: number | null };
export type NodeSettings = v.InferOutput<typeof NodeSettingsSchema>;
export type DashboardItem = v.InferOutput<typeof DashboardItemSchema>;
export type DashboardService = Extract<DashboardItem, { kind: "service" }>;
export type DashboardWidget = Extract<DashboardItem, { kind: "widget" }>;
export type DashboardGroup = v.InferOutput<typeof DashboardGroupSchema>;
export type DashboardData = v.InferOutput<typeof DashboardDataSchema>;
export type NodeEvent = v.InferOutput<typeof NodeEventSchema>;
export type ServiceSettings = v.InferOutput<typeof ServiceSettingsSchema>;
export type WidgetInstanceSettings = v.InferOutput<typeof WidgetInstanceSettingsSchema>;
export type DashboardItemSettings = v.InferOutput<typeof DashboardItemSettingsSchema>;
export type DashboardGroupSettings = DashboardGroup;
export type WidgetOption = v.InferOutput<typeof WidgetOptionSchema>;
export type WidgetField = v.InferOutput<typeof WidgetFieldSchema>;
export type WidgetDescriptor = v.InferOutput<typeof WidgetDescriptorSchema>;
export type SettingsData = v.InferOutput<typeof SettingsDataSchema>;
export type SettingsPayload = {
  dashboard: { nodes: NodeSettings[] };
  app: AppSettings;
};
export type Plugin = v.InferOutput<typeof PluginSchema>;
export type PluginWidget = v.InferOutput<typeof PluginWidgetManifestSchema>;
