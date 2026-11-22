import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Field } from "@/shared/components/form-field";
import { inputClass, selectClass } from "@/shared/components/styles";
import type { AppSettings, Theme } from "@/shared/schemas/core";

function AppearanceSection({
  settings,
  onChange,
}: {
  settings: AppSettings;
  onChange: (update: Partial<AppSettings>) => void;
}) {
  return (
    <Card className="gap-0 border-border/70 bg-card/60 py-0 shadow-none">
      <CardHeader className="border-b border-border/70 px-4 py-4 sm:px-6">
        <CardTitle className="text-base font-semibold">Appearance</CardTitle>
        <p className="text-sm text-muted-foreground">
          Set the name shown on the dashboard and choose a color theme.
        </p>
      </CardHeader>
      <CardContent className="grid min-w-0 gap-5 px-4 py-4 sm:grid-cols-2 sm:px-6 sm:py-5">
        <Field label="Dashboard title" required>
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
            className={selectClass}
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
