import { IconSettings } from "@tabler/icons-react";
import { useState, type FormEvent } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { submitPassword } from "@/features/settings/api";
import { inputClass, primaryButtonClass } from "@/features/settings/components/styles";

function AuthPanel({
  setupRequired,
  onSuccess,
}: {
  setupRequired: boolean;
  onSuccess: () => void;
}) {
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    if (setupRequired && password !== confirmation) {
      setError("The passwords do not match.");
      return;
    }
    if (setupRequired && password.length < 12) {
      setError("Choose a password with at least 12 characters.");
      return;
    }

    setBusy(true);
    try {
      await submitPassword(setupRequired ? "setup" : "login", password);
      onSuccess();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Authentication failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card className="mx-auto w-full max-w-lg gap-0 border-border/70 bg-card/70 py-0 shadow-none">
      <CardHeader className="gap-3 border-b border-border/70 px-5 py-5 sm:px-6">
        <span className="grid size-10 place-items-center rounded-lg bg-muted text-primary">
          <IconSettings size={20} stroke={1.7} aria-hidden="true" />
        </span>
        <div className="space-y-1">
          <CardTitle className="text-lg font-semibold">
            {setupRequired ? "Create your administrator password" : "Sign in to Vesta"}
          </CardTitle>
          <p className="text-sm leading-6 text-muted-foreground">
            {setupRequired
              ? "This password protects dashboard settings and icon uploads. It is stored as a password hash in the database."
              : "Your session is protected by a secure, HTTP-only cookie."}
          </p>
        </div>
      </CardHeader>
      <CardContent className="px-5 py-5 sm:px-6">
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <label className="grid gap-2 text-sm font-medium" htmlFor="admin-password">
            Password
            <Input
              id="admin-password"
              type="password"
              autoComplete={setupRequired ? "new-password" : "current-password"}
              minLength={setupRequired ? 12 : undefined}
              required
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className={inputClass}
            />
          </label>
          {setupRequired && (
            <label className="grid gap-2 text-sm font-medium" htmlFor="admin-password-confirm">
              Confirm password
              <Input
                id="admin-password-confirm"
                type="password"
                autoComplete="new-password"
                required
                value={confirmation}
                onChange={(event) => setConfirmation(event.target.value)}
                className={inputClass}
              />
            </label>
          )}
          {error && (
            <p
              className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
              role="alert"
            >
              {error}
            </p>
          )}
          <button className={primaryButtonClass} type="submit" disabled={busy}>
            {busy ? "Please wait…" : setupRequired ? "Create administrator account" : "Sign in"}
          </button>
        </form>
      </CardContent>
    </Card>
  );
}

export { AuthPanel };
