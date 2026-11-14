import { IconArrowDown, IconArrowUp, IconTrash } from "@tabler/icons-react";
import { Input } from "@/components/ui/input";
import { Field } from "@/features/settings/components/form-field";
import { IconField } from "@/features/settings/components/icon-field";
import { WidgetConfigFields } from "@/features/settings/components/widget-config-fields";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
} from "@/features/settings/components/styles";
import type { EditableNode, EditableService } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

function ServiceEditor({
  service,
  index,
  canMoveDown,
  widgets,
  nodes,
  uploading,
  onChange,
  onRemove,
  onMove,
  onResize,
  onUpload,
}: {
  service: EditableService;
  index: number;
  canMoveDown: boolean;
  widgets: WidgetDescriptor[];
  nodes: EditableNode[];
  uploading: boolean;
  onChange: (update: Partial<EditableService>) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  onResize: (dimension: "width" | "height", change: -1 | 1) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  const selectedWidget = widgets.find(({ name }) => name === service.widget_name);

  return (
    <article className="grid gap-3 rounded-lg border border-border/70 bg-background/50 p-3 sm:p-4">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="min-w-0 flex-1 truncate text-sm font-medium">
          {service.title || `Service ${index + 1}`}
          <span className="ml-2 font-normal text-muted-foreground">
            {service.widget_name || "Link card"}
          </span>
        </h3>
        <button
          className={buttonClass}
          type="button"
          aria-label={`Make ${service.title} narrower`}
          disabled={service.width <= 1}
          onClick={() => onResize("width", -1)}
        >
          − Width
        </button>
        <button
          className={buttonClass}
          type="button"
          aria-label={`Make ${service.title} wider`}
          disabled={service.width >= 12}
          onClick={() => onResize("width", 1)}
        >
          + Width
        </button>
        <button
          className={buttonClass}
          type="button"
          aria-label={`Make ${service.title} shorter`}
          disabled={service.height <= 1}
          onClick={() => onResize("height", -1)}
        >
          − Height
        </button>
        <button
          className={buttonClass}
          type="button"
          aria-label={`Make ${service.title} taller`}
          disabled={service.height >= 12}
          onClick={() => onResize("height", 1)}
        >
          + Height
        </button>
        {index > 0 && (
          <button
            className={buttonClass}
            type="button"
            onClick={() => onMove(-1)}
            aria-label={`Move ${service.title} earlier`}
          >
            <IconArrowUp size={16} aria-hidden="true" /> Move earlier
          </button>
        )}
        <button
          className={destructiveButtonClass}
          type="button"
          onClick={onRemove}
          aria-label={`Remove ${service.title}`}
        >
          <IconTrash size={16} aria-hidden="true" /> Remove
        </button>
      </div>
      <details className="group rounded-md border border-border/70 bg-card/40">
        <summary className="flex min-h-10 cursor-pointer list-none items-center justify-between gap-2 px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
          <span>Configure service</span>
          <span className="text-xs text-muted-foreground group-open:hidden">Show settings</span>
          <span className="hidden text-xs text-muted-foreground group-open:inline">
            Hide settings
          </span>
        </summary>
        <div className="grid gap-4 border-t border-border/70 p-3 sm:grid-cols-2 sm:p-4">
          <Field label="Service title">
            <Input
              value={service.title}
              required
              onChange={(event) => onChange({ title: event.target.value })}
              className={inputClass}
            />
          </Field>
          <Field label="Destination URL" description="Leave empty for a non-clickable card.">
            <Input
              value={service.href}
              inputMode="url"
              placeholder="https://…"
              onChange={(event) => onChange({ href: event.target.value })}
              className={inputClass}
            />
          </Field>
          <Field
            label="Availability check URL"
            description="Optional HTTP(S) URL checked for availability."
          >
            <Input
              value={service.ping_url}
              inputMode="url"
              placeholder="https://…"
              onChange={(event) => onChange({ ping_url: event.target.value })}
              className={inputClass}
            />
          </Field>
          <IconField
            value={service.icon}
            uploading={uploading}
            onChange={(icon) => onChange({ icon })}
            onUpload={onUpload}
          />
          <Field label="Content">
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              value={service.widget_name}
              onChange={(event) => onChange({ widget_name: event.target.value })}
            >
              <option value="">Link card</option>
              {widgets.map((widget) => (
                <option key={widget.name} value={widget.name}>
                  {widget.name}
                </option>
              ))}
            </select>
          </Field>
          {selectedWidget && (
            <div className="grid gap-3 sm:col-span-2">
              <div>
                <h4 className="text-sm font-medium">{selectedWidget.name} settings</h4>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  {selectedWidget.description}
                </p>
              </div>
              <WidgetConfigFields
                descriptor={selectedWidget}
                service={service}
                nodes={nodes}
                uploading={uploading}
                onChange={(widget_config) => onChange({ widget_config })}
                onUpload={onUpload}
              />
            </div>
          )}
          {canMoveDown && (
            <div className="sm:col-span-2">
              <p className="mb-2 text-xs font-medium text-muted-foreground">
                Move this service in the layout
              </p>
              <button
                className={buttonClass}
                type="button"
                onClick={() => onMove(1)}
                aria-label={`Move ${service.title} later`}
              >
                <IconArrowDown size={16} aria-hidden="true" /> Move later
              </button>
            </div>
          )}
        </div>
      </details>
    </article>
  );
}

export { ServiceEditor };
