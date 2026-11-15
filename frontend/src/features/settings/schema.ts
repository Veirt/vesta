import * as v from "valibot";

const ThemeSchema = v.picklist(["system", "light", "dark"]);
const AppSettingsSchema = v.object({
  dashboard_title: v.string(),
  theme: ThemeSchema,
});
const NodeSettingsSchema = v.object({
  id: v.string(),
  name: v.string(),
  hostname: v.optional(v.nullable(v.string())),
  device_name: v.optional(v.nullable(v.string())),
  icon: v.optional(v.nullable(v.string())),
  endpoint: v.string(),
  interval_secs: v.number(),
});
const ServiceSettingsSchema = v.object({
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
const WidgetInstanceSettingsSchema = v.object({
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
const DashboardItemSettingsSchema = v.union([ServiceSettingsSchema, WidgetInstanceSettingsSchema]);
const DashboardGroupSettingsSchema = v.object({
  id: v.string(),
  title: v.string(),
  description: v.optional(v.nullable(v.string())),
  icon: v.optional(v.nullable(v.string())),
  width: v.optional(v.nullable(v.number())),
});
const UploadedIconSchema = v.object({
  id: v.number(),
  name: v.string(),
  content_type: v.string(),
  url: v.string(),
});
const WidgetOptionSchema = v.object({ value: v.string(), label: v.string() });
const WidgetFieldSchema = v.object({
  key: v.string(),
  label: v.string(),
  kind: v.string(),
  required: v.boolean(),
  description: v.string(),
  default: v.optional(v.unknown()),
  options: v.optional(v.array(WidgetOptionSchema)),
});
const WidgetDescriptorSchema = v.object({
  name: v.string(),
  description: v.string(),
  default_width: v.number(),
  default_height: v.number(),
  min_width: v.number(),
  min_height: v.number(),
  fields: v.array(WidgetFieldSchema),
});
const SettingsDataSchema = v.object({
  dashboard: v.object({
    nodes: v.array(NodeSettingsSchema),
    items: v.array(DashboardItemSettingsSchema),
    groups: v.optional(v.array(DashboardGroupSettingsSchema), []),
  }),
  app: AppSettingsSchema,
  widgets: v.array(WidgetDescriptorSchema),
});
const SettingsResponseSchema = v.object({
  status: v.literal("success"),
  data: SettingsDataSchema,
});
const AuthStatusResponseSchema = v.object({
  status: v.literal("success"),
  data: v.object({
    setup_required: v.boolean(),
    authenticated: v.boolean(),
  }),
});
const MessageResponseSchema = v.object({
  status: v.literal("success"),
  data: v.object({ message: v.string() }),
});
const PingTestResponseSchema = v.object({
  status: v.literal("success"),
  data: v.object({ up: v.boolean() }),
});
const IconUploadResponseSchema = v.object({
  status: v.literal("success"),
  data: UploadedIconSchema,
});
const ApiErrorSchema = v.object({ message: v.string() });

type Theme = v.InferOutput<typeof ThemeSchema>;
type AppSettings = v.InferOutput<typeof AppSettingsSchema>;
type NodeSettings = v.InferOutput<typeof NodeSettingsSchema>;
type ServiceSettings = v.InferOutput<typeof ServiceSettingsSchema>;
type WidgetInstanceSettings = v.InferOutput<typeof WidgetInstanceSettingsSchema>;
type DashboardItemSettings = v.InferOutput<typeof DashboardItemSettingsSchema>;
type DashboardGroupSettings = v.InferOutput<typeof DashboardGroupSettingsSchema>;
type WidgetDescriptor = v.InferOutput<typeof WidgetDescriptorSchema>;
type WidgetField = v.InferOutput<typeof WidgetFieldSchema>;
type SettingsData = v.InferOutput<typeof SettingsDataSchema>;
type SettingsPayload = {
  dashboard: { nodes: NodeSettings[] };
  app: AppSettings;
};
type AuthStatus = v.InferOutput<typeof AuthStatusResponseSchema>["data"];

export {
  ApiErrorSchema,
  AuthStatusResponseSchema,
  IconUploadResponseSchema,
  MessageResponseSchema,
  PingTestResponseSchema,
  SettingsResponseSchema,
};
export type {
  AppSettings,
  AuthStatus,
  DashboardItemSettings,
  DashboardGroupSettings,
  ServiceSettings,
  WidgetInstanceSettings,
  WidgetDescriptor,
  WidgetField,
  NodeSettings,
  SettingsData,
  SettingsPayload,
  Theme,
};
