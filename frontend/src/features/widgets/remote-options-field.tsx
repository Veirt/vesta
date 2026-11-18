import { useEffect, useState } from "react";
import { loadRemoteOptions, type RemoteOption } from "@/shared/api/client";
import { Field } from "@/shared/components/form-field";
import { buttonClass } from "@/shared/components/styles";
import type { WidgetField } from "@/shared/schemas/core";

function isAbortError(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === "AbortError";
}

/**
 * A select whose options come from the widget's own external system, resolved
 * against the URL/API key currently typed in the editor; refetches when those
 * change and clears a saved selection that the server lost.
 */
export function RemoteOptionsField({
  widgetName,
  field,
  config,
  value,
  required,
  onChange,
}: {
  widgetName: string;
  field: WidgetField;
  config: Record<string, unknown>;
  value: unknown;
  required: boolean;
  onChange: (value: string) => void;
}) {
  const [options, setOptions] = useState<RemoteOption[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);

  const keyValue = typeof config.key === "string" ? config.key : "";
  const urlValue = typeof config.url === "string" ? config.url : "";

  useEffect(() => {
    if (!urlValue.trim() || !keyValue.trim()) {
      setOptions([]);
      setError("");
      return;
    }
    let active = true;
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      setLoading(true);
      loadRemoteOptions(widgetName, field.key, config, controller.signal)
        .then((loaded) => {
          if (!active) return;
          setOptions(loaded);
          setError("");
        })
        .catch((cause: unknown) => {
          if (!active) return;
          if (isAbortError(cause)) return;
          setOptions([]);
          setError(cause instanceof Error ? cause.message : "The request failed.");
        })
        .finally(() => {
          if (active && !controller.signal.aborted) setLoading(false);
        });
    }, 600);
    return () => {
      active = false;
      controller.abort();
      window.clearTimeout(timer);
    };
    // `config` is intentionally read at fetch time only; the trigger
    // dependencies are the values the option list is derived from.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [urlValue, keyValue, widgetName, field.key, attempt]);

  const currentValue = typeof value === "string" ? value : "";
  const known = currentValue === "" || options.some((option) => option.value === currentValue);

  return (
    <Field
      label={field.label}
      description={
        options.length === 0 && !error && !loading
          ? `${field.description} Enter the URL and API key above to list them.`
          : field.description
      }
      required={required}
    >
      <div className="flex items-center gap-2">
        <span className="relative grow">
          <select
            className="h-9 w-full rounded-md border border-input bg-background px-3 pr-8 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            value={currentValue}
            required={required}
            onChange={(event) => onChange(event.target.value)}
          >
            <option value="">All libraries</option>
            {options.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          {loading && (
            <span className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
              loading…
            </span>
          )}
        </span>
        {error && (
          <button className={buttonClass} type="button" onClick={() => setAttempt(attempt + 1)}>
            Retry
          </button>
        )}
      </div>
      {error && <p className="text-xs leading-5 text-destructive">{error}</p>}
      {options.length > 0 && !known && (
        <p className="text-xs leading-5 text-muted-foreground">
          The saved selection is no longer offered by the server; pick a library again.
        </p>
      )}
    </Field>
  );
}
