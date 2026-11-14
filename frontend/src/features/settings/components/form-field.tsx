import type { ReactNode } from "react";

function Field({
  label,
  children,
  description,
}: {
  label: string;
  children: ReactNode;
  description?: string;
}) {
  return (
    <div className="grid min-w-0 gap-2">
      <label className="grid gap-2 text-sm font-medium">
        {label}
        {children}
      </label>
      {description && <p className="text-xs leading-5 text-muted-foreground">{description}</p>}
    </div>
  );
}

export { Field };
