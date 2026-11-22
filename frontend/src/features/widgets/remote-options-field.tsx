import { useEffect, useState } from "react";
import { loadRemoteOptions, type RemoteOption } from "@/shared/api/client";
import { Field } from "@/shared/components/form-field";
import { buttonClass, selectClass } from "@/shared/components/styles";
import { isAbortError } from "@/shared/lib/errors";
import type { WidgetField } from "@/shared/schemas/core";

export function RemoteOptionsField({
  widgetName,
  field,
  sourceUrl,
  apiKey,
  value,
  required,
  onChange,
}: {
  widgetName: string;
  field: WidgetField;
  sourceUrl: string;
  apiKey: string;
  value: unknown;
  required: boolean;
  onChange: (value: string) => void;
}) {
  const [options, setOptions] = useState<RemoteOption[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!sourceUrl.trim() || !apiKey.trim()) {
      setOptions([]);
      setError("");
      return;
    }
    let active = true;
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      setLoading(true);
      loadRemoteOptions(widgetName, field.key, { url: sourceUrl, key: apiKey }, controller.signal)
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
  }, [sourceUrl, apiKey, widgetName, field.key, attempt]);

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
            className={`${selectClass} pr-8`}
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
