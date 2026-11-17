import { IconArrowDown, IconArrowUp, IconChevronDown, IconTrash } from "@tabler/icons-react";
import { useEffect, useState, type FormEvent } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";
import { Field } from "@/features/settings/components/form-field";
import { IconField } from "@/features/settings/components/icon-field";
import { WidgetConfigFields } from "@/features/settings/components/widget-config-fields";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
  primaryButtonClass,
} from "@/features/settings/components/styles";
import { testPingUrl } from "@/features/settings/api";
import { groupSpanOf, minSizeFor, type TileSize } from "@/features/dashboard/tile-sizes";
import { useDashboardEditor } from "@/features/dashboard/use-dashboard-editor";

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
    try {
      const parsed = new URL(url);
      if (!(["http:", "https:"].includes(parsed.protocol) && parsed.hostname)) {
        setState("invalid");
        return;
      }
    } catch {
      setState("invalid");
      return;
    }

    const controller = new AbortController();
    setState("checking");
    const timer = window.setTimeout(() => {
      void testPingUrl(url, controller.signal)
        .then((up) => setState(up ? "up" : "down"))
        .catch((cause: unknown) => {
          if (!(cause instanceof DOMException && cause.name === "AbortError")) setState("error");
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
        className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
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

function GroupPicker({
  value,
  groups,
  onChange,
  onCreate,
}: {
  value: string | null;
  groups: EditableDashboardGroup[];
  onChange: (group: string | null) => void;
  onCreate: (title: string) => string;
}) {
  const [open, setOpen] = useState(false);
  const [newGroup, setNewGroup] = useState("");

  function choose(group: string | null) {
    onChange(group);
    setOpen(false);
    setNewGroup("");
  }

  function createGroup() {
    const title = newGroup.trim();
    if (title && title.length <= 64) choose(onCreate(title));
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          className="inline-flex h-9 w-full items-center justify-between gap-2 rounded-md border border-input bg-background px-3 text-left text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
          type="button"
          aria-label="Choose a dashboard group"
        >
          <span className="truncate">
            {groups.find((group) => group.id === value)?.title ?? value ?? "Ungrouped"}
          </span>
          <IconChevronDown size={16} aria-hidden="true" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="grid w-72 gap-3">
        <div>
          <h4 className="text-sm font-medium">Group tiles</h4>
          <p className="mt-1 text-xs text-muted-foreground">
            Groups organize services and widgets together without nesting either type.
          </p>
        </div>
        <div className="grid max-h-40 gap-1 overflow-y-auto" role="group" aria-label="Groups">
          <button
            type="button"
            className={`rounded px-2 py-1.5 text-left text-sm hover:bg-accent ${!value ? "bg-accent" : ""}`}
            aria-pressed={!value}
            onClick={() => choose(null)}
          >
            Ungrouped
          </button>
          {groups
            .filter((group) => group.id !== "")
            .map((group) => (
              <button
                key={group.id}
                type="button"
                className={`truncate rounded px-2 py-1.5 text-left text-sm hover:bg-accent ${value === group.id ? "bg-accent" : ""}`}
                aria-pressed={value === group.id}
                onClick={() => choose(group.id)}
              >
                {group.title}
              </button>
            ))}
        </div>
        <div className="grid gap-2 border-t border-border pt-3">
          <label className="grid gap-1 text-sm font-medium" htmlFor="new-dashboard-group">
            Create a group
            <Input
              id="new-dashboard-group"
              value={newGroup}
              maxLength={64}
              placeholder="e.g. Media"
              onChange={(event) => setNewGroup(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  createGroup();
                }
              }}
              className="font-normal"
            />
          </label>
          <button
            type="button"
            className={buttonClass}
            disabled={!newGroup.trim()}
            onClick={createGroup}
          >
            Use group
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}

function TileEditorDialog({
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

const groupWidthOptions = [
  { value: 3, label: "Narrow" },
  { value: 4, label: "Small" },
  { value: 6, label: "Half" },
  { value: 8, label: "Wide" },
  { value: 12, label: "Full width" },
];

function GroupEditorDialog({
  group,
  itemCount,
  uploading,
  onChange,
  onMove,
  onWidthChange,
  onRemove,
  onUpload,
  onClose,
}: {
  group: EditableDashboardGroup;
  itemCount: number;
  uploading: boolean;
  onChange: (update: Partial<EditableDashboardGroup>) => void;
  onMove: (direction: -1 | 1) => void;
  onWidthChange: (width: number) => void;
  onRemove: () => void;
  onUpload: (file: File) => Promise<string | null>;
  onClose: () => void;
}) {
  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onClose();
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="dashboard-editor-dialog sm:max-w-2xl">
        <form className="grid gap-5" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Edit section</DialogTitle>
            <DialogDescription>
              Sections organize independent service and widget tiles. Changes stay in this draft
              until you save the dashboard.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field
              label="Section name"
              description="Shown as the heading above the section's tiles."
              required
            >
              <Input
                value={group.title}
                required
                maxLength={64}
                onChange={(event) => onChange({ title: event.target.value })}
                className={inputClass}
              />
            </Field>
            <Field label="Description" description="A short hint about what belongs here.">
              <textarea
                value={group.description}
                maxLength={240}
                rows={2}
                onChange={(event) => onChange({ description: event.target.value })}
                className="w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
            </Field>
            <IconField
              className="sm:col-span-2"
              value={group.icon}
              uploading={uploading}
              onChange={(icon) => onChange({ icon })}
              onUpload={onUpload}
            />
            <Field
              label="Width"
              description="How wide the section is in the dashboard grid. You can also drag its edge handle on the canvas."
            >
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={group.width}
                onChange={(event) => onWidthChange(Number(event.target.value))}
              >
                {!groupWidthOptions.some((option) => option.value === group.width) && (
                  <option value={group.width}>Custom ({group.width} of 12)</option>
                )}
                {groupWidthOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </Field>
          </div>
          <p className="text-xs text-muted-foreground">
            {itemCount === 1 ? "1 tile is" : `${itemCount} tiles are`} assigned to this section.
          </p>
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
            <div className="flex items-center gap-2">
              <button
                className={buttonClass}
                type="button"
                aria-label={`Move ${group.title} earlier`}
                onClick={() => onMove(-1)}
              >
                <IconArrowUp size={16} aria-hidden="true" /> Move earlier
              </button>
              <button
                className={buttonClass}
                type="button"
                aria-label={`Move ${group.title} later`}
                onClick={() => onMove(1)}
              >
                <IconArrowDown size={16} aria-hidden="true" /> Move later
              </button>
            </div>
            <div className="flex items-center gap-2">
              <button className={destructiveButtonClass} type="button" onClick={onRemove}>
                <IconTrash size={16} aria-hidden="true" /> Remove section
              </button>
              <button className={primaryButtonClass} type="submit">
                Done
              </button>
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            Removing a section keeps its tiles and moves them to Ungrouped.
          </p>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** The two edit-mode dialogs, wired to the dashboard editor. */
function DashboardEditDialogs({
  selected,
  selectedGroup,
  selectedIndex,
  selectedGroupItemCount,
  editor,
  onCloseEditor,
  onCloseGroupEditor,
}: {
  selected: EditableDashboardItem | null;
  selectedGroup: EditableDashboardGroup | null;
  selectedIndex: number;
  selectedGroupItemCount: number;
  editor: ReturnType<typeof useDashboardEditor>;
  onCloseEditor: () => void;
  onCloseGroupEditor: () => void;
}) {
  return (
    <>
      {selected && (
        <TileEditorDialog
          item={selected}
          nodes={editor.nodes.map((node) => ({ id: node.id, name: node.name }))}
          widgets={editor.widgets}
          groups={editor.groups}
          uploading={editor.uploading}
          index={selectedIndex}
          itemCount={selectedGroupItemCount}
          onChange={(update) => editor.updateItem(selected.editor_id, update)}
          onRemove={() => {
            editor.removeItem(selected.editor_id);
            onCloseEditor();
          }}
          onMove={(direction) => editor.moveItem(selected.editor_id, direction)}
          onResize={(width, height) => editor.resizeItem(selected.editor_id, width, height)}
          onGroupChange={(group) => editor.moveItemToGroup(selected.editor_id, group)}
          onCreateGroup={(title) => editor.addGroup(title)}
          onUpload={editor.uploadImage}
          onClose={onCloseEditor}
        />
      )}

      {selectedGroup && (
        <GroupEditorDialog
          group={selectedGroup}
          itemCount={selectedGroupItemCount}
          uploading={editor.uploading}
          onChange={(update) => editor.updateGroup(selectedGroup.id, update)}
          onMove={(direction) => editor.moveGroup(selectedGroup.id, direction)}
          onWidthChange={(width) => editor.setGroupWidth(selectedGroup.id, width)}
          onRemove={() => {
            editor.removeGroup(selectedGroup.id);
            onCloseGroupEditor();
          }}
          onUpload={editor.uploadImage}
          onClose={onCloseGroupEditor}
        />
      )}
    </>
  );
}

export { DashboardEditDialogs };
