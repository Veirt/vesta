const inputClass = "bg-background";
const buttonClass =
  "inline-flex min-h-9 items-center justify-center gap-2 rounded-md border border-border bg-card px-3 text-sm font-medium text-foreground transition-colors hover:bg-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50";
const primaryButtonClass =
  "inline-flex min-h-9 items-center justify-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50";
const destructiveButtonClass =
  "inline-flex min-h-9 items-center justify-center gap-2 rounded-md border border-destructive/40 px-3 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

export { buttonClass, destructiveButtonClass, inputClass, primaryButtonClass };
