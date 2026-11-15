import { IconArrowLeft } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { AuthPanel } from "@/features/settings/components/auth-panel";
import { buttonClass } from "@/features/settings/components/styles";
import { SettingsEditor } from "@/features/settings/settings-editor";
import { fetchAuthStatus, signOut } from "@/features/settings/api";
import type { AuthStatus } from "@/features/settings/schema";

function SettingsPage() {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [authError, setAuthError] = useState("");
  const [authRevision, setAuthRevision] = useState(0);

  useEffect(() => {
    let active = true;
    fetchAuthStatus()
      .then((result) => {
        if (active) {
          setStatus(result);
          setAuthError("");
        }
      })
      .catch((cause: unknown) => {
        if (active)
          setAuthError(
            cause instanceof Error ? cause.message : "Authentication status could not be loaded.",
          );
      });
    return () => {
      active = false;
    };
  }, [authRevision]);

  useEffect(() => {
    document.title = "Settings · Vesta";
  }, []);

  async function handleSignOut() {
    try {
      await signOut();
      setStatus({ setup_required: false, authenticated: false });
      setAuthError("");
    } catch (cause) {
      setAuthError(cause instanceof Error ? cause.message : "Could not sign out.");
    }
  }

  function handleAuthenticated() {
    setStatus({ setup_required: false, authenticated: true });
    setAuthError("");
  }

  function retryAuthStatus() {
    setAuthError("");
    setAuthRevision((revision) => revision + 1);
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <main className="mx-auto grid min-h-screen w-full max-w-6xl content-start gap-6 p-4 sm:p-6 lg:gap-8 lg:p-10">
        <header className="flex flex-wrap items-center justify-between gap-4">
          <a
            className="inline-flex items-center gap-3 rounded-md focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
            href="/"
          >
            <span className="grid size-10 place-items-center rounded-lg border border-border bg-card text-lg font-semibold">
              V
            </span>
            <span className="text-sm font-medium">Vesta</span>
          </a>
          <nav aria-label="Settings navigation" className="flex flex-wrap items-center gap-2">
            <a
              className="inline-flex min-h-10 items-center gap-2 px-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
              href="/"
            >
              <IconArrowLeft size={17} stroke={1.8} aria-hidden="true" /> Back to dashboard
            </a>
            {status?.authenticated && (
              <button className={buttonClass} type="button" onClick={handleSignOut}>
                Sign out
              </button>
            )}
          </nav>
        </header>

        {authError && (
          <p
            className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
            role="alert"
          >
            {authError}
          </p>
        )}
        {!status && authError ? (
          <button
            className={`${buttonClass} justify-self-start`}
            type="button"
            onClick={retryAuthStatus}
          >
            Try again
          </button>
        ) : !status ? (
          <Card className="mx-auto w-full max-w-lg border-border/70 bg-card/70 shadow-none">
            <CardContent className="p-5 text-sm text-muted-foreground" role="status">
              Checking administrator access…
            </CardContent>
          </Card>
        ) : status.authenticated ? (
          <SettingsEditor />
        ) : (
          <AuthPanel setupRequired={status.setup_required} onSuccess={handleAuthenticated} />
        )}
      </main>
    </div>
  );
}

export { SettingsPage };
