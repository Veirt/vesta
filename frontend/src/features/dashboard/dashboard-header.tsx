import { IconMenu2, IconSearch, IconX } from "@tabler/icons-react";
import { Input } from "@/components/ui/input";

function DashboardHeader({
  mobileMenuOpen,
  query,
  onQueryChange,
  onToggleMenu,
}: {
  mobileMenuOpen: boolean;
  query: string;
  onQueryChange: (query: string) => void;
  onToggleMenu: () => void;
}) {
  return (
    <header className="sticky top-0 z-20 flex min-h-16 items-center gap-3 border-b border-border/70 bg-background/90 px-4 backdrop-blur-xl sm:px-6 lg:px-8">
      <button
        className="grid size-9 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring md:hidden"
        type="button"
        aria-label={mobileMenuOpen ? "Close navigation" : "Open navigation"}
        aria-controls="sidebar"
        aria-expanded={mobileMenuOpen}
        onClick={onToggleMenu}
      >
        {mobileMenuOpen ? <IconX size={19} /> : <IconMenu2 size={19} />}
      </button>
      <div className="relative w-full max-w-md">
        <IconSearch
          className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground"
          size={17}
          stroke={1.8}
          aria-hidden="true"
        />
        <label className="screen-reader-only" htmlFor="dashboard-search">
          Search servers, services, and widgets
        </label>
        <Input
          id="dashboard-search"
          type="search"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder="Search servers, services, and widgets…"
          className="h-10 border-border/70 bg-muted/40 pl-9 pr-12 text-sm shadow-none placeholder:text-muted-foreground/80 focus-visible:ring-1"
        />
        <kbd className="pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 rounded border border-border bg-background px-1.5 py-0.5 font-mono text-[0.65rem] text-muted-foreground">
          /
        </kbd>
      </div>
    </header>
  );
}

export { DashboardHeader };
