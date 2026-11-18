import type { DashboardItem as DashboardItemData } from "@/shared/schemas/core";
import { DashboardDrafts } from "@/features/dashboard/dashboard-drafts";
import { DashboardViewGroups } from "@/features/dashboard/dashboard-view-groups";
import type { DashboardGroupSection } from "@/shared/lib/layout";
import { useDashboardEditor, type LayoutEntry } from "@/features/dashboard/use-dashboard-editor";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";

/**
 * The body of the Apps-and-widgets section: loading placeholder, edit-mode
 * drafts, or the read-only tile grid, depending on dashboard state.
 */
function DashboardTilesArea({
  loading,
  editing,
  editor,
  items,
  visibleItems,
  visibleSections,
  draftSections,
  normalizedQuery,
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
  loading: boolean;
  editing: boolean;
  editor: ReturnType<typeof useDashboardEditor>;
  items: DashboardItemData[];
  visibleItems: DashboardItemData[];
  visibleSections: DashboardGroupSection<DashboardItemData>[];
  draftSections: DashboardGroupSection<EditableDashboardItem>[];
  normalizedQuery: string;
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
  if (loading) {
    return (
      <p className="text-sm text-muted-foreground" role="status">
        Loading dashboard tiles…
      </p>
    );
  }

  if (editing && editor.ready) {
    if (editor.loadError) return null;
    if (editor.items.length === 0 && editor.groups.length === 0) {
      return (
        <div className="dashboard-empty-state">
          <p>No tiles yet. Add a service link or a widget to start your dashboard.</p>
        </div>
      );
    }
    return (
      <DashboardDrafts
        editor={editor}
        draftSections={draftSections}
        draggingGroupId={draggingGroupId}
        draggingTile={draggingTile}
        onGroupDragStart={onGroupDragStart}
        onGroupDragEnd={onGroupDragEnd}
        onEditGroup={onEditGroup}
        onSelectEditor={onSelectEditor}
        onLayoutChange={onLayoutChange}
        onTileDragStart={onTileDragStart}
        onTileDrag={onTileDrag}
        onTileDragStop={onTileDragStop}
      />
    );
  }

  const compactViewItems = normalizedQuery ? visibleItems : items;
  if (compactViewItems.length === 0) {
    return (
      <div className="dashboard-empty-state">
        <p>
          {normalizedQuery
            ? "No apps or widgets match your search."
            : "No apps or widgets yet. Sign in and choose Edit dashboard to add your first tile."}
        </p>
      </div>
    );
  }
  return <DashboardViewGroups sections={visibleSections} query={normalizedQuery} />;
}

export { DashboardTilesArea };
