import { GroupEditorDialog } from "@/features/dashboard/group-editor-dialog";
import { TileEditorDialog } from "@/features/dashboard/tile-editor-dialog";
import { useDashboardEditor } from "@/features/dashboard/use-dashboard-editor";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/shared/lib/draft-models";

/** The two edit-mode dialogs, wired to the dashboard editor. */
export function DashboardEditDialogs({
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
