import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import { minSizeFor, type DashboardGroupSection } from "@/shared/lib/layout";
import type { LayoutEntry } from "@/features/dashboard/dashboard-draft-ops";
import { useDashboardEditor } from "@/features/dashboard/use-dashboard-editor";
import { DashboardEditGroup } from "@/features/dashboard/dashboard-edit-group";

function DashboardDrafts({
  editor,
  draftSections,
  draggingGroupId,
  draggingTile,
  onGroupDragStart,
  onGroupDragEnd,
  onEditGroup,
  onSelectEditor,
  onLayoutChange,
  onTileDragStart,
  onTileDrag,
  onTileDragStop,
}: {
  editor: ReturnType<typeof useDashboardEditor>;
  draftSections: DashboardGroupSection<EditableDashboardItem>[];
  draggingGroupId: string | null;
  draggingTile: boolean;
  onGroupDragStart: (sectionId: string) => void;
  onGroupDragEnd: () => void;
  onEditGroup: (sectionId: string) => void;
  onSelectEditor: (editorId: string | null) => void;
  onLayoutChange: (layout: ReadonlyArray<LayoutEntry>, cols: number) => void;
  onTileDragStart: () => void;
  onTileDrag: (event: Event) => void;
  onTileDragStop: () => void;
}) {
  return (
    <div className="dashboard-grid-wrap">
      <div className="dashboard-edit-groups">
        {draftSections.map((section) => (
          <DashboardEditGroup
            key={section.id}
            section={section}
            groupDragActive={draggingGroupId !== null}
            isGroupDragSource={draggingGroupId === section.id}
            minSizeFor={(item) => minSizeFor(item, editor.widgets, section.width)}
            widgets={editor.widgets}
            onGroupDragStart={() => {
              if (section.id === "") editor.ensureUngroupedGroup();
              onGroupDragStart(section.id);
            }}
            onGroupDragEnd={onGroupDragEnd}
            onGroupDragEnter={(placement) => {
              if (draggingGroupId === null) return;
              if (section.id === "") editor.ensureUngroupedGroup();
              editor.moveGroupTo(draggingGroupId, section.id, placement);
            }}
            onGroupResize={(width) => {
              if (section.id === "") editor.ensureUngroupedGroup();
              editor.setGroupWidth(section.id, width);
            }}
            onTileResize={(editorId, width, height) => editor.resizeItem(editorId, width, height)}
            onEditGroup={() => onEditGroup(section.id)}
            onAddService={() => {
              const editorId = editor.addService(section.id || null);
              if (editorId) onSelectEditor(editorId);
            }}
            onAddWidget={(name) => {
              const editorId = editor.addWidget(name, section.id || null);
              if (editorId) onSelectEditor(editorId);
            }}
            onEdit={onSelectEditor}
            onLayoutChange={onLayoutChange}
            onDragStart={onTileDragStart}
            onDrag={onTileDrag}
            onDragStop={onTileDragStop}
          />
        ))}
      </div>
      {draggingTile && <div className="dashboard-edit-runway" aria-hidden="true" />}
    </div>
  );
}

export { DashboardDrafts };
