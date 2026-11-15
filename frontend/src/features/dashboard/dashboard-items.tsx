import {
  IconArrowsHorizontal,
  IconArrowDown,
  IconArrowUp,
  IconChevronDown,
  IconDotsDiagonal,
  IconGripVertical,
  IconPencil,
  IconPlus,
  IconTrash,
} from "@tabler/icons-react";
import ReactGridLayout, { useContainerWidth, verticalCompactor } from "react-grid-layout";
import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent as ReactDragEvent,
  type FormEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import type {
  DashboardGroup,
  DashboardItem as DashboardItemData,
} from "@/features/dashboard/schema";
import { ServiceCard, WidgetCard } from "@/features/dashboard/tile-renderers";
import { useDashboardEditor, type LayoutEntry } from "@/features/dashboard/use-dashboard-editor";
import {
  DEFAULT_GROUP_SPAN,
  MIN_GROUP_SPAN,
  minSizeFor,
  type TileSize,
} from "@/features/dashboard/tile-sizes";
import { validateDashboardGroups, validateDashboardItems } from "@/features/dashboard/validation";
import { Field } from "@/features/settings/components/form-field";
import { IconField } from "@/features/settings/components/icon-field";
import { WidgetConfigFields } from "@/features/settings/components/widget-config-fields";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
  primaryButtonClass,
} from "@/features/settings/components/styles";
import { fetchAuthStatus, testPingUrl } from "@/features/settings/api";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

const sizeOptions = [
  { value: "3x1", label: "Compact", width: 3, height: 1 },
  { value: "4x1", label: "Standard", width: 4, height: 1 },
  { value: "6x1", label: "Wide", width: 6, height: 1 },
  { value: "4x2", label: "Boxed", width: 4, height: 2 },
  { value: "6x2", label: "Large", width: 6, height: 2 },
  { value: "12x3", label: "Full width", width: 12, height: 3 },
];

function editableAsDashboardItem(item: EditableDashboardItem): DashboardItemData {
  if (item.kind === "service") {
    return {
      kind: "service",
      id: item.id,
      title: item.title,
      group: item.group,
      icon: item.icon || null,
      href: item.href || null,
      show_destination: item.show_destination,
      x: item.x,
      y: item.y,
      width: item.width,
      height: item.height,
      has_ping: Boolean(item.ping_url.trim()),
    };
  }
  return {
    kind: "widget",
    id: item.id,
    title: item.title,
    group: item.group,
    name: item.name,
    x: item.x,
    y: item.y,
    width: item.width,
    height: item.height,
    refresh_interval_secs: null,
  };
}

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

type GroupableItem = { group: string | null; x: number | null; y: number | null };
type GroupMetadata = {
  id: string;
  title: string;
  description?: string | null;
  icon?: string | null;
  width?: number | null;
};
type DashboardGroupSection<T> = {
  id: string;
  label: string | null;
  description: string | null;
  icon: string | null;
  width: number;
  items: T[];
};
type EditSection = DashboardGroupSection<EditableDashboardItem>;

function clampGroupSpan(width: number | null | undefined): number {
  return Math.max(MIN_GROUP_SPAN, Math.min(12, Math.round(width ?? DEFAULT_GROUP_SPAN)));
}

/**
 * Grid metrics mirror the `gridConfig` passed to ReactGridLayout (12 columns,
 * 72px rows, 12px gap) so pointer deltas can be snapped to whole cells live.
 */
const TILE_GRID_GAP = 12;
const TILE_ROW_HEIGHT = 72;
const TILE_MAX_SPAN = 12;

function clampResizeUnits(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(Math.max(1, max), Math.round(value)));
}

function gridUnitsFor(widthPx: number, gap: number, cellPx: number): number {
  return (widthPx + gap) / (cellPx + gap);
}

type TileResizeState = {
  editorId: string;
  startX: number;
  startY: number;
  baseWidth: number;
  baseHeight: number;
  x: number;
  minWidth: number;
  minHeight: number;
  columnWidth: number;
  width: number;
  height: number;
};

