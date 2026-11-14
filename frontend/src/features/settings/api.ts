import * as v from "valibot";
import {
  ApiErrorSchema,
  AuthStatusResponseSchema,
  IconUploadResponseSchema,
  MessageResponseSchema,
  SettingsResponseSchema,
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

export { fetchAuthStatus, fetchSettings, saveSettings, signOut, submitPassword, uploadIcon };
