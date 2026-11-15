import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { installPlugin, listPlugins, uninstallPlugin } from "@/features/settings/api";
import type { Plugin } from "@/features/settings/schema";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
} from "@/features/settings/components/styles";

function PluginsSection() {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loadError, setLoadError] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  async function refresh() {
    try {
      setPlugins(await listPlugins());
      setLoadError("");
    } catch (cause) {
      setLoadError(
        cause instanceof Error ? cause.message : "Installed plugins could not be loaded",
      );
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  // This section lives inside the settings page's outer <form>; HTML forbids
  // nested forms, so the install controls are a div and the button handler is
  // wired directly.
  async function handleInstall() {
    if (!url.trim() || busy) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const plugin = await installPlugin(url.trim());
      setNotice(`${plugin.name} v${plugin.version ?? "unknown"} installed.`);
      setUrl("");
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The plugin could not be installed");
    } finally {
      setBusy(false);
    }
  }

  async function handleRemove(id: string) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await uninstallPlugin(id);
      setNotice(`${id} removed.`);
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The plugin could not be removed");
    } finally {
      setBusy(false);
    }
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
              {busy ? "Installing…" : "Install"}
            </button>
          </div>
          {error && (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          )}
          {notice && (
            <p className="text-sm text-muted-foreground" role="status">
              {notice}
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
          {loadError && (
            <p className="text-sm text-destructive" role="alert">
              {loadError}
            </p>
          )}
          {!loadError && plugins.length === 0 && (
            <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
              No plugins installed yet.
            </p>
          )}
          {plugins.map((plugin) => (
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
                <button
                  className={destructiveButtonClass}
                  type="button"
                  onClick={() => void handleRemove(plugin.id)}
                  disabled={busy}
                >
                  Remove
                </button>
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

export { PluginsSection };
