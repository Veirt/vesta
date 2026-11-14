import { IconPlus } from "@tabler/icons-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ServiceGroupEditor } from "@/features/settings/components/service-group-editor";
import { buttonClass } from "@/features/settings/components/styles";
import type { EditableGroup, EditableNode } from "@/features/settings/models";
import type { WidgetDescriptor } from "@/features/settings/schema";

function ServiceGroupsSection({
  groups,
  nodes,
  widgets,
  uploading,
  onAddGroup,
  onChangeGroup,
  onRemoveGroup,
  onMoveGroup,
  onAddService,
  onMoveService,
  onResizeService,
  onLayoutChange,
  onUpload,
}: {
  groups: EditableGroup[];
  nodes: EditableNode[];
  widgets: WidgetDescriptor[];
  uploading: boolean;
  onAddGroup: () => void;
  onChangeGroup: (index: number, update: Partial<EditableGroup>) => void;
  onRemoveGroup: (index: number) => void;
  onMoveGroup: (index: number, direction: -1 | 1) => void;
  onAddService: (groupIndex: number) => void;
  onMoveService: (groupIndex: number, serviceIndex: number, direction: -1 | 1) => void;
  onResizeService: (
    groupIndex: number,
    serviceId: string,
    dimension: "width" | "height",
    change: -1 | 1,
  ) => void;
  onLayoutChange: (
    groupIndex: number,
    layout: Array<{ i: string; x: number; y: number; w: number; h: number }>,
  ) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="flex flex-row flex-wrap items-center justify-between gap-3 border-b border-border/70 px-5 py-4 sm:px-6">
        <div className="space-y-1">
          <CardTitle className="text-base font-semibold">Service groups</CardTitle>
          <p className="text-sm text-muted-foreground">
            Arrange cards visually, then open a service to edit its link or widget settings.
          </p>
        </div>
        <button className={buttonClass} type="button" onClick={onAddGroup}>
          <IconPlus size={16} aria-hidden="true" /> Add group
        </button>
      </CardHeader>
      <CardContent className="grid gap-4 px-5 py-5 sm:px-6">
        {groups.length === 0 && (
          <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
            No groups yet. Add a group, then add service links or built-in widgets.
          </p>
        )}
        {groups.map((group, index) => (
          <ServiceGroupEditor
            key={group.editor_id}
            group={group}
            groupIndex={index}
            groupsCount={groups.length}
            nodes={nodes}
            widgets={widgets}
            uploading={uploading}
            initiallyOpen={index === groups.length - 1}
            onChange={(update) => onChangeGroup(index, update)}
            onRemove={() => onRemoveGroup(index)}
            onMoveGroup={(direction) => onMoveGroup(index, direction)}
            onAddService={() => onAddService(index)}
            onMoveService={(serviceIndex, direction) =>
              onMoveService(index, serviceIndex, direction)
            }
            onResizeService={(serviceId, dimension, change) =>
              onResizeService(index, serviceId, dimension, change)
            }
            onLayoutChange={(layout) => onLayoutChange(index, layout)}
            onUpload={onUpload}
          />
        ))}
      </CardContent>
    </Card>
  );
}

export { ServiceGroupsSection };
