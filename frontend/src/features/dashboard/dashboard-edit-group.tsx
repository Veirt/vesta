import {
  IconArrowsHorizontal,
  IconDotsDiagonal,
  IconGripVertical,
  IconPencil,
  IconPlus,
} from "@tabler/icons-react";
import ReactGridLayout, { useContainerWidth, verticalCompactor } from "react-grid-layout";
import {
  useRef,
  useState,
  type CSSProperties,
  type DragEvent as ReactDragEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { DashboardItem as DashboardItemData, WidgetDescriptor } from "@/shared/schemas/core";
import {
  TILE_GRID_GAP,
  TILE_MAX_SPAN,
  TILE_ROW_HEIGHT,
  clampResizeUnits,
  gridUnitsFor,
  type EditSection,
  type TileResizeState,
  type TileSize,
} from "@/shared/lib/layout";
import { ServiceCard, WidgetCard } from "@/features/dashboard/tile-renderers";
import type { LayoutEntry } from "@/features/dashboard/use-dashboard-editor";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";

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

/**
 * Pointer + keyboard resize control for a group's column span. Self-contained:
 * owns its drag-start bookkeeping, so the parent needs no resize state.
 */
function GroupResizeHandle({
  label,
  width,
  onResize,
}: {
  label: string;
  width: number;
  onResize: (width: number) => void;
}) {
  const dragStart = useRef<{ clientX: number; span: number; groupWidth: number } | null>(null);

  function resizeFromPointer(event: ReactPointerEvent<HTMLDivElement>) {
    const start = dragStart.current;
    if (!start || start.groupWidth <= 0) return;
    const columnWidth = start.groupWidth / start.span;
    onResize(start.span + (event.clientX - start.clientX) / columnWidth);
  }

  return (
    <div
      className="dashboard-group-resize-handle"
      role="slider"
      aria-label={`Resize ${label} width`}
      aria-orientation="vertical"
      aria-valuemin={3}
      aria-valuemax={12}
      aria-valuenow={width}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") onResize(width - 1);
        if (event.key === "ArrowRight") onResize(width + 1);
      }}
      onPointerDown={(event) => {
        const groupWidth =
          event.currentTarget.closest("section")?.getBoundingClientRect().width ?? 0;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        dragStart.current = { clientX: event.clientX, span: width, groupWidth };
      }}
      onPointerMove={resizeFromPointer}
      onPointerUp={(event) => {
        dragStart.current = null;
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onPointerCancel={() => {
        dragStart.current = null;
      }}
    >
      <IconArrowsHorizontal size={12} aria-hidden="true" />
    </div>
  );
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
  onAddService,
  onAddWidget,
  onEdit,
  onLayoutChange,
  onDragStart,
  onDrag,
  onDragStop,
  widgets,
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
  onAddService: () => void;
  onAddWidget: (name: string) => void;
  onEdit: (editorId: string) => void;
  onLayoutChange: (layout: ReadonlyArray<LayoutEntry>, cols: number) => void;
  onDragStart: () => void;
  onDrag: (event: Event) => void;
  onDragStop: () => void;
  widgets: WidgetDescriptor[];
}) {
  // measureBeforeMount so the grid never renders at the initialWidth and then
  // animates every tile to the measured width on entering edit mode.
  const { width, containerRef, mounted } = useContainerWidth({
    initialWidth: 640,
    measureBeforeMount: true,
  });
  const cols = width < 440 ? 1 : 12;
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
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              className="dashboard-group-edit-button"
              type="button"
              aria-label={`Add a tile to ${headingLabel}`}
            >
              <IconPlus size={15} aria-hidden="true" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={onAddService}>Add service</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuLabel>Widgets</DropdownMenuLabel>
            {widgets.length === 0 ? (
              <DropdownMenuItem disabled>No widgets available</DropdownMenuItem>
            ) : (
              widgets.map((widget) => (
                <DropdownMenuItem key={widget.name} onSelect={() => onAddWidget(widget.name)}>
                  {widget.title}
                </DropdownMenuItem>
              ))
            )}
          </DropdownMenuContent>
        </DropdownMenu>
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
          No tiles in this section yet. Add one with the + button above.
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
      <GroupResizeHandle label={headingLabel} width={section.width} onResize={onGroupResize} />
    </section>
  );
}

export { DashboardEditGroup };
