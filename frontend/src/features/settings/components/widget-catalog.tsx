import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { WidgetDescriptor } from "@/features/settings/schema";

function WidgetCatalog({ widgets }: { widgets: WidgetDescriptor[] }) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="border-b border-border/70 px-5 py-4 sm:px-6">
        <CardTitle className="text-base font-semibold">Built-in widgets</CardTitle>
        <p className="text-sm text-muted-foreground">
          Choose a widget on a service card to see only the settings it needs.
        </p>
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
                <h3 className="text-sm font-medium">{widget.name}</h3>
                <p className="mt-1 text-sm leading-5 text-muted-foreground">{widget.description}</p>
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
          GitHub URL installation and third-party widget execution are not implemented. Any future
          installer should use a versioned, permission-limited boundary rather than run repository
          code inside Vesta.
        </p>
      </CardContent>
    </Card>
  );
}

export { WidgetCatalog };
