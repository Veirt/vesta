import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { installPlugin, listPlugins, uninstallPlugin, updatePlugin } from "@/shared/api/client";
import type { Plugin } from "@/shared/schemas/core";
import { buttonClass, destructiveButtonClass, inputClass } from "@/shared/components/styles";

type CatalogState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; plugins: Plugin[] };

type MutationState =
  | { kind: "idle" }
  | { kind: "pending"; verb: "installing" | "updating" | "removing" }
  | { kind: "error"; message: string }
  | { kind: "notice"; message: string };

export function PluginsSection() {
  const [catalog, setCatalog] = useState<CatalogState>({ kind: "loading" });
  const [url, setUrl] = useState("");
  const [mutation, setMutation] = useState<MutationState>({ kind: "idle" });

  async function refresh() {
    try {
      setCatalog({ kind: "ready", plugins: await listPlugins() });
    } catch (cause) {
      setCatalog({
        kind: "error",
        message: cause instanceof Error ? cause.message : "Installed plugins could not be loaded",
      });
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  const busy = mutation.kind === "pending";

  async function runMutation(
    verb: "installing" | "updating" | "removing",
    run: () => Promise<string>,
    fallbackError: string,
  ) {
    if (busy) return;
    setMutation({ kind: "pending", verb });
    try {
      const message = await run();
      setMutation({ kind: "notice", message });
      await refresh();
    } catch (cause) {
      setMutation({
        kind: "error",
        message: cause instanceof Error ? cause.message : fallbackError,
      });
    }
  }

  // SettingsEditor wraps the sections in `<form>`; HTML forbids nested forms,
  // so install controls are a div with a button click handler.
  function handleInstall() {
    if (!url.trim()) return;
    return runMutation(
      "installing",
      async () => {
        const plugin = await installPlugin(url.trim());
        setUrl("");
        return `${plugin.name} v${plugin.version ?? "unknown"} installed.`;
      },
      "The plugin could not be installed",
    );
  }

  function handleUpdate(id: string) {
    return runMutation(
      "updating",
      async () => {
        const plugin = await updatePlugin(id);
        return `${plugin.name} updated to v${plugin.version ?? "unknown"}.`;
      },
      "The plugin could not be updated",
    );
  }

  function handleRemove(id: string) {
    return runMutation(
      "removing",
      async () => {
        await uninstallPlugin(id);
        return `${id} removed.`;
      },
      "The plugin could not be removed",
    );
  }

  return (
    <div className="grid gap-4">
      <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
        <CardHeader className="border-b border-border/70 px-5 py-4 sm:px-6">
          <CardTitle className="text-base font-semibold">Install a plugin</CardTitle>
          <p className="text-sm text-muted-foreground">
            Plugins are downloaded from any git repository URL that contains a{" "}
            <code className="rounded bg-muted px-1 py-0.5 text-xs">vesta-plugin.toml</code> manifest
            at its root.
          </p>
        </CardHeader>
        <CardContent className="grid gap-3 px-5 py-5 sm:px-6">
          <p className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300">
            Only install plugins from sources you trust. Plugin code runs in your browser with the
            page's own access, and the author of the repository can change it at any time.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <input
              className={`${inputClass} min-w-0 flex-1`}
              type="url"
              required
              placeholder="https://github.com/example/vesta-plugin.git"
              aria-label="Git repository URL"
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              disabled={busy}
            />
            <button
              className={buttonClass}
              type="button"
              onClick={() => void handleInstall()}
              disabled={busy || !url.trim()}
            >
              {mutation.kind === "pending" && mutation.verb === "installing"
                ? "Installing…"
                : "Install"}
            </button>
          </div>
          {mutation.kind === "error" && (
            <p className="text-sm text-destructive" role="alert">
              {mutation.message}
            </p>
          )}
          {mutation.kind === "notice" && (
            <p className="text-sm text-muted-foreground" role="status">
              {mutation.message}
            </p>
          )}
        </CardContent>
      </Card>

      <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
        <CardHeader className="border-b border-border/70 px-5 py-4 sm:px-6">
          <CardTitle className="text-base font-semibold">Installed plugins</CardTitle>
          <p className="text-sm text-muted-foreground">
            Each plugin contributes widgets to the Add tile menu on the dashboard.
          </p>
        </CardHeader>
        <CardContent className="grid gap-3 px-5 py-5 sm:px-6">
          {catalog.kind === "error" && (
            <p className="text-sm text-destructive" role="alert">
              {catalog.message}
            </p>
          )}
          {catalog.kind === "ready" && catalog.plugins.length === 0 && (
            <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
              No plugins installed yet.
            </p>
          )}
          {catalog.kind === "ready" &&
            catalog.plugins.map((plugin) => (
              <article
                className="grid gap-2 rounded-lg border border-border/70 bg-background/60 p-4"
                key={plugin.id}
              >
                <div className="flex flex-wrap items-baseline justify-between gap-2">
                  <h3 className="text-sm font-medium">
                    {plugin.name}
                    {plugin.version && (
                      <span className="ml-2 text-xs text-muted-foreground">v{plugin.version}</span>
                    )}
                  </h3>
                  <div className="flex gap-2">
                    <button
                      className={buttonClass}
                      type="button"
                      onClick={() => void handleUpdate(plugin.id)}
                      disabled={busy}
                    >
                      Update
                    </button>
                    <button
                      className={destructiveButtonClass}
                      type="button"
                      onClick={() => void handleRemove(plugin.id)}
                      disabled={busy}
                    >
                      Remove
                    </button>
                  </div>
                </div>
                {plugin.description && (
                  <p className="text-sm leading-5 text-muted-foreground">{plugin.description}</p>
                )}
                <p
                  className="truncate text-xs text-muted-foreground"
                  title={`${plugin.source_url} @ ${plugin.commit.slice(0, 12)}`}
                >
                  {plugin.source_url} @ {plugin.commit.slice(0, 12)}
                </p>
                <ul className="grid gap-1">
                  {plugin.widgets.map((widget) => (
                    <li className="text-sm" key={widget.id}>
                      <span className="font-medium">{widget.title}</span>{" "}
                      <span className="text-xs text-muted-foreground">
                        {widget.width}&times;{widget.height}
                      </span>
                      {widget.description && (
                        <span className="block text-xs text-muted-foreground">
                          {widget.description}
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              </article>
            ))}
        </CardContent>
      </Card>
    </div>
  );
}