function groupDashboardItems<T extends GroupableItem>(
  items: T[],
  groups: GroupMetadata[] = [],
  includeEmptyGroups = false,
): DashboardGroupSection<T>[] {
  const ordered = [...items].sort(
    (left, right) => (left.y ?? 0) - (right.y ?? 0) || (left.x ?? 0) - (right.x ?? 0),
  );
  const metadata = new Map(groups.map((group) => [group.id, group]));
  const grouped = new Map<string, T[]>();
  for (const item of ordered) {
    const key = item.group?.trim() ?? "";
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  // The id-less Ungrouped section keeps its metadata position when one exists,
  // otherwise it stays first for legacy saved groups.
  const sectionOrder: string[] = metadata.has("") ? [] : [""];
  for (const group of groups) {
    if (!sectionOrder.includes(group.id)) sectionOrder.push(group.id);
  }
  const seen = new Set(sectionOrder);
  for (const key of grouped.keys()) {
    if (!seen.has(key)) sectionOrder.push(key);
  }
  const sections = sectionOrder.flatMap((id) => {
    const groupItems = grouped.get(id) ?? [];
    if (groupItems.length === 0 && !includeEmptyGroups) return [];
    const group = metadata.get(id);
    return [
      {
        id,
        label: id ? (group?.title ?? id) : null,
        description: id ? (group?.description ?? null) : null,
        icon: id ? (group?.icon ?? null) : null,
        width: clampGroupSpan(group?.width),
        items: groupItems,
      },
    ];
  });
  if (sections.length === 1 && sections[0].id === "") {
    return [{ ...sections[0], width: 12 }];
  }
  return sections;
}

function DashboardEditGroup({
  section,
  groupDragActive,
  isGroupDragSource,
  minSizeFor,
  onGroupDragStart,
  onGroupDragEnd,
  onGroupDragEnter,
  onGroupResize,
  onTileResize,
  onEditGroup,
  onEdit,
  onLayoutChange,
  onDragStart,
  onDrag,
  onDragStop,
}: {
  section: EditSection;
  groupDragActive: boolean;
  isGroupDragSource: boolean;
  minSizeFor: (item: EditableDashboardItem) => TileSize;
  onGroupDragStart: () => void;
  onGroupDragEnd: () => void;
  onGroupDragEnter: (placement: "before" | "after") => void;
  onGroupResize: (width: number) => void;
  onTileResize: (editorId: string, width: number, height: number) => void;
  onEditGroup: () => void;
  onEdit: (editorId: string) => void;
  onLayoutChange: (layout: ReadonlyArray<LayoutEntry>, cols: number) => void;
  onDragStart: () => void;
  onDrag: (event: Event) => void;
  onDragStop: () => void;
}) {
  const { width, containerRef, mounted } = useContainerWidth({ initialWidth: 640 });
  const cols = width < 440 ? 1 : 12;
  const resizeStart = useRef<{ clientX: number; span: number; groupWidth: number } | null>(null);
  const tileResize = useRef<TileResizeState | null>(null);
  const [tileResizing, setTileResizing] = useState(false);
  const headingLabel = section.id ? (section.label ?? section.id) : "Ungrouped";

  function startTileResize(event: ReactPointerEvent<HTMLDivElement>, item: EditableDashboardItem) {
    event.preventDefault();
    event.stopPropagation();
    const min = minSizeFor(item);
    tileResize.current = {
      editorId: item.editor_id,
      startX: event.clientX,
      startY: event.clientY,
      baseWidth: item.width,
      baseHeight: item.height,
      x: item.x,
      minWidth: min.width,
      minHeight: min.height,
      columnWidth: (width - TILE_GRID_GAP * (cols - 1)) / cols,
      width: item.width,
      height: item.height,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    setTileResizing(true);
  }

  function tileResizeFromPointer(event: ReactPointerEvent<HTMLDivElement>) {
    const start = tileResize.current;
    if (!start) return;
    const baseWidthPx = start.columnWidth * start.baseWidth + (start.baseWidth - 1) * TILE_GRID_GAP;
    const baseHeightPx =
      TILE_ROW_HEIGHT * start.baseHeight + (start.baseHeight - 1) * TILE_GRID_GAP;
    const nextWidth = clampResizeUnits(
      gridUnitsFor(baseWidthPx + (event.clientX - start.startX), TILE_GRID_GAP, start.columnWidth),
      start.minWidth,
      TILE_MAX_SPAN - start.x,
    );
    const nextHeight = clampResizeUnits(
      gridUnitsFor(baseHeightPx + (event.clientY - start.startY), TILE_GRID_GAP, TILE_ROW_HEIGHT),
      start.minHeight,
      TILE_MAX_SPAN,
    );
    if (nextWidth === start.width && nextHeight === start.height) return;
    start.width = nextWidth;
    start.height = nextHeight;
    onTileResize(start.editorId, nextWidth, nextHeight);
  }

  function stopTileResize(event: ReactPointerEvent<HTMLDivElement>) {
    if (!tileResize.current) return;
    tileResize.current = null;
    setTileResizing(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  function dropPlacement(event: ReactDragEvent): "before" | "after" {
    const rect = event.currentTarget.getBoundingClientRect();
    return event.clientX < rect.left + rect.width / 2 ? "before" : "after";
  }

  function resizeFromPointer(event: ReactPointerEvent<HTMLDivElement>) {
    const start = resizeStart.current;
    if (!start || start.groupWidth <= 0) return;
    const columnWidth = start.groupWidth / start.span;
    onGroupResize(start.span + (event.clientX - start.clientX) / columnWidth);
  }

  return (
    <section
      className={`dashboard-edit-group${isGroupDragSource ? " dashboard-edit-group-dragging" : ""}`}
      ref={containerRef}
      style={{ "--group-span": section.width } as CSSProperties}
      data-tile-resize={tileResizing || undefined}
      onDragOver={(event) => {
        if (!groupDragActive || isGroupDragSource) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
      }}
      onDragEnter={(event) => {
        if (!groupDragActive || isGroupDragSource) return;
        event.preventDefault();
        onGroupDragEnter(dropPlacement(event));
      }}
      onDrop={(event) => {
        if (!groupDragActive || isGroupDragSource) return;
        event.preventDefault();
        onGroupDragEnter(dropPlacement(event));
      }}
    >
      <div className="dashboard-group-heading-row">
        <div className="dashboard-group-heading-copy">
          <button
            className="dashboard-group-drag-button"
            type="button"
            draggable
            onDragStart={(event) => {
              event.dataTransfer.effectAllowed = "move";
              event.dataTransfer.setData("text/plain", section.id);
              onGroupDragStart();
            }}
            onDragEnd={onGroupDragEnd}
            aria-label={`Move ${headingLabel} section`}
          >
            <IconGripVertical size={15} aria-hidden="true" />
          </button>
          {section.icon && <img className="dashboard-group-icon" src={section.icon} alt="" />}
          <div className="min-w-0">
            <h3 className="dashboard-group-heading">{headingLabel}</h3>
            {section.description && (
              <p className="dashboard-group-description">{section.description}</p>
            )}
          </div>
        </div>
        {section.id && (
          <button
            className="dashboard-group-edit-button"
            type="button"
            aria-label={`Edit ${section.label} section`}
            onClick={onEditGroup}
          >
            <IconPencil size={15} aria-hidden="true" />
          </button>
        )}
      </div>
      {section.items.length === 0 && (
        <p className="dashboard-group-empty">
          No tiles in this section yet. Assign a tile to add it.
        </p>
      )}
      {cols === 1 ? (
        <div className="dashboard-mobile-list">
          {section.items.map((item) => (
            <div
              className="dashboard-mobile-item"
              key={item.editor_id}
              style={{ height: `${item.height * 72 + Math.max(0, item.height - 1) * 12}px` }}
            >
              <EditTile item={item} onEdit={() => onEdit(item.editor_id)} />
            </div>
          ))}
        </div>
      ) : (
        mounted && (
          <ReactGridLayout
            width={width}
            layout={section.items.map((item) => ({
              i: item.editor_id,
              x: item.x,
              y: item.y,
              w: item.width,
              h: item.height,
              minW: minSizeFor(item).width,
              minH: minSizeFor(item).height,
            }))}
            gridConfig={{
              cols,
              rowHeight: 72,
              margin: [12, 12],
              containerPadding: [0, 0],
            }}
            compactor={verticalCompactor}
            dragConfig={{
              enabled: true,
              cancel: "button,input,select,textarea,details,summary,[role=button],[role=slider]",
              allowMobileScroll: true,
            }}
            onDragStart={onDragStart}
            onDrag={(_layout, _oldItem, _newItem, _placeholder, event) => onDrag(event)}
            onDragStop={(layout) => {
              onDragStop();
              onLayoutChange(layout, cols);
            }}
            onLayoutChange={(layout) => onLayoutChange(layout, cols)}
          >
            {section.items.map((item) => (
              <div className="min-w-0" key={item.editor_id}>
                <EditTile item={item} onEdit={() => onEdit(item.editor_id)} />
                <div
                  className="dashboard-tile-resize-handle"
                  role="slider"
                  aria-label={`Resize ${item.title} tile`}
                  aria-valuemin={minSizeFor(item).width}
                  aria-valuemax={12 - item.x}
                  aria-valuenow={item.width}
                  aria-valuetext={`Width ${item.width} columns, height ${item.height} rows`}
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "ArrowRight") {
                      event.preventDefault();
                      onTileResize(item.editor_id, item.width + 1, item.height);
                    } else if (event.key === "ArrowLeft") {
                      event.preventDefault();
                      onTileResize(item.editor_id, item.width - 1, item.height);
                    } else if (event.key === "ArrowDown") {
                      event.preventDefault();
                      onTileResize(item.editor_id, item.width, item.height + 1);
                    } else if (event.key === "ArrowUp") {
                      event.preventDefault();
                      onTileResize(item.editor_id, item.width, item.height - 1);
                    }
                  }}
                  onPointerDown={(event) => startTileResize(event, item)}
                  onPointerMove={tileResizeFromPointer}
                  onPointerUp={stopTileResize}
                  onPointerCancel={stopTileResize}
                >
                  <IconDotsDiagonal size={13} aria-hidden="true" />
                </div>
              </div>
            ))}
          </ReactGridLayout>
        )
      )}
      <div
        className="dashboard-group-resize-handle"
        role="slider"
        aria-label={`Resize ${headingLabel} width`}
        aria-orientation="vertical"
        aria-valuemin={3}
        aria-valuemax={12}
        aria-valuenow={section.width}
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft") onGroupResize(section.width - 1);
          if (event.key === "ArrowRight") onGroupResize(section.width + 1);
        }}
        onPointerDown={(event) => {
          const groupWidth =
            event.currentTarget.closest("section")?.getBoundingClientRect().width ?? 0;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          resizeStart.current = { clientX: event.clientX, span: section.width, groupWidth };
        }}
        onPointerMove={resizeFromPointer}
        onPointerUp={(event) => {
          resizeStart.current = null;
          event.currentTarget.releasePointerCapture(event.pointerId);
        }}
        onPointerCancel={() => {
          resizeStart.current = null;
        }}
      >
        <IconArrowsHorizontal size={12} aria-hidden="true" />
      </div>
    </section>
  );
}

