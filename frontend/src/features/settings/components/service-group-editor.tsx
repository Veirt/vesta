import {
  IconArrowDown,
  IconArrowUp,
  IconGripVertical,
  IconPlus,
  IconTrash,
} from "@tabler/icons-react";
import ReactGridLayout, { noCompactor, useContainerWidth } from "react-grid-layout";
import { Input } from "@/components/ui/input";
import { Field } from "@/features/settings/components/form-field";
import { ServiceEditor } from "@/features/settings/components/service-editor";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
} from "@/features/settings/components/styles";
import type { EditableGroup, EditableNode, EditableService } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

function ServiceGroupEditor({
  group,
  groupIndex,
  groupsCount,
  nodes,
  widgets,
  uploading,
  initiallyOpen,
  onChange,
  onRemove,
  onMoveGroup,
  onAddService,
  onMoveService,
  onResizeService,
  onLayoutChange,
  onUpload,
}: {
  group: EditableGroup;
  groupIndex: number;
  groupsCount: number;
  nodes: EditableNode[];
  widgets: WidgetDescriptor[];
  uploading: boolean;
  initiallyOpen: boolean;
  onChange: (update: Partial<EditableGroup>) => void;
  onRemove: () => void;
  onMoveGroup: (direction: -1 | 1) => void;
  onAddService: () => void;
  onMoveService: (serviceIndex: number, direction: -1 | 1) => void;
  onResizeService: (serviceId: string, dimension: "width" | "height", change: -1 | 1) => void;
  onLayoutChange: (
    layout: Array<{ i: string; x: number; y: number; w: number; h: number }>,
  ) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  const { width, containerRef, mounted } = useContainerWidth({ initialWidth: 960 });
  const cols = width < 560 ? 2 : width < 820 ? 6 : 12;
  const factor = 12 / cols;
  const layout = group.services.map((service, index) => {
    const w = Math.max(1, Math.min(cols, Math.round(service.width / factor)));
    return {
      i: service.editor_id,
      x: cols === 2 ? 0 : Math.min(cols - w, Math.floor(service.x / factor)),
      y:
        cols === 2
          ? group.services.slice(0, index).reduce((total, previous) => total + previous.height, 0)
          : service.y,
      w: cols === 2 ? 2 : w,
      h: service.height,
      minW: 1,
      minH: 1,
    };
  });

  function handleLayoutChange(
    next: ReadonlyArray<{ i: string; x: number; y: number; w: number; h: number }>,
  ) {
    if (cols === 2) return;
    onLayoutChange(
      next.map((item) => ({
        i: item.i,
        x: Math.round(item.x * factor),
        y: item.y,
        w: Math.max(1, Math.min(12, Math.round(item.w * factor))),
        h: item.h,
      })),
    );
  }

  function updateService(index: number, update: Partial<EditableService>) {
    onChange({
      services: group.services.map((service, serviceIndex) =>
        serviceIndex === index ? { ...service, ...update } : service,
      ),
    });
  }

  function removeService(index: number) {
    onChange({ services: group.services.filter((_, serviceIndex) => serviceIndex !== index) });
  }

  return (
    <section
      className="rounded-lg border border-border/70 bg-background/40 p-4 sm:p-5"
      aria-label={`Service group ${group.name}`}
    >
      <div className="flex flex-wrap items-center gap-2">
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-base font-semibold">{group.name || "New group"}</h3>
          <p className="truncate text-xs text-muted-foreground">{group.id || "group-id"}</p>
        </div>
        {groupIndex > 0 && (
          <button
            className={buttonClass}
            type="button"
            onClick={() => onMoveGroup(-1)}
            aria-label={`Move ${group.name} group up`}
          >
            <IconArrowUp size={16} aria-hidden="true" /> Move up
          </button>
        )}
        {groupIndex < groupsCount - 1 && (
          <button
            className={buttonClass}
            type="button"
            onClick={() => onMoveGroup(1)}
            aria-label={`Move ${group.name} group down`}
          >
            <IconArrowDown size={16} aria-hidden="true" /> Move down
          </button>
        )}
        <button
          className={destructiveButtonClass}
          type="button"
          onClick={onRemove}
          aria-label={`Remove ${group.name} group`}
        >
          <IconTrash size={16} aria-hidden="true" /> Remove group
        </button>
      </div>

      <details
        className="group mt-4 rounded-md border border-border/70 bg-card/30"
        open={initiallyOpen}
      >
        <summary className="flex min-h-10 cursor-pointer list-none items-center justify-between gap-2 px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
          <span>Group settings and service layout</span>
          <span className="text-xs text-muted-foreground group-open:hidden">Show</span>
          <span className="hidden text-xs text-muted-foreground group-open:inline">Hide</span>
        </summary>
        <div className="grid gap-5 border-t border-border/70 p-3 sm:p-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="Group ID">
              <Input
                value={group.id}
                required
                onChange={(event) => onChange({ id: event.target.value })}
                className={inputClass}
              />
            </Field>
            <Field label="Group name">
              <Input
                value={group.name}
                required
                onChange={(event) => onChange({ name: event.target.value })}
                className={inputClass}
              />
            </Field>
          </div>

          <div className="grid gap-2">
            <div>
              <h4 className="text-sm font-medium">Service layout</h4>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">
                Drag a card to rearrange it or use its resize handle on wider screens. On narrow
                screens, use the move and size controls below; they also work with a keyboard.
              </p>
            </div>
            {group.services.length === 0 ? (
              <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
                This group is empty. Add a link card or widget to begin.
              </p>
            ) : (
              <div
                className="rounded-md border border-border/70 bg-background/40 p-3"
                ref={containerRef}
              >
                {mounted && (
                  <ReactGridLayout
                    width={width}
                    layout={layout}
                    gridConfig={{ cols, rowHeight: 68, margin: [10, 10], containerPadding: [0, 0] }}
                    compactor={noCompactor}
                    dragConfig={{
                      enabled: cols > 2,
                      handle: ".layout-drag-handle",
                      cancel: "button,input,select,textarea,a,details,summary",
                      allowMobileScroll: true,
                    }}
                    resizeConfig={{ enabled: cols > 2, handles: ["se"] }}
                    onLayoutChange={handleLayoutChange}
                  >
                    {group.services.map((service) => (
                      <div key={service.editor_id} className="min-w-0">
                        <div className="flex h-full min-w-0 items-center gap-3 overflow-hidden rounded-md border border-border bg-card px-3 py-2">
                          <span
                            className="layout-drag-handle grid size-8 shrink-0 cursor-grab place-items-center rounded-md bg-muted text-muted-foreground active:cursor-grabbing"
                            aria-hidden="true"
                            title="Drag to move"
                          >
                            <IconGripVertical size={17} aria-hidden="true" />
                          </span>
                          {service.icon ? (
                            <img
                              className="size-8 shrink-0 rounded-md border border-border bg-muted object-contain p-1"
                              src={service.icon}
                              alt=""
                            />
                          ) : (
                            <span className="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-muted text-xs font-medium text-muted-foreground">
                              {service.title.slice(0, 1).toUpperCase() || "S"}
                            </span>
                          )}
                          <span className="min-w-0">
                            <span className="block truncate text-sm font-medium">
                              {service.title || "Untitled service"}
                            </span>
                            <span className="block truncate text-xs text-muted-foreground">
                              {service.widget_name || "Link card"}
                            </span>
                          </span>
                        </div>
                      </div>
                    ))}
                  </ReactGridLayout>
                )}
              </div>
            )}
          </div>

          <div className="grid gap-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h4 className="text-sm font-medium">Services and widgets</h4>
              <button className={buttonClass} type="button" onClick={onAddService}>
                <IconPlus size={16} aria-hidden="true" /> Add service or widget
              </button>
            </div>
            {group.services.map((service, serviceIndex) => (
              <ServiceEditor
                key={service.editor_id}
                service={service}
                index={serviceIndex}
                canMoveDown={serviceIndex < group.services.length - 1}
                widgets={widgets}
                nodes={nodes}
                uploading={uploading}
                onChange={(update) => updateService(serviceIndex, update)}
                onRemove={() => removeService(serviceIndex)}
                onMove={(direction) => onMoveService(serviceIndex, direction)}
                onResize={(dimension, change) =>
                  onResizeService(service.editor_id, dimension, change)
                }
                onUpload={onUpload}
              />
            ))}
          </div>
        </div>
      </details>
    </section>
  );
}

export { ServiceGroupEditor };
