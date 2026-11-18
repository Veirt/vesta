import { useEffect, useState } from "react";
import { fetchSettings, saveSettings, uploadIcon } from "@/shared/api/client";
import {
  createEditorId,
  createSettingsPayload,
  slug,
  type EditableNode,
} from "@/shared/lib/draft-models";
import { useTheme } from "@/shared/lib/theme";
import type { AppSettings, SettingsPayload, WidgetDescriptor } from "@/shared/schemas/core";

type SavedDraft = {
  app: AppSettings;
  nodes: EditableNode[];
};

type LoadState =
  | { kind: "loading" }
  | { kind: "ready"; draft: SavedDraft; fingerprint: string }
  | { kind: "error"; message: string };

type ActionState = { kind: "idle" } | { kind: "saving" } | { kind: "uploading" };
type FeedbackBanner =
  | { kind: "none" }
  | { kind: "error"; message: string }
  | { kind: "notice"; message: string };

function uniqueId(base: string, existing: string[]) {
  const used = new Set(existing);
  let candidate = base;
  let suffix = 2;
  while (used.has(candidate)) candidate = `${base}-${suffix++}`;
  return candidate;
}

export function useSettingsEditor() {
  const [app, setApp] = useState<AppSettings>({
    dashboard_title: "Server Monitor",
    theme: "system",
  });
  const [nodes, setNodes] = useState<EditableNode[]>([]);
  const [widgets, setWidgets] = useState<WidgetDescriptor[]>([]);
  const [loadState, setLoadState] = useState<LoadState>({ kind: "loading" });
  const [loadRevision, setLoadRevision] = useState(0);
  const [action, setAction] = useState<ActionState>({ kind: "idle" });
  const [banner, setBanner] = useState<FeedbackBanner>({ kind: "none" });
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

  function clearNotice() {
    setBanner((current) => (current.kind === "notice" ? { kind: "none" } : current));
  }

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
    setAction({ kind: "uploading" });
    setBanner({ kind: "none" });
    try {
      const uploaded = await uploadIcon(file);
      setBanner({
        kind: "notice",
        message: `${uploaded.name} uploaded. Save changes to use it.`,
      });
      return uploaded.url;
    } catch (cause) {
      setBanner({
        kind: "error",
        message: cause instanceof Error ? cause.message : "The image could not be uploaded.",
      });
      return null;
    } finally {
      setAction({ kind: "idle" });
    }
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

  function retryLoad() {
    setLoadRevision((revision) => revision + 1);
  }

  return {
    app,
    dirty,
    error: banner.kind === "error" ? banner.message : "",
    loadError: loadState.kind === "error" ? loadState.message : "",
    loading: loadState.kind === "loading",
    notice: banner.kind === "notice" ? banner.message : "",
    nodes,
    saving: action.kind === "saving",
    uploading: action.kind === "uploading",
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
