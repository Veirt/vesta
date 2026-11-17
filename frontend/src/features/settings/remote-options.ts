type RemoteOption = { value: string; label: string };

function errorMessage(result: unknown, status: number): string {
  if (
    result &&
    typeof result === "object" &&
    "message" in result &&
    typeof result.message === "string"
  ) {
    return result.message;
  }
  return `The request failed (${status}).`;
}

async function loadRemoteOptions(
  widgetName: string,
  fieldKey: string,
  config: Record<string, unknown>,
  signal: AbortSignal,
): Promise<RemoteOption[]> {
  const response = await fetch(`/api/widgets/${encodeURIComponent(widgetName)}/options`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: fieldKey, config }),
    signal,
  });
  const result: unknown = await response.json();
  if (!response.ok) throw new Error(errorMessage(result, response.status));
  const body = (result && typeof result === "object" ? result : {}) as {
    status?: unknown;
    data?: { options?: unknown } | null;
  };
  if (body.status !== "success" || !Array.isArray(body.data?.options)) {
    throw new Error("The server returned an invalid options list.");
  }
  return ((body.data?.options as unknown[]) ?? []).flatMap((option): RemoteOption[] => {
    if (!option || typeof option !== "object") return [];
    const record = option as Record<string, unknown>;
    return typeof record.value === "string" && typeof record.label === "string"
      ? [{ value: record.value, label: record.label }]
      : [];
  });
}

export { loadRemoteOptions };
export type { RemoteOption };
