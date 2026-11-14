import { useEffect, useState } from "react";
import { fetchSettings, saveSettings, uploadIcon } from "@/features/settings/api";
import {
  createEditorId,
  createSettingsPayload,
  slug,
  toEditableService,
} from "@/features/settings/models";
import type { EditableGroup, EditableNode } from "@/features/settings/models";
import type { AppSettings, SettingsPayload, WidgetDescriptor } from "@/features/settings/schema";
import { useTheme } from "@/features/settings/theme";

type SavedDraft = {
  app: AppSettings;
  nodes: EditableNode[];
  groups: EditableGroup[];
};

type LayoutChange = Array<{ i: string; x: number; y: number; w: number; h: number }>;

function uniqueId(base: string, existing: string[]) {
  const used = new Set(existing);
  let candidate = base;
  let suffix = 2;
  while (used.has(candidate)) candidate = `${base}-${suffix++}`;
  return candidate;
}

function defaultWidgetConfig(descriptor: WidgetDescriptor | undefined) {
  return Object.fromEntries(
    (descriptor?.fields ?? [])
      .filter((field) => field.default !== undefined)
      .map((field) => [field.key, field.default]),
  );
}

function useSettingsEditor() {
  const [app, setApp] = useState<AppSettings>({
    dashboard_title: "Server Monitor",
    theme: "system",
  });
  const [nodes, setNodes] = useState<EditableNode[]>([]);
  const [groups, setGroups] = useState<EditableGroup[]>([]);
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
          nodes: data.dashboard.nodes.map((node) => ({
            ...node,
            hostname: node.hostname ?? "",
            device_name: node.device_name ?? "",
            icon: node.icon ?? "",
            editor_id: createEditorId(),
          })),
          groups: data.dashboard.groups.map((group) => ({
            ...group,
            editor_id: createEditorId(),
            services: group.services.map((service) => toEditableService(service)),
          })),
        };
        setApp(draft.app);
        setNodes(draft.nodes);
        setGroups(draft.groups);
        setWidgets(data.widgets);
        setSavedDraft(draft);
        setSavedFingerprint(
          JSON.stringify(createSettingsPayload(draft.nodes, draft.groups, draft.app, data.widgets)),
        );
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

  const currentPayload = createSettingsPayload(nodes, groups, app, widgets);
  const dirty =
    !loading && savedDraft !== null && JSON.stringify(currentPayload) !== savedFingerprint;

  function updateApp(update: Partial<AppSettings>) {
    setApp((current) => ({ ...current, ...update }));
    setNotice("");
  }

  function addNode() {
    const id = uniqueId(
      `server-${nodes.length + 1}`,
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
      },
    ]);
    setNotice("");
  }

  function updateNode(index: number, update: Partial<EditableNode>) {
    setNodes((current) => current.map((node, i) => (i === index ? { ...node, ...update } : node)));
    setNotice("");
  }

  function removeNode(index: number) {
    setNodes((current) => current.filter((_, i) => i !== index));
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

  function addGroup() {
    const id = uniqueId(
      slug(`group-${groups.length + 1}`, "group"),
      groups.map((group) => group.id),
    );
    setGroups((current) => [
      ...current,
      { id, editor_id: createEditorId(), name: "New group", services: [] },
    ]);
    setNotice("");
  }

  function updateGroup(index: number, update: Partial<EditableGroup>) {
    setGroups((current) =>
      current.map((group, groupIndex) => {
        if (groupIndex !== index) return group;
        const updated = { ...group, ...update };
        if (!update.services) return updated;
        const previousById = new Map(group.services.map((service) => [service.editor_id, service]));
        return {
          ...updated,
          services: update.services.map((service) => {
            const previous = previousById.get(service.editor_id);
            if (!previous || previous.widget_name === service.widget_name) return service;
            const descriptor = widgets.find(({ name }) => name === service.widget_name);
            return { ...service, widget_config: defaultWidgetConfig(descriptor) };
          }),
        };
      }),
    );
    setNotice("");
  }

  function removeGroup(index: number) {
    setGroups((current) => current.filter((_, i) => i !== index));
    setNotice("");
  }

  function moveGroup(index: number, direction: -1 | 1) {
    setGroups((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
    setNotice("");
  }

  function addService(groupIndex: number) {
    const editorId = createEditorId();
    setGroups((current) =>
      current.map((group, index) =>
        index === groupIndex
          ? {
              ...group,
              services: [
                ...group.services,
                {
                  editor_id: editorId,
                  title: "New service",
                  href: "",
                  icon: "",
                  x: (group.services.length % 3) * 4,
                  y: Math.floor(group.services.length / 3) * 2,
                  width: 4,
                  height: 1,
                  ping_url: "",
                  widget_name: "",
                  widget_config: {},
                },
              ],
            }
          : group,
      ),
    );
    setNotice("");
  }

  function moveService(groupIndex: number, serviceIndex: number, direction: -1 | 1) {
    setGroups((current) =>
      current.map((group, index) => {
        const targetIndex = serviceIndex + direction;
        if (index !== groupIndex || targetIndex < 0 || targetIndex >= group.services.length)
          return group;
        const next = [...group.services];
        const currentService = next[serviceIndex];
        const targetService = next[targetIndex];
        next[serviceIndex] = {
          ...currentService,
          x: targetService.x,
          y: targetService.y,
          width: targetService.width,
          height: targetService.height,
        };
        next[targetIndex] = {
          ...targetService,
          x: currentService.x,
          y: currentService.y,
          width: currentService.width,
          height: currentService.height,
        };
        return { ...group, services: next };
      }),
    );
    setNotice("");
  }

  function resizeService(
    groupIndex: number,
    serviceId: string,
    dimension: "width" | "height",
    change: -1 | 1,
  ) {
    setGroups((current) =>
      current.map((group, index) =>
        index === groupIndex
          ? {
              ...group,
              services: group.services.map((service) => {
                if (service.editor_id !== serviceId) return service;
                if (dimension === "width") {
                  const width = Math.max(1, Math.min(12, service.width + change));
                  return { ...service, width, x: Math.min(service.x, 12 - width) };
                }
                return { ...service, height: Math.max(1, Math.min(12, service.height + change)) };
              }),
            }
          : group,
      ),
    );
    setNotice("");
  }

  function updateServiceLayout(groupIndex: number, layout: LayoutChange) {
    const byId = new Map(layout.map((item) => [item.i, item]));
    setGroups((current) =>
      current.map((group, index) => {
        if (index !== groupIndex) return group;
        let changed = false;
        const services = group.services.map((service) => {
          const item = byId.get(service.editor_id);
          if (
            !item ||
            (service.x === item.x &&
              service.y === item.y &&
              service.width === item.w &&
              service.height === item.h)
          ) {
            return service;
          }
          changed = true;
          return { ...service, x: item.x, y: item.y, width: item.w, height: item.h };
        });
        return changed ? { ...group, services } : group;
      }),
    );
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
    const payload: SettingsPayload = createSettingsPayload(nodes, groups, app, widgets);
    setSaving(true);
    try {
      await saveSettings(payload);
      const saved: SavedDraft = { app, nodes, groups };
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
    setGroups(savedDraft.groups);
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
    groups,
    loadError,
    loading,
    notice,
    nodes,
    saving,
    uploading,
    widgets,
    addGroup,
    addNode,
    addService,
    discardChanges,
    moveGroup,
    moveNode,
    moveService,
    removeGroup,
    removeNode,
    retryLoad,
    save,
    updateApp,
    updateGroup,
    updateNode,
    updateServiceLayout,
    resizeService,
    uploadImage,
  };
}

export { useSettingsEditor };
