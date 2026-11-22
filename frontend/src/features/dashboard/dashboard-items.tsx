import { useEffect, useState } from "react";
import {
  DashboardEditBanners,
  DashboardItemsHeader,
} from "@/features/dashboard/dashboard-edit-chrome";
import { DashboardEditDialogs } from "@/features/dashboard/dashboard-edit-dialogs";
import { DashboardTilesArea } from "@/features/dashboard/dashboard-tiles-area";
import type { LayoutEntry } from "@/features/dashboard/dashboard-draft-ops";
import { useDashboardEditor } from "@/features/dashboard/use-dashboard-editor";
import { useWindowEdgeScroll } from "@/features/dashboard/use-window-edge-scroll";
import { fetchAuthStatus } from "@/shared/api/client";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import { groupDashboardItems, type DashboardGroupSection } from "@/shared/lib/layout";
import { validateDashboardGroups, validateDashboardItems } from "@/shared/lib/validation";
import type { DashboardGroup, DashboardItem as DashboardItemData } from "@/shared/schemas/core";

type EditorMode =
  | { kind: "viewing" }
  | {
      kind: "editing";
      modal: null | { kind: "tile"; editorId: string } | { kind: "group"; groupId: string };
    };

function locateSelection(
  editor: ReturnType<typeof useDashboardEditor>,
  mode: EditorMode,
  draftSections: DashboardGroupSection<EditableDashboardItem>[],
) {
  const modal = mode.kind === "editing" ? mode.modal : null;
  const selectedEditorId = modal?.kind === "tile" ? modal.editorId : null;
  const selectedGroupId = modal?.kind === "group" ? modal.groupId : null;
  const orderedDraft = draftSections.flatMap((section) => section.items);
  const selected = editor.items.find((item) => item.editor_id === selectedEditorId) ?? null;
  const selectedGroup = selectedGroupId
    ? (editor.groups.find((group) => group.id === selectedGroupId) ?? null)
    : null;
  const activeGroupKey = selected
    ? (selected.group ?? "")
    : selectedGroup
      ? selectedGroup.id
      : null;
  const selectedGroupItems =
    activeGroupKey !== null
      ? orderedDraft.filter((item) => (item.group ?? "") === activeGroupKey)
      : [];
  const selectedIndex = selected
    ? selectedGroupItems.findIndex((item) => item.editor_id === selected.editor_id)
    : -1;
  return { selected, selectedGroup, selectedGroupItems, selectedIndex };
}

function editorAlert(editor: ReturnType<typeof useDashboardEditor>) {
  const message = editor.error || editor.loadError;
  if (!message) return null;
  return {
    message,
    showRetry: Boolean(editor.loadError),
    showSignIn: editor.error.includes("sign in") || editor.loadError.includes("401"),
  };
}

export function DashboardItems({
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
  const [mode, setMode] = useState<EditorMode>({ kind: "viewing" });
  const [draggingTile, setDraggingTile] = useState(false);
  const [draggingGroupId, setDraggingGroupId] = useState<string | null>(null);
  const editing = mode.kind === "editing";
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
    mode,
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
    if (saved) setMode({ kind: "viewing" });
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
        onEdit={() => setMode({ kind: "editing", modal: null })}
        onDiscard={() => setMode({ kind: "viewing" })}
        onSave={() => void saveDashboard()}
      />

      <DashboardEditBanners
        editing={editing}
        editor={editor}
        validationIssues={validationIssues}
        alert={editorAlert(editor)}
        onSelectEditor={(editorId) =>
          setMode({ kind: "editing", modal: editorId ? { kind: "tile", editorId } : null })
        }
        onSelectGroup={(groupId) => setMode({ kind: "editing", modal: { kind: "group", groupId } })}
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
        onEditGroup={(groupId) => setMode({ kind: "editing", modal: { kind: "group", groupId } })}
        onSelectEditor={(editorId) =>
          setMode({ kind: "editing", modal: editorId ? { kind: "tile", editorId } : null })
        }
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
        onCloseEditor={() => setMode({ kind: "editing", modal: null })}
        onCloseGroupEditor={() => setMode({ kind: "editing", modal: null })}
      />
    </section>
  );
}
