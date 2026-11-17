import { useEffect, useState } from "react";
import type {
  DashboardItem as DashboardItemData,
  DashboardGroup,
} from "@/features/dashboard/schema";
import {
  DashboardEditBanners,
  DashboardItemsHeader,
} from "@/features/dashboard/dashboard-edit-chrome";
import { DashboardEditDialogs } from "@/features/dashboard/dashboard-edit-dialogs";
import { DashboardTilesArea } from "@/features/dashboard/dashboard-tiles-area";
import { groupDashboardItems, type DashboardGroupSection } from "@/features/dashboard/tile-grid";
import { useDashboardEditor, type LayoutEntry } from "@/features/dashboard/use-dashboard-editor";
import { useWindowEdgeScroll } from "@/features/dashboard/use-window-edge-scroll";
import { validateDashboardGroups, validateDashboardItems } from "@/features/dashboard/validation";
import { fetchAuthStatus } from "@/features/settings/api";
import type { EditableDashboardItem } from "@/features/settings/models";

/**
 * Locate the currently edited tile/group within the draft sections, plus its
 * position among its group siblings for the move earlier/later controls.
 */
function locateSelection(
  editor: ReturnType<typeof useDashboardEditor>,
  selectedEditorId: string | null,
  selectedGroupId: string | null,
  draftSections: DashboardGroupSection<EditableDashboardItem>[],
) {
  const orderedDraft = draftSections.flatMap((section) => section.items);
  const selected = editor.items.find((item) => item.editor_id === selectedEditorId) ?? null;
  const selectedGroup = editor.groups.find((group) => group.id === selectedGroupId) ?? null;
  const selectedGroupItems = selected
    ? orderedDraft.filter((item) => (item.group ?? "") === (selected.group ?? ""))
    : [];
  const selectedIndex = selected
    ? selectedGroupItems.findIndex((item) => item.editor_id === selected.editor_id)
    : -1;
  return { selected, selectedGroup, selectedGroupItems, selectedIndex };
}

/** Load/save error details for the editor alert banner, null when healthy. */
function editorAlert(editor: ReturnType<typeof useDashboardEditor>) {
  const message = editor.error || editor.loadError;
  if (!message) return null;
  return {
    message,
    showRetry: Boolean(editor.loadError),
    showSignIn: editor.error.includes("sign in") || editor.loadError.includes("401"),
  };
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
  const { handleWindowEdgeScroll, stopWindowEdgeScroll } = useWindowEdgeScroll();

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
  const { selected, selectedGroup, selectedGroupItems, selectedIndex } = locateSelection(
    editor,
    selectedEditorId,
    selectedGroupId,
    draftSections,
  );
  const validationIssues = [
    ...validateDashboardItems(editor.items, editor.widgets, editor.nodes, editor.groups),
    ...validateDashboardGroups(editor.groups),
  ];
  const visibleSections = groupDashboardItems(normalizedQuery ? visibleItems : items, groups, true);

  function handleLayoutChange(layout: ReadonlyArray<LayoutEntry>, cols: number) {
    editor.updateLayout([...layout], cols);
  }

  async function saveDashboard() {
    const saved = await editor.save();
    if (saved) setEditing(false);
  }

  return (
    <section id="dashboard-items" aria-labelledby="dashboard-items-heading" className="space-y-4">
      <DashboardItemsHeader
        authenticated={authenticated}
        editing={editing}
        saving={editor.saving}
        dirty={editor.dirty}
        loading={editor.loading}
        hasIssues={validationIssues.length > 0}
        onEdit={() => setEditing(true)}
        onDiscard={() => {
          setEditing(false);
          setSelectedEditorId(null);
        }}
        onSave={() => void saveDashboard()}
      />

      <DashboardEditBanners
        editing={editing}
        editor={editor}
        validationIssues={validationIssues}
        alert={editorAlert(editor)}
        onSelectEditor={setSelectedEditorId}
        onSelectGroup={setSelectedGroupId}
      />

      <DashboardTilesArea
        loading={loading}
        editing={editing}
        editor={editor}
        items={items}
        visibleItems={visibleItems}
        visibleSections={visibleSections}
        draftSections={draftSections}
        normalizedQuery={normalizedQuery}
        draggingGroupId={draggingGroupId}
        draggingTile={draggingTile}
        onGroupDragStart={setDraggingGroupId}
        onGroupDragEnd={() => setDraggingGroupId(null)}
        onEditGroup={setSelectedGroupId}
        onSelectEditor={setSelectedEditorId}
        onLayoutChange={handleLayoutChange}
        onTileDragStart={() => setDraggingTile(true)}
        onTileDrag={handleWindowEdgeScroll}
        onTileDragStop={() => {
          stopWindowEdgeScroll();
          setDraggingTile(false);
        }}
      />

      <DashboardEditDialogs
        selected={selected}
        selectedGroup={selectedGroup}
        selectedIndex={selectedIndex}
        selectedGroupItemCount={selectedGroupItems.length}
        editor={editor}
        onCloseEditor={() => setSelectedEditorId(null)}
        onCloseGroupEditor={() => setSelectedGroupId(null)}
      />
    </section>
  );
}

export { DashboardItems };
