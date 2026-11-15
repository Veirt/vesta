import type { ReactNode } from "react";

function Field({
  label,
  children,
  description,
  required = false,
}: {
  label: string;
  children: ReactNode;
  description?: string;
  required?: boolean;
}) {
  return (
    <div className="grid min-w-0 gap-2">
      <label className="grid gap-2 text-sm font-medium">
        <span>
          {label}
          {required && (
            <span className="ml-1 text-destructive" aria-hidden="true">
              *
            </span>
          )}
        </span>
        {children}
      </label>
      {description && <p className="text-xs leading-5 text-muted-foreground">{description}</p>}
    </div>
  );
}

export { Field };
