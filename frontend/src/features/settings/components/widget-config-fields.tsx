import { IconPlus, IconTrash } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import { Field } from "@/features/settings/components/form-field";
import { IconField } from "@/features/settings/components/icon-field";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
} from "@/features/settings/components/styles";
import type { EditableWidget } from "@/features/settings/models";
import { loadRemoteOptions, type RemoteOption } from "@/features/settings/remote-options";
import type { WidgetDescriptor, WidgetField } from "@/features/settings/schema";

type QuickLink = { title: string; url: string; icon: string };

function isAbortError(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === "AbortError";
}

/**
 * A select whose options come from the widget's own external system,
 * resolved against the URL/API key currently typed in the editor; refetches
 * when those change and clears a saved selection that the server lost.
 */
function RemoteOptionsField({
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
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const keyValue = typeof config.key === "string" ? config.key : "";
  const urlValue = typeof config.url === "string" ? config.url : "";

  useEffect(() => {
    if (!urlValue.trim() || !keyValue.trim()) {
      setOptions([]);
      setError("");
      return;
    }
    // Deferred state updates must check this flag: the effect may be cleaned
    // up (unmount or dependency change) while the request is in flight.
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

function parseLinks(value: unknown): QuickLink[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item): QuickLink[] => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const record = item as Record<string, unknown>;
    return [
      {
        title: typeof record.title === "string" ? record.title : "",
        url: typeof record.url === "string" ? record.url : "",
        icon: typeof record.icon === "string" ? record.icon : "",
      },
    ];
  });
}

function QuickLinksField({
  value,
  uploading,
  onChange,
  onUpload,
}: {
  value: unknown;
  uploading: boolean;
  onChange: (links: QuickLink[]) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  const links = parseLinks(value);
  // Rows are edited and removed by position, so identity is positional:
  // hand each row a stable id instead of the array index.
  const [rowIds, setRowIds] = useState<Array<string>>(() =>
    links.map((_, index) => `link-row-${index + 1}`),
  );
  useEffect(() => {
    setRowIds((current) => {
      if (current.length >= links.length) return current;
      const added = Array.from(
        { length: links.length - current.length },
        (_, offset) => `link-row-${current.length + offset + 1}`,
      );
      return [...current, ...added];
    });
  }, [links.length]);

  return (
    <fieldset className="grid gap-3 rounded-md border border-border/70 p-3 sm:p-4">
      <legend className="px-1 text-sm font-medium">Links</legend>
      {links.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No links yet. Add one with a title and destination.
        </p>
      )}
      {links.map((link, index) => (
        <article
          className="grid gap-3 rounded-md border border-border/70 bg-background/50 p-3 sm:grid-cols-2"
          key={rowIds[index]}
        >
          <Field label={`Link ${index + 1} title`} required>
            <Input
              value={link.title}
              required
              onChange={(event) =>
                onChange(
                  links.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, title: event.target.value } : item,
                  ),
                )
              }
              className={inputClass}
            />
          </Field>
          <Field label={`Link ${index + 1} destination`} required>
            <Input
              value={link.url}
              required
              inputMode="url"
              placeholder="https://…"
              onChange={(event) =>
                onChange(
                  links.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, url: event.target.value } : item,
                  ),
                )
              }
              className={inputClass}
            />
          </Field>
          <IconField
            value={link.icon}
            uploading={uploading}
            onChange={(icon) =>
              onChange(
                links.map((item, itemIndex) => (itemIndex === index ? { ...item, icon } : item)),
              )
            }
            onUpload={onUpload}
          />
          <div className="flex items-end">
            <button
              className={destructiveButtonClass}
              type="button"
              onClick={() => onChange(links.filter((_, itemIndex) => itemIndex !== index))}
              aria-label={`Remove link ${link.title || index + 1}`}
            >
              <IconTrash size={16} aria-hidden="true" /> Remove link
            </button>
          </div>
        </article>
      ))}
      <button
        className={`${buttonClass} justify-self-start`}
        type="button"
        onClick={() => onChange([...links, { title: "", url: "", icon: "" }])}
      >
        <IconPlus size={16} aria-hidden="true" /> Add link
      </button>
    </fieldset>
  );
}

function WidgetConfigFields({
  descriptor,
  widget,
  nodes,
  uploading,
  onChange,
  onUpload,
}: {
  descriptor: WidgetDescriptor;
  widget: EditableWidget;
  nodes: Array<{ id: string; name: string }>;
  uploading: boolean;
  onChange: (config: Record<string, unknown>) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  function updateField(field: WidgetField, value: unknown) {
    onChange({ ...widget.config, [field.key]: value });
  }

  if (descriptor.fields.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">{descriptor.description} No settings needed.</p>
    );
  }

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      {descriptor.fields.map((field) => {
        const value = widget.config[field.key] ?? field.default ?? "";
        if (field.kind === "remote_select") {
          return (
            <RemoteOptionsField
              key={field.key}
              widgetName={descriptor.name}
              field={field}
              config={widget.config}
              value={value}
              required={field.required}
              onChange={(next) => updateField(field, next)}
            />
          );
        }
        if (field.kind === "links") {
          return (
            <div className="sm:col-span-2" key={field.key}>
              <QuickLinksField
                value={value}
                uploading={uploading}
                onChange={(links) => updateField(field, links)}
                onUpload={onUpload}
              />
              <p className="mt-2 text-xs leading-5 text-muted-foreground">{field.description}</p>
            </div>
          );
        }
        if (field.kind === "node") {
          return (
            <Field
              key={field.key}
              label={field.label}
              description={field.description}
              required={field.required}
            >
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={String(value)}
                required={field.required}
                onChange={(event) => updateField(field, event.target.value)}
              >
                <option value="">First configured server</option>
                {nodes.map((node) => (
                  <option key={node.id} value={node.id}>
                    {node.name}
                  </option>
                ))}
              </select>
            </Field>
          );
        }
        if (field.kind === "select") {
          return (
            <Field
              key={field.key}
              label={field.label}
              description={field.description}
              required={field.required}
            >
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={String(value)}
                required={field.required}
                onChange={(event) => updateField(field, event.target.value)}
              >
                {(field.options ?? []).map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </Field>
          );
        }
        const inputType =
          field.kind === "url"
            ? "url"
            : field.kind === "password"
              ? "password"
              : field.kind === "number"
                ? "number"
                : "text";
        const inputValue = value === null || value === undefined ? "" : String(value);
        return (
          <Field
            key={field.key}
            label={field.label}
            description={field.description}
            required={field.required}
          >
            <Input
              type={inputType}
              value={inputValue}
              required={field.required}
              onChange={(event) => {
                const rawValue = event.target.value;
                updateField(
                  field,
                  field.kind === "number" && rawValue !== "" ? Number(rawValue) : rawValue,
                );
              }}
              className={inputClass}
            />
          </Field>
        );
      })}
    </div>
  );
}

export { WidgetConfigFields };
