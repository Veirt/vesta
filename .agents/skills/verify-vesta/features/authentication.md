# First-run access

Vesta protects Settings with an administrator password and a browser session cookie. First launch requires creating the administrator; later visits require signing in.

## Sub-features

- First-run administrator setup, including password confirmation and the 12-character minimum.
- Sign-in with an existing administrator password.
- Sign-out and return to the sign-in form.
- Authenticated access to Settings and denied unauthenticated access to `/api/settings`.

## How to get to it (user POV)

Open `/settings`. A fresh SQLite database shows the account-creation form. After creating the administrator, the Settings editor replaces the form. Sign out from the editor to return to the sign-in form; enter the same password to regain Settings.

## Driving it with playwright-cli

Open `/settings` in the run's Firefox session and capture a snapshot. For a fresh database, drive the exact setup controls:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" open "$BASE_URL/settings" --browser=firefox
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#admin-password' "$VERIFY_PASSWORD"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#admin-password-confirm' "$VERIFY_PASSWORD"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Create administrator account")'
```

After the Settings editor appears, sign out with the visible button, then sign in again with the same disposable password. Capture screenshots before setup, after editor access, after sign-out, and after sign-in. The doctor separately confirms the auth-status response and public 401 boundary.

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Sign out")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#admin-password' "$VERIFY_PASSWORD"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Sign in")'
```

## Gotchas

- A fresh run must show account setup. If it shows sign-in instead, the database is not fresh; report the precondition failure and create a new run ID rather than bypassing setup.
- Passwords must contain 12–256 characters. Use a disposable verification password only; never reuse a real administrator credential or save the temporary password in evidence.
- The session cookie is HTTP-only, so browser UI state and the protected Settings page are the user-facing proof. Do not try to read the cookie from page JavaScript.
