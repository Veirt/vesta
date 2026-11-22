import { useEffect, useState } from "react";
import { fetchSettings, saveSettings } from "@/shared/api/client";
import {
  createEditorId,
  createSettingsPayload,
  slug,
  uniqueId,
  type EditableNode,
} from "@/shared/lib/draft-models";
import { useTheme } from "@/shared/lib/theme";
import { useEditorFeedback, type LoadState } from "@/shared/lib/use-editor-feedback";
import type { AppSettings, SettingsPayload, WidgetDescriptor } from "@/shared/schemas/core";

type SavedDraft = {
  app: AppSettings;
  nodes: EditableNode[];
};

export function useSettingsEditor() {
  const [app, setApp] = useState<AppSettings>({
    dashboard_title: "Server Monitor",
    theme: "system",
  });
  const [nodes, setNodes] = useState<EditableNode[]>([]);
  const [widgets, setWidgets] = useState<WidgetDescriptor[]>([]);
  const [loadState, setLoadState] = useState<LoadState<{ draft: SavedDraft; fingerprint: string }>>(
    { kind: "loading" },
  );
  const feedback = useEditorFeedback();
  const { loadRevision, setAction, setBanner, clearNotice, retryLoad } = feedback;
  useTheme(app.theme);

  useEffect(() => {
    let active = true;
    setLoadState({ kind: "loading" });
    fetchSettings()
      .then((data) => {
        if (!active) return;
        const draft: SavedDraft = {
          app: data.app,
          nodes: data.dashboard.nodes.map((node, index) => ({
            ...node,
            hostname: node.hostname ?? "",
            device_name: node.device_name ?? "",
            icon: node.icon ?? "",
            editor_id: createEditorId(),
            id_touched:
              node.id !==
              uniqueId(
                slug(node.name, "server"),
                data.dashboard.nodes.slice(0, index).map((previous) => previous.id),
              ),
          })),
        };
        setApp(draft.app);
        setNodes(draft.nodes);
        setWidgets(data.widgets);
        setLoadState({
          kind: "ready",
          draft,
          fingerprint: JSON.stringify(createSettingsPayload(draft.nodes, draft.app)),
        });
      })
      .catch((cause: unknown) => {
        if (active) {
          setLoadState({
            kind: "error",
            message: cause instanceof Error ? cause.message : "Settings could not be loaded.",
          });
        }
      });
    return () => {
      active = false;
    };
  }, [loadRevision]);

  const currentPayload = createSettingsPayload(nodes, app);
  const dirty =
    loadState.kind === "ready" && JSON.stringify(currentPayload) !== loadState.fingerprint;

  function updateApp(update: Partial<AppSettings>) {
    setApp((current) => ({ ...current, ...update }));
    clearNotice();
  }

  function addNode() {
    const id = uniqueId(
      slug("New server", "server"),
      nodes.map((node) => node.id),
    );
    setNodes((current) => [
      ...current,
      {
        id,
        name: "New server",
        hostname: "",
        device_name: "",
        icon: "",
        endpoint: "local",
        interval_secs: 15,
        editor_id: createEditorId(),
        id_touched: false,
      },
    ]);
    clearNotice();
  }

  function updateNode(index: number, update: Partial<EditableNode>) {
    setNodes((current) =>
      current.map((node, nodeIndex) => {
        if (nodeIndex !== index) return node;
        const idTouched = node.id_touched || ("id" in update && update.id !== node.id);
        const updated = { ...node, ...update, id_touched: idTouched };
        if (update.name !== undefined && !idTouched) {
          updated.id = uniqueId(
            slug(update.name, "server"),
            current.filter((_, otherIndex) => otherIndex !== index).map((other) => other.id),
          );
        }
        return updated;
      }),
    );
    clearNotice();
  }

  function removeNode(index: number) {
    setNodes((current) => current.filter((_, nodeIndex) => nodeIndex !== index));
    clearNotice();
  }

  function moveNode(index: number, direction: -1 | 1) {
    setNodes((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
    clearNotice();
  }

  async function uploadImage(file: File): Promise<string | null> {
    return feedback.uploadImage(file, "Save changes to use it.");
  }

  async function save() {
    setAction({ kind: "saving" });
    setBanner({ kind: "none" });
    const payload: SettingsPayload = createSettingsPayload(nodes, app);
    try {
      await saveSettings(payload);
      const saved: SavedDraft = { app, nodes };
      setLoadState({
        kind: "ready",
        draft: saved,
        fingerprint: JSON.stringify(payload),
      });
      setBanner({ kind: "notice", message: "Changes saved to config.toml." });
    } catch (cause) {
      setBanner({
        kind: "error",
        message: cause instanceof Error ? cause.message : "Settings could not be saved.",
      });
    } finally {
      setAction({ kind: "idle" });
    }
  }

  function discardChanges() {
    if (loadState.kind !== "ready") return;
    setApp(loadState.draft.app);
    setNodes(loadState.draft.nodes);
    setBanner({ kind: "notice", message: "Unsaved changes discarded." });
  }

  async function refreshWidgets() {
    try {
      const data = await fetchSettings();
      setWidgets(data.widgets);
    } catch {}
  }

  return {
    app,
    dirty,
    error: feedback.error,
    loadError: loadState.kind === "error" ? loadState.message : "",
    loading: loadState.kind === "loading",
    notice: feedback.notice,
    nodes,
    saving: feedback.saving,
    uploading: feedback.uploading,
    widgets,
    addNode,
    discardChanges,
    moveNode,
    refreshWidgets,
    removeNode,
    retryLoad,
    save,
    updateApp,
    updateNode,
    uploadImage,
  };
}
