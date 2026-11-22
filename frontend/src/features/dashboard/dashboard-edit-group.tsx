import { IconDotsDiagonal, IconGripVertical, IconPencil, IconPlus } from "@tabler/icons-react";
import ReactGridLayout, { useContainerWidth, verticalCompactor } from "react-grid-layout";
import {
  useRef,
  useState,
  type CSSProperties,
  type DragEvent as ReactDragEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { AddTileMenu } from "@/features/dashboard/add-tile-menu";
import type { LayoutEntry } from "@/features/dashboard/dashboard-draft-ops";
import { EditTile } from "@/features/dashboard/edit-tile";
import { GroupResizeHandle } from "@/features/dashboard/group-resize-handle";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import {
  MAX_GRID_SIZE,
  TILE_GRID_GAP,
  TILE_MAX_SPAN,
  TILE_ROW_HEIGHT,
  clampResizeUnits,
  gridUnitsFor,
  type EditSection,
  type TileResizeState,
  type TileSize,
} from "@/shared/lib/layout";
import type { WidgetDescriptor } from "@/shared/schemas/core";

export function DashboardEditGroup({
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
  const { width, containerRef, mounted } = useContainerWidth({
    initialWidth: 640,
    measureBeforeMount: true,
  });
  const cols = width < 440 ? 1 : MAX_GRID_SIZE;
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
        <AddTileMenu
          widgets={widgets}
          align="end"
          trigger={
            <button
              className="dashboard-group-edit-button"
              type="button"
              aria-label={`Add a tile to ${headingLabel}`}
            >
              <IconPlus size={15} aria-hidden="true" />
            </button>
          }
          onAddService={onAddService}
          onAddWidget={onAddWidget}
        />
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
              style={{
                height: `${item.height * TILE_ROW_HEIGHT + Math.max(0, item.height - 1) * TILE_GRID_GAP}px`,
              }}
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
              rowHeight: TILE_ROW_HEIGHT,
              margin: [TILE_GRID_GAP, TILE_GRID_GAP],
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
                  aria-valuemax={MAX_GRID_SIZE - item.x}
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
