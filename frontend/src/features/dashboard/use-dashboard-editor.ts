import { useEffect, useState } from "react";
import { fetchSettings, saveDashboardItems } from "@/shared/api/client";
import {
  serializeDashboardGroups,
  serializeDashboardItems,
  toEditableGroup,
  toEditableItem,
  type EditableDashboardGroup,
  type EditableDashboardItem,
} from "@/shared/lib/draft-models";
import { DEFAULT_GROUP_SPAN } from "@/shared/lib/layout";
import { useEditorFeedback, type LoadState } from "@/shared/lib/use-editor-feedback";
import type { NodeSettings, WidgetDescriptor } from "@/shared/schemas/core";
import {
  applyLayoutDraft,
  clampGroupSpanUpdate,
  compactY,
  createGroupEntry,
  createServiceDraft,
  createWidgetDraft,
  ensureKnownGroups,
  moveGroupDraft,
  moveGroupToDraft,
  moveItemDraft,
  moveItemToGroupDraft,
  removeGroupFromItems,
  resizeItemDraft,
  type LayoutEntry,
} from "@/features/dashboard/dashboard-draft-ops";

export function useDashboardEditor(enabled: boolean) {
  const [items, setItems] = useState<EditableDashboardItem[]>([]);
  const [groups, setGroups] = useState<EditableDashboardGroup[]>([]);
  const [nodes, setNodes] = useState<NodeSettings[]>([]);
  const [widgets, setWidgets] = useState<WidgetDescriptor[]>([]);
  const [loadState, setLoadState] = useState<LoadState>({ kind: "idle" });
  const feedback = useEditorFeedback();
  const { loadRevision, setAction, setBanner, clearNotice, retryLoad } = feedback;

  useEffect(() => {
    if (!enabled) {
      setLoadState({ kind: "idle" });
      return;
    }
    let active = true;
    setLoadState({ kind: "loading" });
    fetchSettings()
      .then((data) => {
        if (!active) return;
        const editableItems = data.dashboard.items.map(toEditableItem);
        const editableGroups = ensureKnownGroups(
          editableItems,
          data.dashboard.groups.map(toEditableGroup),
        );
        const compactedItems = compactY(editableItems);
        setItems(compactedItems);
        setGroups(editableGroups);
        setNodes(data.dashboard.nodes);
        setWidgets(data.widgets);
        setLoadState({
          kind: "ready",
          baseline: JSON.stringify({
            items: serializeDashboardItems(compactedItems, data.widgets),
            groups: serializeDashboardGroups(editableGroups),
          }),
        });
      })
      .catch((cause: unknown) => {
        if (active) {
          setLoadState({
            kind: "error",
            message:
              cause instanceof Error ? cause.message : "Dashboard settings could not be loaded.",
          });
        }
      });
    return () => {
      active = false;
    };
  }, [enabled, loadRevision]);

  const serializedItems = serializeDashboardItems(items, widgets);
  const serializedGroups = serializeDashboardGroups(groups);
  const serializedDraft = JSON.stringify({ items: serializedItems, groups: serializedGroups });
  const baseline = loadState.kind === "ready" ? loadState.baseline : "";
  const dirty = loadState.kind === "ready" && serializedDraft !== baseline;

  function addGroup(requestedTitle?: string) {
    const { id, nextGroups } = createGroupEntry(groups, requestedTitle);
    setGroups(nextGroups);
    clearNotice();
    return id;
  }

  function updateGroup(id: string, update: Partial<EditableDashboardGroup>) {
    setGroups((current) =>
      current.map((group) => (group.id === id ? { ...group, ...update } : group)),
    );
    clearNotice();
  }

  function removeGroup(id: string) {
    setGroups((current) => current.filter((group) => group.id !== id));
    setItems((current) => removeGroupFromItems(current, id));
    clearNotice();
  }

  function addService(group?: string | null) {
    const service = createServiceDraft(items, group);
    setItems((current) => [...current, service]);
    clearNotice();
    return service.editor_id;
  }

  function addWidget(name: string, group?: string | null) {
    const widget = createWidgetDraft(items, widgets, name, group);
    if (!widget) return null;
    setItems((current) => [...current, widget]);
    clearNotice();
    return widget.editor_id;
  }

  function updateItem(editorId: string, update: object) {
    setItems((current) =>
      current.map((item) =>
        item.editor_id === editorId ? ({ ...item, ...update } as EditableDashboardItem) : item,
      ),
    );
    clearNotice();
  }

  function removeItem(editorId: string) {
    setItems((current) => current.filter((item) => item.editor_id !== editorId));
    clearNotice();
  }

  function moveItem(editorId: string, direction: -1 | 1) {
    setItems((current) => moveItemDraft(current, editorId, direction));
    clearNotice();
  }

  function resizeItem(editorId: string, width: number, height: number) {
    setItems((current) => resizeItemDraft(current, groups, widgets, editorId, width, height));
    clearNotice();
  }

  function moveGroup(id: string, direction: -1 | 1) {
    setGroups((current) => moveGroupDraft(current, id, direction));
    clearNotice();
  }

  function moveGroupTo(id: string, targetId: string, placement: "before" | "after") {
    setGroups((current) => moveGroupToDraft(current, id, targetId, placement));
    clearNotice();
  }

  function ensureUngroupedGroup() {
    setGroups((current) =>
      current.some((group) => group.id === "")
        ? current
        : [{ id: "", title: "", description: "", icon: "", width: DEFAULT_GROUP_SPAN }, ...current],
    );
  }

  function setGroupWidth(id: string, width: number) {
    const clamped = clampGroupSpanUpdate(width);
    setGroups((current) =>
      current.map((group) => (group.id === id ? { ...group, width: clamped } : group)),
    );
    clearNotice();
  }

  function moveItemToGroup(editorId: string, group: string | null) {
    if (group) {
      setGroups((current) =>
        current.some((entry) => entry.id === group)
          ? current
          : [
              ...current,
              { id: group, title: group, description: "", icon: "", width: DEFAULT_GROUP_SPAN },
            ],
      );
    }
    setItems((current) => moveItemToGroupDraft(current, editorId, group));
    clearNotice();
  }

  function updateLayout(layout: LayoutEntry[], cols: number) {
    setItems((current) => applyLayoutDraft(current, groups, widgets, layout, cols));
    clearNotice();
  }

  async function uploadImage(file: File): Promise<string | null> {
    return feedback.uploadImage(file, "Save the dashboard to use it.");
  }

  async function save() {
    setAction({ kind: "saving" });
    setBanner({ kind: "none" });
    try {
      await saveDashboardItems(serializedItems, serializedGroups);
      setLoadState({ kind: "ready", baseline: serializedDraft });
      setBanner({ kind: "notice", message: "Dashboard saved." });
      return true;
    } catch (cause) {
      setBanner({
        kind: "error",
        message: cause instanceof Error ? cause.message : "Dashboard could not be saved.",
      });
      return false;
    } finally {
      setAction({ kind: "idle" });
    }
  }

  return {
    dirty,
    error: feedback.error,
    items,
    groups,
    loadError: loadState.kind === "error" ? loadState.message : "",
    loading: loadState.kind === "loading",
    ready: loadState.kind === "ready",
    nodes,
    notice: feedback.notice,
    saving: feedback.saving,
    uploading: feedback.uploading,
    widgets,
    addService,
    addGroup,
    addWidget,
    ensureUngroupedGroup,
    moveGroup,
    moveGroupTo,
    moveItem,
    moveItemToGroup,
    removeItem,
    removeGroup,
    resizeItem,
    retryLoad,
    save,
    setGroupWidth,
    updateItem,
    updateGroup,
    updateLayout,
    uploadImage,
  };
}
