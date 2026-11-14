import { IconPlus, IconTrash } from "@tabler/icons-react";
import { Input } from "@/components/ui/input";
import { Field } from "@/features/settings/components/form-field";
import { IconField } from "@/features/settings/components/icon-field";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
} from "@/features/settings/components/styles";
import type { EditableNode, EditableService } from "@/features/settings/models";
import type { WidgetDescriptor, WidgetField } from "@/features/settings/schema";

type QuickLink = { title: string; url: string; icon: string };

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
          key={index}
        >
          <Field label={`Link ${index + 1} title`}>
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
          <Field label={`Link ${index + 1} destination`}>
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
  service,
  nodes,
  uploading,
  onChange,
  onUpload,
}: {
  descriptor: WidgetDescriptor;
  service: EditableService;
  nodes: EditableNode[];
  uploading: boolean;
  onChange: (config: Record<string, unknown>) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  function updateField(field: WidgetField, value: unknown) {
    onChange({ ...service.widget_config, [field.key]: value });
  }

  if (descriptor.fields.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">{descriptor.description} No settings needed.</p>
    );
  }

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      {descriptor.fields.map((field) => {
        const value = service.widget_config[field.key] ?? field.default ?? "";
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
            <Field key={field.key} label={field.label} description={field.description}>
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={String(value)}
                required={field.required}
                onChange={(event) => updateField(field, event.target.value)}
              >
                <option value="">First configured server</option>
                {nodes.map((node) => (
                  <option key={node.editor_id} value={node.id}>
                    {node.name}
                  </option>
                ))}
              </select>
            </Field>
          );
        }
        if (field.kind === "select") {
          return (
            <Field key={field.key} label={field.label} description={field.description}>
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
          <Field key={field.key} label={field.label} description={field.description}>
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
