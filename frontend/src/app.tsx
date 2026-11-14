import {
  IconActivity,
  IconClock,
  IconCpu,
  IconDatabase,
  IconFolder,
  IconHome,
  IconLayoutGrid,
  IconMenu2,
  IconSearch,
  IconServer,
  IconSettings,
  IconX,
} from "@tabler/icons-react";
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useDashboard } from "@/features/dashboard/use-dashboard";
import type { DashboardNode } from "@/features/dashboard/schema";
import { SettingsPage } from "@/features/settings/settings-page";
import { useTheme } from "@/features/settings/theme";

type Sample = { time: number; value: number };
type MetricName = "cpu" | "memory" | "disk" | "load";

const MetricTrend = lazy(() =>
  import("@/components/metric-trend").then((module) => ({ default: module.MetricTrend })),
);

declare global {
  interface Window {
    htmx?: { process: (element: Element) => void };
  }
}

const metricMeta: Record<MetricName, { label: string; color: string }> = {
  cpu: { label: "CPU usage", color: "var(--chart-1)" },
  memory: { label: "Memory", color: "var(--chart-2)" },
  disk: { label: "Disk usage", color: "var(--chart-3)" },
  load: { label: "Load average", color: "var(--chart-4)" },
};

const navItems = [
  { href: "#overview", label: "Overview", Icon: IconHome },
  { href: "#nodes", label: "Servers", Icon: IconServer },
  { href: "#service-groups", label: "Services", Icon: IconLayoutGrid },
];

