import { Input } from "@/components/ui/input";
import { Field } from "@/shared/components/form-field";
import { inputClass, selectClass } from "@/shared/components/styles";
import type { EditableWidget } from "@/shared/lib/draft-models";
import type { WidgetDescriptor, WidgetField } from "@/shared/schemas/core";
import { QuickLinksField } from "@/features/widgets/quick-links-field";
import { RemoteOptionsField } from "@/features/widgets/remote-options-field";

export function WidgetConfigFields({
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
              sourceUrl={typeof widget.config.url === "string" ? widget.config.url : ""}
              apiKey={typeof widget.config.key === "string" ? widget.config.key : ""}
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
                key={`${widget.id}:${descriptor.name}`}
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
                className={selectClass}
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
                className={selectClass}
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
