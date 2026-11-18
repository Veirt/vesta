import {
  IconArrowDown,
  IconArrowUp,
  IconChevronDown,
  IconPlus,
  IconTrash,
} from "@tabler/icons-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Field } from "@/shared/components/form-field";
import { IconField } from "@/shared/components/icon-field";
import { buttonClass, destructiveButtonClass, inputClass } from "@/shared/components/styles";
import type { EditableNode } from "@/shared/lib/draft-models";

function NodesSection({
  nodes,
  uploading,
  onAdd,
  onChange,
  onMove,
  onRemove,
  onUpload,
}: {
  nodes: EditableNode[];
  uploading: boolean;
  onAdd: () => void;
  onChange: (index: number, update: Partial<EditableNode>) => void;
  onMove: (index: number, direction: -1 | 1) => void;
  onRemove: (index: number) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="flex flex-row flex-wrap items-center justify-between gap-3 border-b border-border/70 px-4 py-4 sm:px-6">
        <div className="space-y-1">
          <CardTitle className="text-base font-semibold">Servers</CardTitle>
          <p className="text-sm text-muted-foreground">
            Set an icon, hostname, and device name for each metrics source.
          </p>
        </div>
        <button className={buttonClass} type="button" onClick={onAdd}>
          <IconPlus size={16} aria-hidden="true" /> Add server
        </button>
      </CardHeader>
      <CardContent className="grid min-w-0 gap-3 px-4 py-4 sm:px-6 sm:py-5">
        {nodes.length === 0 && (
          <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
            No servers yet. Add a local host or a node_exporter endpoint to show telemetry.
          </p>
        )}
        {nodes.map((node, index) => (
          <details
            className="group rounded-lg border border-border/70 bg-background/60"
            key={node.editor_id}
            open={index === nodes.length - 1}
          >
            <summary className="flex min-h-12 cursor-pointer list-none items-center gap-3 px-4 py-3 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
              {node.icon ? (
                <img
                  className="size-8 rounded-md border border-border bg-muted object-contain p-1"
                  src={node.icon}
                  alt=""
                />
              ) : (
                <span className="grid size-8 place-items-center rounded-md border border-border bg-muted text-xs font-medium text-muted-foreground">
                  {node.name.slice(0, 1).toUpperCase() || "S"}
                </span>
              )}
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">
                  {node.name || "New server"}
                </span>
                <span className="block truncate text-xs text-muted-foreground">
                  {[node.hostname, node.device_name].filter(Boolean).join(" · ") ||
                    "Metrics source"}
                </span>
              </span>
              <IconChevronDown
                className="size-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180"
                aria-hidden="true"
              />
            </summary>
            <div className="grid gap-4 border-t border-border/70 p-4 sm:grid-cols-2 sm:p-5 lg:grid-cols-3">
              <Field label="Server name" required>
                <Input
                  value={node.name}
                  required
                  onChange={(event) => onChange(index, { name: event.target.value })}
                  className={inputClass}
                />
              </Field>
              <Field label="Hostname">
                <Input
                  value={node.hostname ?? ""}
                  placeholder="nas.home.arpa"
                  onChange={(event) => onChange(index, { hostname: event.target.value })}
                  className={inputClass}
                />
              </Field>
              <Field label="Device name">
                <Input
                  value={node.device_name ?? ""}
                  placeholder="Synology DS923+"
                  onChange={(event) => onChange(index, { device_name: event.target.value })}
                  className={inputClass}
                />
              </Field>
              <Field label="Metrics endpoint" required>
                <Input
                  value={node.endpoint}
                  required
                  placeholder="local or http://server:9100/metrics"
                  onChange={(event) => onChange(index, { endpoint: event.target.value })}
                  className={inputClass}
                />
              </Field>
              <Field label="Poll interval (seconds)" required>
                <Input
                  type="number"
                  min={1}
                  max={86400}
                  value={node.interval_secs}
                  required
                  onChange={(event) =>
                    onChange(index, { interval_secs: Number(event.target.value) })
                  }
                  className={inputClass}
                />
              </Field>
              <IconField
                className="sm:col-span-2 lg:col-span-3"
                value={node.icon ?? ""}
                uploading={uploading}
                onChange={(icon) => onChange(index, { icon })}
                onUpload={onUpload}
              />
              <div className="flex flex-wrap items-end gap-2 sm:col-span-2 lg:col-span-3">
                <button
                  className={destructiveButtonClass}
                  type="button"
                  aria-label={`Remove ${node.name}`}
                  onClick={() => onRemove(index)}
                >
                  <IconTrash size={16} aria-hidden="true" /> Remove server
                </button>
                {index > 0 && (
                  <button
                    className={buttonClass}
                    type="button"
                    aria-label={`Move ${node.name} up`}
                    onClick={() => onMove(index, -1)}
                  >
                    <IconArrowUp size={16} aria-hidden="true" /> Move up
                  </button>
                )}
                {index < nodes.length - 1 && (
                  <button
                    className={buttonClass}
                    type="button"
                    aria-label={`Move ${node.name} down`}
                    onClick={() => onMove(index, 1)}
                  >
                    <IconArrowDown size={16} aria-hidden="true" /> Move down
                  </button>
                )}
              </div>
            </div>
          </details>
        ))}
      </CardContent>
    </Card>
  );
}

export { NodesSection };