function formatBytes(bytes: number) {
  return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
}

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function formatAge(seconds: number | null) {
  if (seconds === null || !Number.isFinite(seconds)) return "Not yet seen";
  if (seconds < 6) return "Just now";
  if (seconds < 60) return `${Math.floor(seconds)}s ago`;
  if (seconds < 3_600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3_600)}h ago`;
  return new Date(Date.now() - seconds * 1_000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function nodeAge(node: DashboardNode, now: number) {
  if (!node.stats || node.receivedAt === null) return null;
  return node.stats.collected_age_secs + (now - node.receivedAt) / 1_000;
}

function nodeStatus(node: DashboardNode, now: number) {
  const age = nodeAge(node, now);
  if (age === null) return node.scrape_failed ? ("unavailable" as const) : ("collecting" as const);
  return age <= Math.max(node.interval_secs * 3, 30) ? ("online" as const) : ("delayed" as const);
}

function statusLabel(status: ReturnType<typeof nodeStatus>) {
  if (status === "online") return "Reporting";
  if (status === "delayed") return "Delayed";
  if (status === "unavailable") return "Unavailable";
  return "Collecting";
}

function MetricCard({
  name,
  value,
  note,
  data,
  Icon,
}: {
  name: MetricName;
  value: string;
  note: string;
  data: Sample[];
  Icon: typeof IconCpu;
}) {
  return (
    <Card className="min-w-0 gap-4 border-border/70 bg-card/70 py-4 shadow-none">
      <CardHeader className="flex flex-row items-start justify-between gap-3 px-4 pb-0">
        <div className="flex min-w-0 items-center gap-2.5">
          <span className="grid size-8 shrink-0 place-items-center rounded-md bg-muted text-muted-foreground">
            <Icon aria-hidden="true" size={17} stroke={1.7} />
          </span>
          <CardTitle className="truncate text-sm font-medium text-muted-foreground">
            {metricMeta[name].label}
          </CardTitle>
        </div>
        {data.length > 1 ? (
          <Suspense fallback={<span className="h-10 w-28 shrink-0" aria-hidden="true" />}>
            <MetricTrend name={name} data={data} />
          </Suspense>
        ) : (
          <span className="h-10 w-28 shrink-0" aria-hidden="true" />
        )}
      </CardHeader>
      <CardContent className="px-4">
        <p className="text-2xl font-semibold tracking-tight tabular-nums">{value}</p>
        <p className="mt-1 truncate text-xs text-muted-foreground" title={note}>
          {note}
        </p>
      </CardContent>
    </Card>
  );
}

function Status({ status }: { status: ReturnType<typeof nodeStatus> }) {
  const styles = {
    online: "bg-emerald-400",
    delayed: "bg-amber-400",
    collecting: "bg-muted-foreground",
    unavailable: "bg-red-400",
  };
  return (
    <span className="inline-flex items-center gap-2 whitespace-nowrap text-xs text-muted-foreground">
      <span className={`size-1.5 rounded-full ${styles[status]}`} aria-hidden="true" />
      {statusLabel(status)}
    </span>
  );
}

function Dashboard() {
  const { nodes, groups, app, history, servicesHtml, serviceError, error } = useDashboard();
  const [selectedId, setSelectedId] = useState("");
  const [query, setQuery] = useState("");
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [serviceMatches, setServiceMatches] = useState(0);
  const servicesRef = useRef<HTMLDivElement>(null);

  useTheme(app.theme);

  useEffect(() => {
    document.title = `${app.dashboard_title} · Vesta`;
  }, [app.dashboard_title]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    document.body.classList.toggle("mobile-menu-open", mobileMenuOpen);
    return () => document.body.classList.remove("mobile-menu-open");
  }, [mobileMenuOpen]);

  useEffect(() => {
    const container = servicesRef.current;
    if (!container || !servicesHtml) return;
    const parsed = new DOMParser().parseFromString(servicesHtml, "text/html");
    container.replaceChildren(
      ...Array.from(parsed.body.childNodes, (node) => document.importNode(node, true)),
    );
    window.htmx?.process(container);
  }, [servicesHtml]);

  useEffect(() => {
    const container = servicesRef.current;
    if (!container) return;
    const normalized = query.trim().toLocaleLowerCase();
    const items = Array.from(container.querySelectorAll<HTMLElement>("[data-search-item]"));
    let count = 0;
    for (const item of items) {
      const text = (item.dataset.searchText ?? item.textContent ?? "").toLocaleLowerCase();
      item.hidden = Boolean(normalized) && !text.includes(normalized);
      if (!item.hidden) count += 1;
    }
    for (const group of container.querySelectorAll<HTMLElement>("[data-service-group]")) {
      const groupItems = Array.from(group.querySelectorAll<HTMLElement>("[data-search-item]"));
      group.hidden = groupItems.length > 0 && groupItems.every((item) => item.hidden);
    }
    setServiceMatches(count);
  }, [query, servicesHtml]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.key === "/" &&
        !event.metaKey &&
        !event.ctrlKey &&
        !event.altKey &&
        !["INPUT", "TEXTAREA", "SELECT"].includes(document.activeElement?.tagName ?? "")
      ) {
        event.preventDefault();
        document.getElementById("dashboard-search")?.focus();
      }
      if (event.key === "Escape") setMobileMenuOpen(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  const selectedNode = nodes.find((node) => node.id === selectedId) ?? nodes[0];
  const stats = selectedNode?.stats ?? null;
  const age = selectedNode ? nodeAge(selectedNode, now) : null;
  const selectedStatus = selectedNode ? nodeStatus(selectedNode, now) : "collecting";
  const searchText = query.trim().toLocaleLowerCase();
  const visibleNodes = useMemo(
    () =>
      nodes.filter(
        (node) => !searchText || `${node.name} ${node.id}`.toLocaleLowerCase().includes(searchText),
      ),
    [nodes, searchText],
  );
  const selectedHistory = selectedNode ? history[selectedNode.id] : undefined;
  const filteredCount = visibleNodes.length + serviceMatches;
  const bootedAt = stats
    ? new Date(Date.now() - stats.uptime_secs * 1_000).toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      })
    : "Waiting for first scrape";

  const metricCards: Array<{
    name: MetricName;
    value: string;
    note: string;
    Icon: typeof IconCpu;
  }> = [
    {
      name: "cpu",
      value: stats ? `${stats.cpu_usage_percent.toFixed(1)}%` : "—",
      note: selectedNode ? selectedNode.name : "No server configured",
      Icon: IconCpu,
    },
    {
      name: "memory",
      value: stats ? `${stats.memory.usage_percent.toFixed(1)}%` : "—",
      note: stats
        ? `${formatBytes(stats.memory.used_bytes)} / ${formatBytes(stats.memory.total_bytes)} used`
        : "Waiting for first scrape",
      Icon: IconDatabase,
    },
    {
      name: "disk",
      value: stats ? `${stats.disk.usage_percent.toFixed(1)}%` : "—",
      note: stats
        ? `${formatBytes(stats.disk.used_bytes)} / ${formatBytes(stats.disk.total_bytes)} · ${stats.disk.mount}`
        : "Waiting for first scrape",
      Icon: IconServer,
    },
    {
      name: "load",
      value: stats?.load ? stats.load.one.toFixed(2) : "—",
      note: stats?.load ? "1-minute load average" : "Waiting for first scrape",
      Icon: IconActivity,
    },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      {mobileMenuOpen && (
        <button
          className="mobile-menu-overlay md:hidden"
          type="button"
          aria-label="Close navigation menu"
          onClick={() => setMobileMenuOpen(false)}
        />
      )}
      <aside
        id="sidebar"
        className={`app-sidebar fixed inset-y-0 left-0 z-30 flex w-[4.5rem] flex-col items-center border-r border-border/70 bg-background px-2.5 py-4 transition-transform md:translate-x-0 ${mobileMenuOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"}`}
        aria-label="Main navigation"
      >
        <a
          href="#overview"
          className="mb-8 grid size-10 place-items-center rounded-lg text-foreground transition-colors hover:bg-muted"
          aria-label="Vesta overview"
          onClick={() => setMobileMenuOpen(false)}
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
              onClick={() => setMobileMenuOpen(false)}
            >
              <Icon size={19} stroke={1.7} aria-hidden="true" />
            </a>
          ))}
          {groups.length > 0 && <div className="my-2 h-px w-7 bg-border" aria-hidden="true" />}
          {groups.map((group) => (
            <a
              key={group.id}
              href={`#group-${encodeURIComponent(group.id)}`}
              className="grid size-9 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
              aria-label={group.name}
              title={group.name}
              onClick={() => setMobileMenuOpen(false)}
            >
              <IconFolder size={17} stroke={1.7} aria-hidden="true" />
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

      <div className="md:pl-[4.5rem]">
        <header className="sticky top-0 z-20 flex min-h-16 items-center gap-3 border-b border-border/70 bg-background/90 px-4 backdrop-blur-xl sm:px-6 lg:px-8">
          <button
            className="grid size-9 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring md:hidden"
            type="button"
            aria-label={mobileMenuOpen ? "Close navigation" : "Open navigation"}
            aria-controls="sidebar"
            aria-expanded={mobileMenuOpen}
            onClick={() => setMobileMenuOpen((open) => !open)}
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
              Search servers and services
            </label>
            <Input
              id="dashboard-search"
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search servers and services…"
              className="h-10 border-border/70 bg-muted/40 pl-9 pr-12 text-sm shadow-none placeholder:text-muted-foreground/80 focus-visible:ring-1"
            />
            <kbd className="pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 rounded border border-border bg-background px-1.5 py-0.5 font-mono text-[0.65rem] text-muted-foreground">
              /
            </kbd>
          </div>
        </header>

        <main
          id="main-content"
          className="mx-auto w-full max-w-[1600px] space-y-7 p-4 sm:p-6 lg:space-y-8 lg:p-8"
        >
          <section id="overview" aria-labelledby="page-title" className="space-y-5">
            <div className="flex flex-wrap items-end justify-between gap-4">
              <div>
                <h1
                  id="page-title"
                  className="text-2xl font-semibold tracking-tight sm:text-[1.75rem]"
                >
                  {app.dashboard_title}
                </h1>
                <p className="mt-1 text-sm text-muted-foreground">
                  A live view of your servers and self-hosted services.
                </p>
              </div>
              {nodes.length > 0 && (
                <label className="flex items-center gap-2 text-xs text-muted-foreground">
                  <span>Overview for</span>
                  <select
                    className="h-9 max-w-52 rounded-md border border-input bg-card px-3 text-sm text-foreground shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    value={selectedNode?.id ?? ""}
                    aria-label="Choose server for metric cards"
                    onChange={(event) => setSelectedId(event.target.value)}
                  >
                    {nodes.map((node) => (
                      <option key={node.id} value={node.id}>
                        {node.name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </div>

            {error && (
              <div
                className="rounded-lg border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive"
                role="alert"
              >
                {error}. Refresh the page to try again.
              </div>
            )}

            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
              {metricCards.map((metric) => (
                <MetricCard
                  key={metric.name}
                  name={metric.name}
                  value={metric.value}
                  note={metric.note}
                  data={selectedHistory?.[metric.name] ?? []}
                  Icon={metric.Icon}
                />
              ))}
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <Card className="gap-4 border-border/70 bg-card/50 py-4 shadow-none">
                <CardHeader className="flex flex-row items-center justify-between px-4 pb-0">
                  <div className="flex items-center gap-2.5">
                    <IconClock
                      size={17}
                      stroke={1.7}
                      className="text-muted-foreground"
                      aria-hidden="true"
                    />
                    <CardTitle className="text-sm font-medium">Uptime</CardTitle>
                  </div>
                  <span className="max-w-[50%] truncate text-xs text-muted-foreground">
                    {selectedNode?.name ?? "No server selected"}
                  </span>
                </CardHeader>
                <CardContent className="px-4">
                  <p className="text-xl font-semibold tabular-nums">
                    {stats ? formatUptime(stats.uptime_secs) : "—"}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">Booted {bootedAt}</p>
                </CardContent>
              </Card>
              <Card className="gap-4 border-border/70 bg-card/50 py-4 shadow-none">
                <CardHeader className="flex flex-row items-center justify-between px-4 pb-0">
                  <div className="flex items-center gap-2.5">
                    <IconActivity
                      size={17}
                      stroke={1.7}
                      className="text-muted-foreground"
                      aria-hidden="true"
                    />
                    <CardTitle className="text-sm font-medium">Data collection</CardTitle>
                  </div>
                  <Status status={selectedStatus} />
                </CardHeader>
                <CardContent className="grid grid-cols-1 gap-3 px-4 sm:grid-cols-3">
                  <div className="min-w-0">
                    <p className="text-[0.68rem] uppercase tracking-wide text-muted-foreground">
                      Source
                    </p>
                    <p className="mt-1 truncate text-xs" title={selectedNode?.source}>
                      {selectedNode?.source ?? "No source configured"}
                    </p>
                  </div>
                  <div>
                    <p className="text-[0.68rem] uppercase tracking-wide text-muted-foreground">
                      Scrape interval
                    </p>
                    <p className="mt-1 text-xs tabular-nums">
                      {selectedNode ? `${selectedNode.interval_secs} seconds` : "—"}
                    </p>
                  </div>
                  <div>
                    <p className="text-[0.68rem] uppercase tracking-wide text-muted-foreground">
                      Last successful scrape
                    </p>
                    <p className="mt-1 text-xs tabular-nums">{formatAge(age)}</p>
                  </div>
                </CardContent>
              </Card>
            </div>
          </section>

          <section id="nodes" aria-labelledby="nodes-heading" className="space-y-3">
            <div className="flex items-center justify-between gap-3">
              <h2 id="nodes-heading" className="text-lg font-semibold tracking-tight">
                Servers
              </h2>
              {searchText && (
                <span className="text-xs text-muted-foreground">
                  {filteredCount} matching results
                </span>
              )}
            </div>
            <Card className="gap-0 overflow-hidden border-border/70 bg-card/50 py-0 shadow-none">
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="border-border/70 hover:bg-transparent">
                      <TableHead className="min-w-44 pl-4">Server</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Uptime</TableHead>
                      <TableHead>CPU</TableHead>
                      <TableHead>Memory</TableHead>
                      <TableHead>Disk</TableHead>
                      <TableHead>Load</TableHead>
                      <TableHead className="pr-4">Last seen</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {visibleNodes.map((node) => {
                      const nodeStats = node.stats;
                      const nodeSeen = nodeAge(node, now);
                      const status = nodeStatus(node, now);
                      return (
                        <TableRow
                          key={node.id}
                          className={`border-border/60 ${node.id === selectedNode?.id ? "bg-accent/30" : ""}`}
                        >
                          <TableCell className="pl-4">
                            <button
                              type="button"
                              className="group flex min-h-10 items-center gap-2.5 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                              aria-label={`Show ${node.name} in the overview`}
                              onClick={() => setSelectedId(node.id)}
                            >
                              {node.icon ? (
                                <img
                                  className="size-8 shrink-0 rounded-md border border-border bg-muted object-contain p-1"
                                  src={node.icon}
                                  alt=""
                                  loading="lazy"
                                  decoding="async"
                                />
                              ) : (
                                <span className="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-muted text-muted-foreground">
                                  <IconServer size={17} stroke={1.7} aria-hidden="true" />
                                </span>
                              )}
                              <span className="grid">
                                <span className="font-medium group-hover:text-primary">
                                  {node.name}
                                </span>
                                <span className="text-[0.68rem] text-muted-foreground">
                                  {node.hostname
                                    ? `Hostname · ${node.hostname}`
                                    : `ID · ${node.id}`}
                                </span>
                                {node.device_name && (
                                  <span className="text-[0.68rem] text-muted-foreground">
                                    Device · {node.device_name}
                                  </span>
                                )}
                              </span>
                            </button>
                          </TableCell>
                          <TableCell>
                            <Status status={status} />
                          </TableCell>
                          <TableCell className="tabular-nums">
                            {nodeStats ? formatUptime(nodeStats.uptime_secs) : "—"}
                          </TableCell>
                          <TableCell className="tabular-nums">
                            {nodeStats ? `${nodeStats.cpu_usage_percent.toFixed(1)}%` : "—"}
                          </TableCell>
                          <TableCell className="tabular-nums">
                            {nodeStats ? `${nodeStats.memory.usage_percent.toFixed(1)}%` : "—"}
                          </TableCell>
                          <TableCell className="tabular-nums">
                            {nodeStats ? `${nodeStats.disk.usage_percent.toFixed(1)}%` : "—"}
                          </TableCell>
                          <TableCell className="tabular-nums">
                            {nodeStats?.load ? nodeStats.load.one.toFixed(2) : "—"}
                          </TableCell>
                          <TableCell className="pr-4 text-muted-foreground tabular-nums">
                            {formatAge(nodeSeen)}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                    {visibleNodes.length === 0 && (
                      <TableRow>
                        <TableCell
                          colSpan={8}
                          className="h-32 text-center text-sm text-muted-foreground"
                        >
                          {nodes.length === 0
                            ? "No servers configured. Add a server in Settings to start monitoring."
                            : "No servers match your search."}
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </div>
            </Card>
          </section>

          <div ref={servicesRef} aria-live="polite">
            {serviceError && (
              <p className="text-sm text-destructive" role="alert">
                {serviceError}
              </p>
            )}
            {!servicesHtml && !serviceError && (
              <div className="space-y-3" aria-label="Loading configured services">
                <h2 className="text-lg font-semibold tracking-tight">Services</h2>
                <p className="text-sm text-muted-foreground">Loading configured services…</p>
              </div>
            )}
          </div>
          <p className="screen-reader-only" role="status" aria-live="polite">
            {searchText ? `${filteredCount} matching results` : ""}
          </p>
        </main>
      </div>
    </div>
  );
}

function App() {
  return window.location.pathname === "/settings" ? <SettingsPage /> : <Dashboard />;
}

export { App };
