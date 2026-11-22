import { IconPencil, IconPlus } from "@tabler/icons-react";
import { AddTileMenu } from "@/features/dashboard/add-tile-menu";
import type { ValidationIssue } from "@/shared/lib/validation";
import { useDashboardEditor } from "@/features/dashboard/use-dashboard-editor";
import { buttonClass, primaryButtonClass } from "@/shared/components/styles";

function DashboardItemsHeader({
  authenticated,
  editing,
  saving,
  dirty,
  loading,
  hasIssues,
  onEdit,
  onDiscard,
  onSave,
}: {
  authenticated: boolean;
  editing: boolean;
  saving: boolean;
  dirty: boolean;
  loading: boolean;
  hasIssues: boolean;
  onEdit: () => void;
  onDiscard: () => void;
  onSave: () => void;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 id="dashboard-items-heading" className="text-lg font-semibold tracking-tight">
          Apps and widgets
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Keep service links and live widgets side by side, in any order.
        </p>
      </div>
      {authenticated && !editing && (
        <button className={buttonClass} type="button" onClick={onEdit}>
          <IconPencil size={16} aria-hidden="true" /> Edit dashboard
        </button>
      )}
      {editing && (
        <div className="flex flex-wrap items-center gap-2">
          <button className={buttonClass} type="button" onClick={onDiscard} disabled={saving}>
            Discard
          </button>
          <button
            className={primaryButtonClass}
            type="button"
            onClick={onSave}
            disabled={!dirty || saving || loading || hasIssues}
            aria-describedby={hasIssues ? "dashboard-validation" : undefined}
          >
            {saving ? "Saving…" : "Save dashboard"}
          </button>
        </div>
      )}
    </div>
  );
}

function DashboardEditToolbar({
  editor,
  onSelectEditor,
  onSelectGroup,
}: {
  editor: ReturnType<typeof useDashboardEditor>;
  onSelectEditor: (editorId: string) => void;
  onSelectGroup: (groupId: string) => void;
}) {
  const widgets = editor.widgets;
  const ready = editor.ready;
  const loading = editor.loading;
  return (
    <div className="dashboard-edit-toolbar">
      <AddTileMenu
        widgets={widgets}
        align="start"
        trigger={
          <button className={buttonClass} type="button" disabled={!ready || loading}>
            <IconPlus size={16} aria-hidden="true" /> Add tile
          </button>
        }
        onAddService={() => {
          const editorId = editor.addService();
          if (editorId) onSelectEditor(editorId);
        }}
        onAddWidget={(name) => {
          const editorId = editor.addWidget(name);
          if (editorId) onSelectEditor(editorId);
        }}
      />
      <button
        className={buttonClass}
        type="button"
        onClick={() => onSelectGroup(editor.addGroup())}
        disabled={!ready || loading}
      >
        <IconPlus size={16} aria-hidden="true" /> Add group
      </button>
      <span className="text-xs text-muted-foreground">
        {loading && !ready
          ? "Loading your dashboard layout…"
          : "Drag any tile to arrange it. Use the pencil to edit, group, or resize; on narrow screens, use the edit panel to move and size tiles."}
      </span>
    </div>
  );
}

function DashboardValidationIssues({ issues }: { issues: ValidationIssue[] }) {
  return (
    <div
      id="dashboard-validation"
      className="grid gap-1 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
      role="alert"
    >
      <p>Fix these dashboard details before saving:</p>
      <ul className="list-inside list-disc">
        {issues.map((issue) => (
          <li key={`${issue.id}-${issue.message}`}>
            {issue.title || "Untitled tile"}: {issue.message}
          </li>
        ))}
      </ul>
    </div>
  );
}

function DashboardEditorAlert({
  message,
  showRetry,
  showSignIn,
  onRetryLoad,
}: {
  message: string;
  showRetry: boolean;
  showSignIn: boolean;
  onRetryLoad: () => void;
}) {
  return (
    <div
      className="grid justify-items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
      role="alert"
    >
      <p>{message}</p>
      {showRetry && (
        <button className={buttonClass} type="button" onClick={onRetryLoad}>
          Try again
        </button>
      )}
      {showSignIn && (
        <a className="underline underline-offset-4" href="/settings">
          Open Settings to sign in
        </a>
      )}
    </div>
  );
}

function DashboardEditBanners({
  editing,
  editor,
  validationIssues,
  alert,
  onSelectEditor,
  onSelectGroup,
}: {
  editing: boolean;
  editor: ReturnType<typeof useDashboardEditor>;
  validationIssues: ValidationIssue[];
  alert: { message: string; showRetry: boolean; showSignIn: boolean } | null;
  onSelectEditor: (editorId: string) => void;
  onSelectGroup: (groupId: string) => void;
}) {
  if (!editing) return null;
  return (
    <>
      <DashboardEditToolbar
        editor={editor}
        onSelectEditor={onSelectEditor}
        onSelectGroup={onSelectGroup}
      />
      {validationIssues.length > 0 && <DashboardValidationIssues issues={validationIssues} />}
      {alert && <DashboardEditorAlert {...alert} onRetryLoad={editor.retryLoad} />}
      {editor.notice && (
        <p className="text-sm text-muted-foreground" role="status">
          {editor.notice}
        </p>
      )}
    </>
  );
}

export { DashboardEditBanners, DashboardItemsHeader };
