import { IconArrowDown, IconArrowUp, IconTrash } from "@tabler/icons-react";
import { useEffect, useState, type FormEvent } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { GroupPicker } from "@/features/dashboard/group-picker";
import { WidgetConfigFields } from "@/features/widgets/widget-config-fields";
import { testPingUrl } from "@/shared/api/client";
import { Field } from "@/shared/components/form-field";
import { IconField } from "@/shared/components/icon-field";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
  primaryButtonClass,
  selectClass,
} from "@/shared/components/styles";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/shared/lib/draft-models";
import { isAbortError } from "@/shared/lib/errors";
import { groupSpanOf, minSizeFor, type TileSize } from "@/shared/lib/layout";
import { validHttpUrl } from "@/shared/lib/validation";
import type { WidgetDescriptor } from "@/shared/schemas/core";

const sizeOptions = [
  { value: "3x1", label: "Compact", width: 3, height: 1 },
  { value: "4x1", label: "Standard", width: 4, height: 1 },
  { value: "6x1", label: "Wide", width: 6, height: 1 },
  { value: "4x2", label: "Boxed", width: 4, height: 2 },
  { value: "6x2", label: "Large", width: 6, height: 2 },
  { value: "12x3", label: "Full width", width: 12, height: 3 },
];

function PingUrlFeedback({ url }: { url: string }) {
  const [state, setState] = useState<"idle" | "invalid" | "checking" | "up" | "down" | "error">(
    "idle",
  );

  useEffect(() => {
    if (!url.trim()) {
      setState("idle");
      return;
    }
    if (!validHttpUrl(url)) {
      setState("invalid");
      return;
    }

    const controller = new AbortController();
    setState("checking");
    const timer = window.setTimeout(() => {
      void testPingUrl(url, controller.signal)
        .then((up) => setState(up ? "up" : "down"))
        .catch((cause: unknown) => {
          if (!isAbortError(cause)) setState("error");
        });
    }, 500);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [url]);

  const feedback = {
    idle: "Add an HTTP(S) URL to check it from this server.",
    invalid: "Enter a complete http:// or https:// URL.",
    checking: "Checking this URL…",
    up: "The endpoint is responding.",
    down: "No successful response. Check the URL or service access.",
    error: "The check could not run. Sign in again and retry.",
  }[state];
  const tone =
    state === "up"
      ? "text-primary"
      : state === "down" || state === "error"
        ? "text-destructive"
        : "text-muted-foreground";

  return (
    <p className={`text-xs ${tone}`} role="status" aria-live="polite">
      {feedback}
    </p>
  );
}

function TileSizeField({
  item,
  min,
  onResize,
}: {
  item: EditableDashboardItem;
  min: TileSize;
  onResize: (width: number, height: number) => void;
}) {
  const current = `${item.width}x${item.height}`;
  const isPreset = sizeOptions.some((option) => option.value === current);
  const options = sizeOptions.filter(
    (option) => option.width >= min.width && option.height >= min.height,
  );
  return (
    <Field
      label="Tile size"
      description={`Compatible sizes start at ${min.width}×${min.height}. Resize the tile directly on the canvas, or choose a preset here.`}
    >
      <select
        className={selectClass}
        value={current}
        onChange={(event) => {
          const option = sizeOptions.find(({ value }) => value === event.target.value);
          if (option) onResize(option.width, option.height);
        }}
      >
        {!isPreset && (
          <option value={current}>
            {current === `${min.width}x${min.height}`
              ? `Minimum (${item.width} × ${item.height})`
              : `Custom size (${item.width} × ${item.height})`}
          </option>
        )}
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </Field>
  );
}

function PingUrlField({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <Field
      label="Availability check URL"
      description="Vesta checks this endpoint every 30 seconds."
    >
      <div className="grid gap-2">
        <Input
          value={value}
          inputMode="url"
          placeholder="https://service.example/health"
          onChange={(event) => onChange(event.target.value)}
          className={inputClass}
        />
        <PingUrlFeedback url={value} />
      </div>
    </Field>
  );
}

export function TileEditorDialog({
  item,
  nodes,
  widgets,
  groups,
  uploading,
  index,
  itemCount,
  onChange,
  onRemove,
  onMove,
  onResize,
  onGroupChange,
  onCreateGroup,
  onUpload,
  onClose,
}: {
  item: EditableDashboardItem;
  nodes: Array<{ id: string; name: string }>;
  widgets: WidgetDescriptor[];
  groups: EditableDashboardGroup[];
  uploading: boolean;
  index: number;
  itemCount: number;
  onChange: (update: object) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  onResize: (width: number, height: number) => void;
  onGroupChange: (group: string | null) => void;
  onCreateGroup: (title: string) => string;
  onUpload: (file: File) => Promise<string | null>;
  onClose: () => void;
}) {
  const widget = item.kind === "widget" ? item : null;
  const descriptor = widget ? widgets.find(({ name }) => name === widget.name) : undefined;

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onClose();
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="dashboard-editor-dialog sm:max-w-3xl">
        <form className="grid gap-5" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle id="tile-editor-title">
              Edit {item.kind === "service" ? "service" : "widget"}
            </DialogTitle>
            <DialogDescription>
              Changes stay in this draft until you save the dashboard.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 sm:grid-cols-2">
            <Field
              label="Tile title"
              description="Shown as the heading on the dashboard tile."
              required
            >
              <Input
                value={item.title}
                required
                maxLength={100}
                onChange={(event) => onChange({ title: event.target.value })}
                className={inputClass}
              />
            </Field>
            <Field label="Group" description="Group services and widgets as independent tiles.">
              <GroupPicker
                value={item.group}
                groups={groups}
                onChange={onGroupChange}
                onCreate={onCreateGroup}
              />
            </Field>
            <TileSizeField
              item={item}
              min={minSizeFor(item, widgets, groupSpanOf(groups, item.group))}
              onResize={onResize}
            />
            {item.kind === "service" ? (
              <>
                <Field label="Destination URL" description="Where the service tile opens.">
                  <Input
                    value={item.href}
                    inputMode="url"
                    placeholder="https://service.example"
                    onChange={(event) => onChange({ href: event.target.value })}
                    className={inputClass}
                  />
                </Field>
                <Field
                  label="Destination display"
                  description="Hiding this only removes the URL text; the tile will still open the destination."
                >
                  <label className="flex min-h-9 items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={item.show_destination}
                      onChange={(event) => onChange({ show_destination: event.target.checked })}
                      className="size-4 accent-primary"
                    />
                    Show destination URL on the tile
                  </label>
                </Field>
                <PingUrlField
                  value={item.ping_url}
                  onChange={(ping_url) => onChange({ ping_url })}
                />
                <IconField
                  className="sm:col-span-2"
                  value={item.icon}
                  uploading={uploading}
                  onChange={(icon) => onChange({ icon })}
                  onUpload={onUpload}
                />
              </>
            ) : (
              <div className="sm:col-span-2">
                {descriptor ? (
                  <div className="grid gap-3">
                    <div>
                      <h4 className="text-sm font-medium">{descriptor.title} settings</h4>
                      <p className="mt-1 text-xs leading-5 text-muted-foreground">
                        {descriptor.description}
                      </p>
                    </div>
                    <WidgetConfigFields
                      descriptor={descriptor}
                      widget={item}
                      nodes={nodes}
                      uploading={uploading}
                      onChange={(config) => onChange({ config })}
                      onUpload={onUpload}
                    />
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    This widget type is no longer available to add. Existing settings remain intact.
                  </p>
                )}
              </div>
            )}
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
            <div className="flex items-center gap-2">
              <button
                className={buttonClass}
                type="button"
                disabled={index === 0}
                onClick={() => onMove(-1)}
                aria-label={`Move ${item.title} earlier`}
              >
                <IconArrowUp size={16} aria-hidden="true" /> Move earlier
              </button>
              <button
                className={buttonClass}
                type="button"
                disabled={index === itemCount - 1}
                onClick={() => onMove(1)}
                aria-label={`Move ${item.title} later`}
              >
                <IconArrowDown size={16} aria-hidden="true" /> Move later
              </button>
            </div>
            <button className={destructiveButtonClass} type="button" onClick={onRemove}>
              <IconTrash size={16} aria-hidden="true" /> Remove tile
            </button>
          </div>

          <div className="flex justify-end">
            <button className={primaryButtonClass} type="submit">
              Done
            </button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
