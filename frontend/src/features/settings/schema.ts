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
const WidgetSettingsSchema = v.object({
  name: v.string(),
  config: v.nullable(v.record(v.string(), v.unknown())),
});
const ServiceSettingsSchema = v.object({
  title: v.string(),
  href: v.nullable(v.string()),
  icon: v.optional(v.nullable(v.string())),
  x: v.optional(v.nullable(v.number())),
  y: v.optional(v.nullable(v.number())),
  width: v.nullable(v.number()),
  height: v.nullable(v.number()),
  widget: v.nullable(WidgetSettingsSchema),
  ping: v.nullable(v.object({ url: v.string() })),
});
const GroupSettingsSchema = v.object({
  id: v.string(),
  name: v.string(),
  services: v.array(ServiceSettingsSchema),
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
  fields: v.array(WidgetFieldSchema),
});
const SettingsDataSchema = v.object({
  dashboard: v.object({
    nodes: v.array(NodeSettingsSchema),
    groups: v.array(GroupSettingsSchema),
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
const IconUploadResponseSchema = v.object({
  status: v.literal("success"),
  data: UploadedIconSchema,
});
const ApiErrorSchema = v.object({ message: v.string() });

type Theme = v.InferOutput<typeof ThemeSchema>;
type AppSettings = v.InferOutput<typeof AppSettingsSchema>;
type NodeSettings = v.InferOutput<typeof NodeSettingsSchema>;
type ServiceSettings = v.InferOutput<typeof ServiceSettingsSchema>;
type GroupSettings = v.InferOutput<typeof GroupSettingsSchema>;
type WidgetDescriptor = v.InferOutput<typeof WidgetDescriptorSchema>;
type WidgetField = v.InferOutput<typeof WidgetFieldSchema>;
type SettingsData = v.InferOutput<typeof SettingsDataSchema>;
type SettingsPayload = Pick<SettingsData, "dashboard" | "app">;
type AuthStatus = v.InferOutput<typeof AuthStatusResponseSchema>["data"];

export {
  ApiErrorSchema,
  AuthStatusResponseSchema,
  IconUploadResponseSchema,
  MessageResponseSchema,
  SettingsResponseSchema,
};
export type {
  AppSettings,
  AuthStatus,
  GroupSettings,
  WidgetDescriptor,
  WidgetField,
  NodeSettings,
  ServiceSettings,
  SettingsData,
  SettingsPayload,
  Theme,
};