function eventClientY(event: Event): number | null {
  if ("clientY" in event && typeof event.clientY === "number") return event.clientY;
  if ("touches" in event) {
    const touchEvent = event as TouchEvent;
    return touchEvent.touches[0]?.clientY ?? touchEvent.changedTouches[0]?.clientY ?? null;
  }
  return null;
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
            <TileSizeField item={item} min={minSizeFor(item, widgets)} onResize={onResize} />
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

function EditTile({ item, onEdit }: { item: EditableDashboardItem; onEdit: () => void }) {
  return (
    <div className="dashboard-edit-tile">
      <button
        className="dashboard-tile-edit-button"
        type="button"
        aria-label={`Edit ${item.title}`}
        onClick={onEdit}
      >
        <IconPencil size={16} aria-hidden="true" />
      </button>
      {item.kind === "service" ? (
        <ServiceCard
          service={editableAsDashboardItem(item) as Extract<DashboardItemData, { kind: "service" }>}
          editing
        />
      ) : (
        <WidgetCard
          widget={editableAsDashboardItem(item) as Extract<DashboardItemData, { kind: "widget" }>}
          editing
        />
      )}
    </div>
  );
}

function DashboardItems({
  items,
  groups,
  query,
  loading,
}: {
  items: DashboardItemData[];
  groups: DashboardGroup[];
  query: string;
  loading: boolean;
}) {
  const [authenticated, setAuthenticated] = useState(false);
  const [editing, setEditing] = useState(false);
  const [selectedEditorId, setSelectedEditorId] = useState<string | null>(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [draggingTile, setDraggingTile] = useState(false);
  const [draggingGroupId, setDraggingGroupId] = useState<string | null>(null);
  const editor = useDashboardEditor(editing);
  const edgeScrollDirection = useRef(0);
  const edgeScrollStep = useRef(0);
  const edgeScrollFrame = useRef<number | null>(null);

  useEffect(() => {
    let active = true;

    const refreshAuth = () => {
      fetchAuthStatus()
        .then((status) => {
          if (active) setAuthenticated(status.authenticated);
        })
        .catch(() => {
          if (active) setAuthenticated(false);
        });
    };

    refreshAuth();
    window.addEventListener("focus", refreshAuth);
    document.addEventListener("visibilitychange", refreshAuth);
    return () => {
      active = false;
      window.removeEventListener("focus", refreshAuth);
      document.removeEventListener("visibilitychange", refreshAuth);
    };
  }, []);

  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleItems = items.filter((item) => {
    const group = groups.find((entry) => entry.id === item.group);
    return (
      !normalizedQuery ||
      `${item.id} ${item.title} ${item.group ?? ""} ${group?.title ?? ""} ${group?.description ?? ""} ${item.kind === "widget" ? item.name : (item.href ?? "")}`
        .toLocaleLowerCase()
        .includes(normalizedQuery)
    );
  });
  const draftSections = groupDashboardItems(editor.items, editor.groups, true);
  const orderedDraft = draftSections.flatMap((section) => section.items);
  const selected = editor.items.find((item) => item.editor_id === selectedEditorId) ?? null;
  const selectedGroup = editor.groups.find((group) => group.id === selectedGroupId) ?? null;
  const selectedGroupItems = selected
    ? orderedDraft.filter((item) => (item.group ?? "") === (selected.group ?? ""))
    : [];
  const selectedIndex = selected
    ? selectedGroupItems.findIndex((item) => item.editor_id === selected.editor_id)
    : -1;
  const validationIssues = [
    ...validateDashboardItems(editor.items, editor.widgets, editor.nodes),
    ...validateDashboardGroups(editor.groups),
  ];
  const visibleSections = groupDashboardItems(normalizedQuery ? visibleItems : items, groups, true);

  function stopWindowEdgeScroll() {
    edgeScrollDirection.current = 0;
    if (edgeScrollFrame.current !== null) {
      cancelAnimationFrame(edgeScrollFrame.current);
      edgeScrollFrame.current = null;
    }
  }

  function handleWindowEdgeScroll(event: Event) {
    const clientY = eventClientY(event);
    if (clientY === null) return;
    const edgeSize = 72;
    const distanceToEdge = Math.min(clientY, window.innerHeight - clientY);
    if (distanceToEdge >= edgeSize) {
      edgeScrollDirection.current = 0;
      return;
    }

    edgeScrollDirection.current = clientY < edgeSize ? -1 : 1;
    edgeScrollStep.current = Math.max(2, Math.ceil(((edgeSize - distanceToEdge) / edgeSize) * 10));
    if (edgeScrollFrame.current !== null) return;

    const scroll = () => {
      const direction = edgeScrollDirection.current;
      if (!direction) {
        edgeScrollFrame.current = null;
        return;
      }

      const maxScroll = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
      const canScroll = direction < 0 ? window.scrollY > 0 : window.scrollY < maxScroll;
      if (!canScroll) {
        edgeScrollFrame.current = null;
        return;
      }

      window.scrollBy(0, direction * edgeScrollStep.current);
      edgeScrollFrame.current = requestAnimationFrame(scroll);
    };
    edgeScrollFrame.current = requestAnimationFrame(scroll);
  }

  function handleLayoutChange(layout: ReadonlyArray<LayoutEntry>, cols: number) {
    editor.updateLayout([...layout], cols);
  }

  useEffect(
    () => () => {
      edgeScrollDirection.current = 0;
      if (edgeScrollFrame.current !== null) cancelAnimationFrame(edgeScrollFrame.current);
    },
    [],
  );

  function addService() {
    setSelectedEditorId(editor.addService());
  }

  function addWidget(name: string) {
    const added = editor.addWidget(name);
    if (added) setSelectedEditorId(added);
  }

  function addGroup() {
    setSelectedGroupId(editor.addGroup());
  }

  function updateSelected(update: object) {
    if (selected) editor.updateItem(selected.editor_id, update);
  }

  function removeSelected() {
    if (!selected) return;
    editor.removeItem(selected.editor_id);
    setSelectedEditorId(null);
  }

  function removeSelectedGroup() {
    if (!selectedGroup) return;
    editor.removeGroup(selectedGroup.id);
    setSelectedGroupId(null);
  }

  async function saveDashboard() {
    const saved = await editor.save();
    if (saved) setEditing(false);
  }

  const compactViewItems = normalizedQuery ? visibleItems : items;
  const showEditorDraft = editing && editor.ready;

  return (
    <section id="dashboard-items" aria-labelledby="dashboard-items-heading" className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 id="dashboard-items-heading" className="text-lg font-semibold tracking-tight">
            Apps and widgets
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Keep service links and live widgets side by side, in any order.
          </p>
        </div>
        {authenticated && !editing && (
          <button className={buttonClass} type="button" onClick={() => setEditing(true)}>
            <IconPencil size={16} aria-hidden="true" /> Edit dashboard
          </button>
        )}
        {editing && (
          <div className="flex flex-wrap items-center gap-2">
            <button
              className={buttonClass}
              type="button"
              onClick={() => {
                setEditing(false);
                setSelectedEditorId(null);
              }}
              disabled={editor.saving}
            >
              Discard
            </button>
            <button
              className={primaryButtonClass}
              type="button"
              onClick={() => void saveDashboard()}
              disabled={
                !editor.dirty || editor.saving || editor.loading || validationIssues.length > 0
              }
              aria-describedby={validationIssues.length > 0 ? "dashboard-validation" : undefined}
            >
              {editor.saving ? "Saving…" : "Save dashboard"}
            </button>
          </div>
        )}
      </div>

      {editing && (
        <div className="dashboard-edit-toolbar">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                className={buttonClass}
                type="button"
                disabled={!editor.ready || editor.loading}
              >
                <IconPlus size={16} aria-hidden="true" /> Add tile
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onSelect={addService}>Add service</DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuLabel>Widgets</DropdownMenuLabel>
              {editor.widgets.length === 0 ? (
                <DropdownMenuItem disabled>No widgets available</DropdownMenuItem>
              ) : (
                editor.widgets.map((widget) => (
                  <DropdownMenuItem key={widget.name} onSelect={() => addWidget(widget.name)}>
                    {widget.title}
                  </DropdownMenuItem>
                ))
              )}
            </DropdownMenuContent>
          </DropdownMenu>
          <button
            className={buttonClass}
            type="button"
            onClick={addGroup}
            disabled={!editor.ready || editor.loading}
          >
            <IconPlus size={16} aria-hidden="true" /> Add group
          </button>
          <span className="text-xs text-muted-foreground">
            {editor.loading && !editor.ready
              ? "Loading your dashboard layout…"
              : "Drag any tile to arrange it. Use the pencil to edit, group, or resize; on narrow screens, use the edit panel to move and size tiles."}
          </span>
        </div>
      )}

      {editing && validationIssues.length > 0 && (
        <div
          id="dashboard-validation"
          className="grid gap-1 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          <p>Fix these dashboard details before saving:</p>
          <ul className="list-inside list-disc">
            {validationIssues.map((issue, index) => (
              <li key={`${issue.id}-${index}`}>
                {issue.title || "Untitled tile"}: {issue.message}
              </li>
            ))}
          </ul>
        </div>
      )}

      {(editor.error || editor.loadError) && editing && (
        <div
          className="grid justify-items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          <p>{editor.error || editor.loadError}</p>
          {editor.loadError && (
            <button className={buttonClass} type="button" onClick={editor.retryLoad}>
              Try again
            </button>
          )}
          {(editor.error.includes("sign in") || editor.loadError.includes("401")) && (
            <a className="underline underline-offset-4" href="/settings">
              Open Settings to sign in
            </a>
          )}
        </div>
      )}
      {editor.notice && editing && (
        <p className="text-sm text-muted-foreground" role="status">
          {editor.notice}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground" role="status">
          Loading dashboard tiles…
        </p>
      ) : showEditorDraft ? (
        editor.loadError ? null : editor.items.length === 0 && editor.groups.length === 0 ? (
          <div className="dashboard-empty-state">
            <p>No tiles yet. Add a service link or a widget to start your dashboard.</p>
          </div>
        ) : (
          <div className="dashboard-grid-wrap">
            <div className="dashboard-edit-groups">
              {draftSections.map((section) => (
                <DashboardEditGroup
                  key={section.id}
                  section={section}
                  groupDragActive={draggingGroupId !== null}
                  isGroupDragSource={draggingGroupId === section.id}
                  minSizeFor={(item) => minSizeFor(item, editor.widgets)}
                  onGroupDragStart={() => {
                    // The Ungrouped section needs its metadata slot before any
                    // reorder or width change can find it in the groups array.
                    if (section.id === "") editor.ensureUngroupedGroup();
                    setDraggingGroupId(section.id);
                  }}
                  onGroupDragEnd={() => setDraggingGroupId(null)}
                  onGroupDragEnter={(placement) => {
                    if (draggingGroupId === null) return;
                    // Dropping onto/near Ungrouped requires its metadata slot to exist.
                    if (section.id === "") editor.ensureUngroupedGroup();
                    editor.moveGroupTo(draggingGroupId, section.id, placement);
                  }}
                  onGroupResize={(width) => {
                    if (section.id === "") editor.ensureUngroupedGroup();
                    editor.setGroupWidth(section.id, width);
                  }}
                  onTileResize={(editorId, width, height) =>
                    editor.resizeItem(editorId, width, height)
                  }
                  onEditGroup={() => setSelectedGroupId(section.id)}
                  onEdit={setSelectedEditorId}
                  onLayoutChange={handleLayoutChange}
                  onDragStart={() => setDraggingTile(true)}
                  onDrag={handleWindowEdgeScroll}
                  onDragStop={() => {
                    stopWindowEdgeScroll();
                    setDraggingTile(false);
                  }}
                />
              ))}
            </div>
            {draggingTile && <div className="dashboard-edit-runway" aria-hidden="true" />}
          </div>
        )
      ) : compactViewItems.length === 0 ? (
        <div className="dashboard-empty-state">
          <p>
            {normalizedQuery
              ? "No apps or widgets match your search."
              : "No apps or widgets yet. Sign in and choose Edit dashboard to add your first tile."}
          </p>
        </div>
      ) : (
        <div
          className={
            visibleSections.some((section) => section.label) ? "dashboard-groups" : undefined
          }
        >
          {visibleSections.map((section) => (
            <section
              className="dashboard-view-group"
              key={section.id}
              style={{ "--group-span": section.width } as CSSProperties}
            >
              {section.label && (
                <div className="dashboard-group-heading-row">
                  <div className="dashboard-group-heading-copy">
                    {section.icon && (
                      <img className="dashboard-group-icon" src={section.icon} alt="" />
                    )}
                    <div className="min-w-0">
                      <h3 className="dashboard-group-heading">{section.label}</h3>
                      {section.description && (
                        <p className="dashboard-group-description">{section.description}</p>
                      )}
                    </div>
                  </div>
                </div>
              )}
              {section.items.length === 0 ? (
                <p className="dashboard-group-empty">
                  {normalizedQuery
                    ? "No tiles in this section match the search."
                    : "No tiles in this section yet."}
                </p>
              ) : (
                <div className={`dashboard-grid${normalizedQuery ? " dashboard-grid-search" : ""}`}>
                  {section.items.map((item) =>
                    item.kind === "service" ? (
                      <ServiceCard key={item.id} service={item} />
                    ) : (
                      <WidgetCard key={item.id} widget={item} />
                    ),
                  )}
                </div>
              )}
            </section>
          ))}
        </div>
      )}

      {selected && (
        <TileEditorDialog
          item={selected}
          nodes={editor.nodes.map((node) => ({ id: node.id, name: node.name }))}
          widgets={editor.widgets}
          groups={editor.groups}
          uploading={editor.uploading}
          index={selectedIndex}
          itemCount={selectedGroupItems.length}
          onChange={updateSelected}
          onRemove={removeSelected}
          onMove={(direction) => editor.moveItem(selected.editor_id, direction)}
          onResize={(width, height) => editor.resizeItem(selected.editor_id, width, height)}
          onGroupChange={(group) => editor.moveItemToGroup(selected.editor_id, group)}
          onCreateGroup={(title) => editor.addGroup(title)}
          onUpload={editor.uploadImage}
          onClose={() => setSelectedEditorId(null)}
        />
      )}

      {selectedGroup && (
        <GroupEditorDialog
          group={selectedGroup}
          itemCount={editor.items.filter((item) => item.group === selectedGroup.id).length}
          uploading={editor.uploading}
          onChange={(update) => editor.updateGroup(selectedGroup.id, update)}
          onMove={(direction) => editor.moveGroup(selectedGroup.id, direction)}
          onWidthChange={(width) => editor.setGroupWidth(selectedGroup.id, width)}
          onRemove={removeSelectedGroup}
          onUpload={editor.uploadImage}
          onClose={() => setSelectedGroupId(null)}
        />
      )}
    </section>
  );
}

export { DashboardItems };
