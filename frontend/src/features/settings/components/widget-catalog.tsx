import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { WidgetPreview } from "@/features/widgets/widget-preview";
import type { WidgetDescriptor } from "@/shared/schemas/core";

function WidgetCatalog({
  widgets,
  onOpenPlugins,
}: {
  widgets: WidgetDescriptor[];
  onOpenPlugins: () => void;
}) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="border-b border-border/70 px-5 py-4 sm:px-6">
        <CardTitle className="text-base font-semibold">Built-in widgets</CardTitle>
        <p className="text-sm text-muted-foreground">
          Widgets are independent dashboard tiles. Add and configure them from the homepage.
        </p>
        <a
          className="text-sm font-medium text-primary underline-offset-4 hover:underline"
          href="/#dashboard-items"
        >
          Manage widgets on the homepage
        </a>
      </CardHeader>
      <CardContent className="grid gap-3 px-5 py-5 sm:px-6">
        {widgets.length === 0 ? (
          <p className="text-sm text-muted-foreground">No widget handlers are registered.</p>
        ) : (
          <ul className="grid gap-3 sm:grid-cols-2">
            {widgets.map((widget) => (
              <li
                className="rounded-md border border-border/70 bg-background/60 p-4"
                key={widget.name}
              >
                <h3 className="text-sm font-medium">
                  {widget.title || widget.name}
                  <span className="ml-2 text-xs text-muted-foreground">{widget.name}</span>
                </h3>
                <p className="mt-1 text-sm leading-5 text-muted-foreground">{widget.description}</p>
                <div className="mt-3 h-40 overflow-hidden rounded-md border border-border/70 bg-background p-3">
                  <WidgetPreview name={widget.name} />
                </div>
                <p className="mt-3 text-xs text-muted-foreground">
                  {widget.fields.length === 0
                    ? "No configuration required"
                    : `${widget.fields.length} structured ${widget.fields.length === 1 ? "setting" : "settings"}`}
                </p>
              </li>
            ))}
          </ul>
        )}
        <p className="border-t border-border/70 pt-3 text-sm leading-6 text-muted-foreground">
          Third-party widgets come from plugins: install one from a git URL on the{" "}
          <button
            className="text-primary underline-offset-4 hover:underline"
            type="button"
            onClick={onOpenPlugins}
          >
            Plugins
          </button>{" "}
          tab, and its widgets appear here and in the dashboard's Add tile menu. Plugin code runs in
          your browser, so only install repositories you trust.
        </p>
      </CardContent>
    </Card>
  );
}

export { WidgetCatalog };
