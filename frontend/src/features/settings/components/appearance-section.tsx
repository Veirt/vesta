import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Field } from "@/features/settings/components/form-field";
import { inputClass } from "@/features/settings/components/styles";
import type { AppSettings, Theme } from "@/features/settings/schema";

function AppearanceSection({
  settings,
  onChange,
}: {
  settings: AppSettings;
  onChange: (update: Partial<AppSettings>) => void;
}) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="border-b border-border/70 px-5 py-4 sm:px-6">
        <CardTitle className="text-base font-semibold">Appearance</CardTitle>
        <p className="text-sm text-muted-foreground">
          Set the name shown on the dashboard and choose a color theme.
        </p>
      </CardHeader>
      <CardContent className="grid gap-5 px-5 py-5 sm:grid-cols-2 sm:px-6">
        <Field label="Dashboard title">
          <Input
            value={settings.dashboard_title}
            maxLength={80}
            required
            onChange={(event) => onChange({ dashboard_title: event.target.value })}
            className={inputClass}
          />
        </Field>
        <Field label="Theme">
          <select
            className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            value={settings.theme}
            onChange={(event) => onChange({ theme: event.target.value as Theme })}
          >
            <option value="system">Use system setting</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </Field>
      </CardContent>
    </Card>
  );
}

export { AppearanceSection };
