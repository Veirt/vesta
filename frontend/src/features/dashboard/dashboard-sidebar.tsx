import { IconHome, IconLayoutGrid, IconServer, IconSettings } from "@tabler/icons-react";

const navItems = [
  { href: "#overview", label: "Overview", Icon: IconHome },
  { href: "#nodes", label: "Servers", Icon: IconServer },
  { href: "#dashboard-items", label: "Apps and widgets", Icon: IconLayoutGrid },
];

/** Fixed left rail with section navigation and the settings link. */
function AppSidebar({ mobileMenuOpen, onClose }: { mobileMenuOpen: boolean; onClose: () => void }) {
  return (
    <aside
      id="sidebar"
      className={`app-sidebar fixed inset-y-0 left-0 z-30 flex w-[4.5rem] flex-col items-center border-r border-border/70 bg-background px-2.5 py-4 transition-transform md:translate-x-0 ${mobileMenuOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"}`}
      aria-label="Main navigation"
    >
      <a
        href="#overview"
        className="mb-8 grid size-10 place-items-center rounded-lg text-foreground transition-colors hover:bg-muted"
        aria-label="Vesta overview"
        onClick={onClose}
      >
        <span className="text-xl font-bold tracking-[-0.12em]">V</span>
      </a>
      <nav
        className="flex w-full flex-1 flex-col items-center gap-2"
        aria-label="Dashboard sections"
      >
        {navItems.map(({ href, label, Icon }) => (
          <a
            key={href}
            href={href}
            className="grid size-10 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
            aria-label={label}
            title={label}
            onClick={onClose}
          >
            <Icon size={19} stroke={1.7} aria-hidden="true" />
          </a>
        ))}
      </nav>
      <a
        href="/settings"
        className="grid size-10 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
        aria-label="Settings"
        title="Settings"
      >
        <IconSettings size={19} stroke={1.7} aria-hidden="true" />
      </a>
    </aside>
  );
}

export { AppSidebar };
