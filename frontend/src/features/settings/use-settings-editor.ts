import { useEffect, useState } from "react";
import { fetchSettings, saveSettings, uploadIcon } from "@/features/settings/api";
import { createEditorId, createSettingsPayload, slug } from "@/features/settings/models";
import type { EditableNode } from "@/features/settings/models";
import type { AppSettings, SettingsPayload, WidgetDescriptor } from "@/features/settings/schema";
import { useTheme } from "@/features/settings/theme";

type SavedDraft = {
  app: AppSettings;
  nodes: EditableNode[];
};

function uniqueId(base: string, existing: string[]) {
  const used = new Set(existing);
  let candidate = base;
  let suffix = 2;
  while (used.has(candidate)) candidate = `${base}-${suffix++}`;
  return candidate;
}

function useSettingsEditor() {
  const [app, setApp] = useState<AppSettings>({
    dashboard_title: "Server Monitor",
    theme: "system",
  });
  const [nodes, setNodes] = useState<EditableNode[]>([]);
  const [widgets, setWidgets] = useState<WidgetDescriptor[]>([]);
  const [savedDraft, setSavedDraft] = useState<SavedDraft | null>(null);
  const [savedFingerprint, setSavedFingerprint] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [loadRevision, setLoadRevision] = useState(0);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useTheme(app.theme);

  useEffect(() => {
    let active = true;
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
        setSavedDraft(draft);
        setSavedFingerprint(JSON.stringify(createSettingsPayload(draft.nodes, draft.app)));
      })
      .catch((cause: unknown) => {
        if (active)
          setLoadError(cause instanceof Error ? cause.message : "Settings could not be loaded.");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [loadRevision]);

  const currentPayload = createSettingsPayload(nodes, app);
  const dirty =
    !loading && savedDraft !== null && JSON.stringify(currentPayload) !== savedFingerprint;

  function updateApp(update: Partial<AppSettings>) {
    setApp((current) => ({ ...current, ...update }));
    setNotice("");
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
    setNotice("");
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
    setNotice("");
  }

  function removeNode(index: number) {
    setNodes((current) => current.filter((_, nodeIndex) => nodeIndex !== index));
    setNotice("");
  }

  function moveNode(index: number, direction: -1 | 1) {
    setNodes((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
    setNotice("");
  }

  async function uploadImage(file: File): Promise<string | null> {
    setUploading(true);
    setError("");
    try {
      const uploaded = await uploadIcon(file);
      setNotice(`${uploaded.name} uploaded. Save changes to use it.`);
      return uploaded.url;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The image could not be uploaded.");
      return null;
    } finally {
      setUploading(false);
    }
  }

  async function save() {
    setError("");
    setNotice("");
    const payload: SettingsPayload = createSettingsPayload(nodes, app);
    setSaving(true);
    try {
      await saveSettings(payload);
      const saved: SavedDraft = { app, nodes };
      setSavedDraft(saved);
      setSavedFingerprint(JSON.stringify(payload));
      setNotice("Changes saved to config.toml.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Settings could not be saved.");
    } finally {
      setSaving(false);
    }
  }

  function discardChanges() {
    if (!savedDraft) return;
    setApp(savedDraft.app);
    setNodes(savedDraft.nodes);
    setError("");
    setNotice("Unsaved changes discarded.");
  }

  function retryLoad() {
    setLoading(true);
    setLoadError("");
    setLoadRevision((revision) => revision + 1);
  }

  return {
    app,
    dirty,
    error,
    loadError,
    loading,
    notice,
    nodes,
    saving,
    uploading,
    widgets,
    addNode,
    discardChanges,
    moveNode,
    removeNode,
    retryLoad,
    save,
    updateApp,
    updateNode,
    uploadImage,
  };
}

export { useSettingsEditor };
