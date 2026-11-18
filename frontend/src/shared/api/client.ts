import * as v from "valibot";
import {
  ApiErrorSchema,
  AuthStatusResponseSchema,
  DashboardResponseSchema,
  IconUploadResponseSchema,
  InstalledPluginResponseSchema,
  MessageResponseSchema,
  PingStatusResponseSchema,
  PingTestResponseSchema,
  PluginsResponseSchema,
  RemovedPluginResponseSchema,
  SettingsResponseSchema,
  WidgetDataResponseSchema,
  WidgetOptionsResponseSchema,
} from "@/shared/schemas/api";
import type {
  DashboardData,
  Plugin,
  SettingsData,
  SettingsPayload,
  WidgetOption,
} from "@/shared/schemas/core";

export type RemoteOption = WidgetOption;

export async function decode<TSchema extends v.GenericSchema>(
  response: Response,
  schema: TSchema,
): Promise<v.InferOutput<TSchema>> {
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new Error(`The server returned an unreadable response (${response.status})`);
  }

  if (!response.ok) {
    const parsedError = v.safeParse(ApiErrorSchema, body);
    throw new Error(
      parsedError.success ? parsedError.output.message : `Request failed (${response.status})`,
    );
  }
  const parsed = v.safeParse(schema, body);
  if (!parsed.success) throw new Error("The server returned data in an unexpected format");
  return parsed.output;
}

export async function requestJson<TSchema extends v.GenericSchema>(
  url: string,
  schema: TSchema,
  init?: RequestInit,
): Promise<v.InferOutput<TSchema>> {
  const response = await fetch(url, init);
  return decode(response, schema);
}

export async function fetchDashboard(signal?: AbortSignal): Promise<DashboardData> {
  return (
    await requestJson("/api/dashboard", DashboardResponseSchema, {
      headers: { Accept: "application/json" },
      cache: "no-store",
      signal,
    })
  ).data;
}

export async function fetchAuthStatus() {
  return (
    await requestJson("/api/auth/status", AuthStatusResponseSchema, {
      cache: "no-store",
    })
  ).data;
}

export async function fetchSettings(): Promise<SettingsData> {
  return (
    await requestJson("/api/settings", SettingsResponseSchema, {
      cache: "no-store",
    })
  ).data;
}

export async function saveSettings(value: SettingsPayload) {
  await requestJson("/api/settings", MessageResponseSchema, {
    method: "PUT",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(value),
  });
}

export async function saveDashboardItems(
  items: SettingsData["dashboard"]["items"],
  groups: SettingsData["dashboard"]["groups"],
  signal?: AbortSignal,
) {
  await requestJson("/api/settings/dashboard", MessageResponseSchema, {
    method: "PUT",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ items, groups }),
    signal,
  });
}

export async function testPingUrl(url: string, signal?: AbortSignal): Promise<boolean> {
  return (
    await requestJson("/api/settings/ping-test", PingTestResponseSchema, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ url }),
      signal,
    })
  ).data.up;
}

export async function pingService(id: string, signal?: AbortSignal): Promise<boolean> {
  const params = new URLSearchParams({ id });
  return (
    await requestJson(`/api/ping?${params.toString()}`, PingStatusResponseSchema, {
      cache: "no-store",
      signal,
    })
  ).data.up;
}

export async function submitPassword(mode: "setup" | "login", password: string) {
  await requestJson(`/api/auth/${mode}`, MessageResponseSchema, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ password }),
  });
}

export async function signOut() {
  await requestJson("/api/auth/logout", MessageResponseSchema, { method: "POST" });
}

export async function uploadIcon(file: File) {
  return (
    await requestJson(
      `/api/settings/icons?name=${encodeURIComponent(file.name)}`,
      IconUploadResponseSchema,
      {
        method: "POST",
        headers: { "Content-Type": file.type },
        body: file,
      },
    )
  ).data;
}

export async function listPlugins(): Promise<Plugin[]> {
  return (await requestJson("/api/plugins", PluginsResponseSchema, { cache: "no-store" })).data
    .plugins;
}

export async function installPlugin(url: string): Promise<Plugin> {
  return (
    await requestJson("/api/plugins", InstalledPluginResponseSchema, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ url }),
    })
  ).data;
}

export async function updatePlugin(id: string): Promise<Plugin> {
  return (
    await requestJson(`/api/plugins/${encodeURIComponent(id)}`, InstalledPluginResponseSchema, {
      method: "PUT",
      headers: { Accept: "application/json" },
    })
  ).data;
}

export async function uninstallPlugin(id: string) {
  await requestJson(`/api/plugins/${encodeURIComponent(id)}`, RemovedPluginResponseSchema, {
    method: "DELETE",
    headers: { Accept: "application/json" },
  });
}

export async function loadRemoteOptions(
  widgetName: string,
  fieldKey: string,
  config: Record<string, unknown>,
  signal: AbortSignal,
): Promise<RemoteOption[]> {
  return (
    await requestJson(
      `/api/widgets/${encodeURIComponent(widgetName)}/options`,
      WidgetOptionsResponseSchema,
      {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ field: fieldKey, config }),
        signal,
      },
    )
  ).data.options;
}

export async function fetchWidgetData(
  widgetName: string,
  itemId: string,
  signal?: AbortSignal,
): Promise<Record<string, unknown>> {
  const params = new URLSearchParams({ id: itemId });
  return (
    await requestJson(
      `/api/widgets/${encodeURIComponent(widgetName)}?${params.toString()}`,
      WidgetDataResponseSchema,
      {
        cache: "no-store",
        signal,
      },
    )
  ).data.widget;
}
