import { useState, type FormEvent } from "react";
import { AppearanceSection } from "@/features/settings/components/appearance-section";
import { NodesSection } from "@/features/settings/components/nodes-section";
import { ServiceGroupsSection } from "@/features/settings/components/service-groups-section";
import { WidgetCatalog } from "@/features/settings/components/widget-catalog";
import { buttonClass, primaryButtonClass } from "@/features/settings/components/styles";
import { useSettingsEditor } from "@/features/settings/use-settings-editor";

const sections = [
  { id: "appearance", label: "Appearance", group: "General" },
  { id: "servers", label: "Servers", group: "Infrastructure" },
  { id: "services", label: "Service groups", group: "Dashboard" },
  { id: "widgets", label: "Widgets", group: "Dashboard" },
] as const;

type SectionId = (typeof sections)[number]["id"];

function SettingsEditor({ onSignOut }: { onSignOut: () => void }) {
  const editor = useSettingsEditor();
  const [activeSection, setActiveSection] = useState<SectionId>("appearance");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void editor.save();
  }

  const activeTitle = sections.find(({ id }) => id === activeSection)?.label ?? "Appearance";

  return (
    <form className="grid gap-5 lg:gap-7" onSubmit={handleSubmit}>
      <header className="flex flex-wrap items-end justify-between gap-4 border-l-2 border-primary pl-4">
        <div>
          <p className="text-sm text-muted-foreground">Vesta dashboard</p>
          <h1 className="mt-1 text-2xl font-semibold tracking-tight">Settings</h1>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            Configure your servers, service cards, widgets, and dashboard appearance.
          </p>
        </div>
        <button className={buttonClass} type="button" onClick={onSignOut}>
          Sign out
        </button>
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
            <h2 className="mb-3 hidden px-3 text-xs font-medium text-muted-foreground lg:block">
              Settings
            </h2>
            <div className="flex gap-1 overflow-x-auto border-b border-border pb-2 lg:grid lg:gap-5 lg:overflow-visible lg:border-0 lg:pb-0">
              {(["General", "Infrastructure", "Dashboard"] as const).map((group) => (
                <div className="grid shrink-0 gap-1" key={group}>
                  <h3 className="hidden px-3 text-xs font-medium text-muted-foreground lg:block">
                    {group}
                  </h3>
                  {sections
                    .filter((section) => section.group === group)
                    .map((section) => (
                      <button
                        className={`min-h-10 rounded-md px-3 text-left text-sm transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${activeSection === section.id ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground"}`}
                        type="button"
                        key={section.id}
                        aria-current={activeSection === section.id ? "page" : undefined}
                        onClick={() => setActiveSection(section.id)}
                      >
                        {section.label}
                      </button>
                    ))}
                </div>
              ))}
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
            {activeSection === "services" && (
              <ServiceGroupsSection
                groups={editor.groups}
                nodes={editor.nodes}
                widgets={editor.widgets}
                uploading={editor.uploading}
                onAddGroup={editor.addGroup}
                onChangeGroup={editor.updateGroup}
                onRemoveGroup={editor.removeGroup}
                onMoveGroup={editor.moveGroup}
                onAddService={editor.addService}
                onMoveService={editor.moveService}
                onResizeService={editor.resizeService}
                onLayoutChange={editor.updateServiceLayout}
                onUpload={editor.uploadImage}
              />
            )}
            {activeSection === "widgets" && <WidgetCatalog widgets={editor.widgets} />}
          </section>
        </div>
      )}
    </form>
  );
}

export { SettingsEditor };
