import { useEffect, useState, type FormEvent } from "react";
import { AppearanceSection } from "@/features/settings/components/appearance-section";
import { NodesSection } from "@/features/settings/components/nodes-section";
import { PluginsSection } from "@/features/settings/components/plugins-section";
import { WidgetCatalog } from "@/features/settings/components/widget-catalog";
import { buttonClass, primaryButtonClass } from "@/shared/components/styles";
import { useSettingsEditor } from "@/features/settings/use-settings-editor";

const sections = [
  { id: "appearance", label: "Appearance", group: "General" },
  { id: "servers", label: "Servers", group: "Infrastructure" },
  { id: "widgets", label: "Widgets", group: "Dashboard" },
  { id: "plugins", label: "Plugins", group: "Dashboard" },
] as const;

type SectionId = (typeof sections)[number]["id"];

function sectionFromHash(): SectionId {
  const section = window.location.hash.slice(1);
  return sections.some(({ id }) => id === section) ? (section as SectionId) : "appearance";
}

function SettingsEditor() {
  const editor = useSettingsEditor();
  const [activeSection, setActiveSection] = useState<SectionId>(sectionFromHash);

  useEffect(() => {
    const syncSection = () => setActiveSection(sectionFromHash());
    window.addEventListener("hashchange", syncSection);
    return () => window.removeEventListener("hashchange", syncSection);
  }, []);

  function selectSection(section: SectionId) {
    setActiveSection(section);
    if (window.location.hash !== `#${section}`) window.location.hash = section;
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void editor.save();
  }

  const activeTitle = sections.find(({ id }) => id === activeSection)?.label ?? "Appearance";

  return (
    <form className="grid gap-5 lg:gap-7" onSubmit={handleSubmit}>
      <header className="border-l-2 border-primary pl-4">
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
      </header>

      {editor.dirty && (
        <div className="sticky top-0 z-20 -mx-4 flex flex-wrap items-center justify-between gap-3 border-y border-border bg-background/95 px-4 py-3 backdrop-blur sm:-mx-6 sm:px-6 lg:-mx-8 lg:px-8">
          <p className="text-sm text-muted-foreground" role="status">
            Unsaved changes
          </p>
          <div className="flex items-center gap-2">
            <button
              className={buttonClass}
              type="button"
              onClick={editor.discardChanges}
              disabled={editor.saving}
            >
              Discard changes
            </button>
            <button
              className={primaryButtonClass}
              type="submit"
              disabled={editor.saving || editor.uploading}
            >
              {editor.saving ? "Saving…" : "Save changes"}
            </button>
          </div>
        </div>
      )}

      {editor.error && (
        <p
          className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {editor.error}
        </p>
      )}
      {editor.notice && (
        <p className="rounded-md border border-primary/30 bg-primary/5 p-3 text-sm" role="status">
          {editor.notice}
        </p>
      )}

      {editor.loading ? (
        <p className="p-4 text-sm text-muted-foreground" role="status">
          Loading dashboard settings…
        </p>
      ) : editor.loadError ? (
        <div className="grid justify-items-start gap-3 rounded-lg border border-border/70 bg-card/60 p-5">
          <p className="text-sm text-muted-foreground" role="alert">
            {editor.loadError}
          </p>
          <button className={buttonClass} type="button" onClick={editor.retryLoad}>
            Try again
          </button>
        </div>
      ) : (
        <div className="grid gap-5 lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-8">
          <nav aria-label="Settings sections" className="min-w-0 lg:sticky lg:top-20 lg:self-start">
            <div className="lg:hidden">
              <label className="sr-only" htmlFor="settings-section-select">
                Settings section
              </label>
              <select
                id="settings-section-select"
                className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={activeSection}
                onChange={(event) => selectSection(event.target.value as SectionId)}
              >
                {(["General", "Infrastructure", "Dashboard"] as const).map((group) => (
                  <optgroup key={group} label={group}>
                    {sections
                      .filter((section) => section.group === group)
                      .map((section) => (
                        <option key={section.id} value={section.id}>
                          {section.label}
                        </option>
                      ))}
                  </optgroup>
                ))}
              </select>
            </div>
            <div className="hidden lg:block">
              <div className="grid gap-5">
                {(["General", "Infrastructure", "Dashboard"] as const).map((group) => (
                  <div className="grid gap-1" key={group}>
                    <h3 className="px-3 text-xs font-medium text-muted-foreground">{group}</h3>
                    {sections
                      .filter((section) => section.group === group)
                      .map((section) => (
                        <button
                          className={`min-h-10 rounded-md px-3 text-left text-sm transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${activeSection === section.id ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground"}`}
                          type="button"
                          key={section.id}
                          aria-current={activeSection === section.id ? "page" : undefined}
                          onClick={() => selectSection(section.id)}
                        >
                          {section.label}
                        </button>
                      ))}
                  </div>
                ))}
              </div>
            </div>
          </nav>

          <section className="min-w-0 space-y-4" aria-labelledby="settings-section-title">
            <div>
              <h2 id="settings-section-title" className="text-xl font-semibold tracking-tight">
                {activeTitle}
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                Settings are saved to config.toml.
              </p>
            </div>
            {activeSection === "appearance" && (
              <AppearanceSection settings={editor.app} onChange={editor.updateApp} />
            )}
            {activeSection === "servers" && (
              <NodesSection
                nodes={editor.nodes}
                uploading={editor.uploading}
                onAdd={editor.addNode}
                onChange={editor.updateNode}
                onMove={editor.moveNode}
                onRemove={editor.removeNode}
                onUpload={editor.uploadImage}
              />
            )}
            {activeSection === "widgets" && (
              <WidgetCatalog
                widgets={editor.widgets}
                onOpenPlugins={() => selectSection("plugins")}
              />
            )}
            {activeSection === "plugins" && (
              <PluginsSection onPluginsChanged={() => void editor.refreshWidgets()} />
            )}
          </section>
        </div>
      )}
    </form>
  );
}

export { SettingsEditor };
