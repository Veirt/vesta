import * as v from "valibot";
import {
  ApiErrorSchema,
  AuthStatusResponseSchema,
  IconUploadResponseSchema,
  InstalledPluginResponseSchema,
  MessageResponseSchema,
  PingTestResponseSchema,
  PluginsResponseSchema,
  SettingsResponseSchema,
  type Plugin,
  type SettingsData,
  type SettingsPayload,
} from "@/features/settings/schema";

async function decode<TSchema extends v.GenericSchema>(
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

async function fetchAuthStatus() {
  const response = await fetch("/api/auth/status", { cache: "no-store" });
  return (await decode(response, AuthStatusResponseSchema)).data;
}

async function fetchSettings(): Promise<SettingsData> {
  const response = await fetch("/api/settings", { cache: "no-store" });
  return (await decode(response, SettingsResponseSchema)).data;
}

async function saveSettings(value: SettingsPayload) {
  const response = await fetch("/api/settings", {
    method: "PUT",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(value),
  });
  await decode(response, MessageResponseSchema);
}

async function saveDashboardItems(
  items: SettingsData["dashboard"]["items"],
  groups: SettingsData["dashboard"]["groups"],
  signal?: AbortSignal,
) {
  const response = await fetch("/api/settings/dashboard", {
    method: "PUT",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ items, groups }),
    signal,
  });
  await decode(response, MessageResponseSchema);
}

async function testPingUrl(url: string, signal?: AbortSignal): Promise<boolean> {
  const response = await fetch("/api/settings/ping-test", {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ url }),
    signal,
  });
  return (await decode(response, PingTestResponseSchema)).data.up;
}

async function submitPassword(mode: "setup" | "login", password: string) {
  const response = await fetch(`/api/auth/${mode}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ password }),
  });
  await decode(response, MessageResponseSchema);
}

async function signOut() {
  const response = await fetch("/api/auth/logout", { method: "POST" });
  await decode(response, MessageResponseSchema);
}

async function uploadIcon(file: File) {
  const response = await fetch(`/api/settings/icons?name=${encodeURIComponent(file.name)}`, {
    method: "POST",
    headers: { "Content-Type": file.type },
    body: file,
  });
  return (await decode(response, IconUploadResponseSchema)).data;
}

async function listPlugins(): Promise<Plugin[]> {
  const response = await fetch("/api/plugins", { cache: "no-store" });
  return (await decode(response, PluginsResponseSchema)).data.plugins;
}

async function installPlugin(url: string): Promise<Plugin> {
  const response = await fetch("/api/plugins", {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ url }),
  });
  return (await decode(response, InstalledPluginResponseSchema)).data;
}

async function updatePlugin(id: string): Promise<Plugin> {
  const response = await fetch(`/api/plugins/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { Accept: "application/json" },
  });
  return (await decode(response, InstalledPluginResponseSchema)).data;
}

async function uninstallPlugin(id: string) {
  const response = await fetch(`/api/plugins/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: { Accept: "application/json" },
  });
  await decode(response, MessageResponseSchema);
}

export {
  fetchAuthStatus,
  fetchSettings,
  installPlugin,
  listPlugins,
  uninstallPlugin,
  updatePlugin,
  saveDashboardItems,
  saveSettings,
  signOut,
  submitPassword,
  testPingUrl,
  uploadIcon,
};
