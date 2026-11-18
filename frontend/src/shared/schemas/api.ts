import * as v from "valibot";
import {
  DashboardDataSchema,
  PluginSchema,
  SettingsDataSchema,
  UploadedIconSchema,
  WidgetOptionSchema,
} from "@/shared/schemas/core";

export const ApiErrorSchema = v.object({ message: v.string() });

export function successEnvelope<TSchema extends v.GenericSchema>(data: TSchema) {
  return v.object({
    status: v.literal("success"),
    data,
  });
}

export const DashboardResponseSchema = successEnvelope(DashboardDataSchema);
export const SettingsResponseSchema = successEnvelope(SettingsDataSchema);
export const AuthStatusResponseSchema = successEnvelope(
  v.object({
    setup_required: v.boolean(),
    authenticated: v.boolean(),
  }),
);
export const MessageResponseSchema = successEnvelope(v.object({ message: v.string() }));
export const PingStatusResponseSchema = successEnvelope(v.object({ up: v.boolean() }));
export const PingTestResponseSchema = PingStatusResponseSchema;
export const IconUploadResponseSchema = successEnvelope(UploadedIconSchema);
export const PluginsResponseSchema = successEnvelope(v.object({ plugins: v.array(PluginSchema) }));
export const InstalledPluginResponseSchema = successEnvelope(PluginSchema);
export const RemovedPluginResponseSchema = successEnvelope(v.object({ removed: v.string() }));
export const WidgetOptionsResponseSchema = successEnvelope(
  v.object({ options: v.array(WidgetOptionSchema) }),
);
export const WidgetDataResponseSchema = successEnvelope(
  v.object({
    widget: v.record(v.string(), v.unknown()),
    refresh_interval_secs: v.nullable(v.number()),
  }),
);

export type AuthStatus = v.InferOutput<typeof AuthStatusResponseSchema>["data"];

export function parseEvent<TSchema extends v.GenericSchema>(schema: TSchema, raw: string) {
  try {
    const parsed = v.safeParse(schema, JSON.parse(raw));
    return parsed.success ? parsed.output : null;
  } catch {
    return null;
  }
}
