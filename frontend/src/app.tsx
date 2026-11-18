import { useEffect, useMemo, useState } from "react";
import { AppSidebar } from "@/features/dashboard/dashboard-sidebar";
import { DashboardItems } from "@/features/dashboard/dashboard-items";
import { NodesTable } from "@/features/servers/nodes-table";
import { OverviewSection } from "@/features/dashboard/overview-section";
import { useDashboard } from "@/features/dashboard/use-dashboard";
import { DashboardHeader } from "@/features/dashboard/dashboard-header";
import { SettingsPage } from "@/features/settings/settings-page";
import { useTheme } from "@/shared/lib/theme";

function Dashboard() {
  const { nodes, items, groups, app, history, error, loading } = useDashboard();
  const [selectedId, setSelectedId] = useState("");
  const [query, setQuery] = useState("");
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [now, setNow] = useState(Date.now());

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
  const searchText = query.trim().toLocaleLowerCase();
  const itemMatches = items.filter(
    (item) =>
      !searchText ||
      `${item.id} ${item.title} ${item.kind === "widget" ? item.name : (item.href ?? "")}`
        .toLocaleLowerCase()
        .includes(searchText),
  ).length;
  const visibleNodes = useMemo(
    () =>
      nodes.filter(
        (node) => !searchText || `${node.name} ${node.id}`.toLocaleLowerCase().includes(searchText),
      ),
    [nodes, searchText],
  );
  const filteredCount = visibleNodes.length + itemMatches;

  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppSidebar mobileMenuOpen={mobileMenuOpen} onClose={() => setMobileMenuOpen(false)} />
      <div className="md:pl-[4.5rem]">
        <DashboardHeader
          mobileMenuOpen={mobileMenuOpen}
          query={query}
          onQueryChange={setQuery}
          onToggleMenu={() => setMobileMenuOpen((open) => !open)}
        />
        <main
          id="main-content"
          className="mx-auto w-full max-w-[1600px] space-y-7 p-4 sm:p-6 lg:p-8"
        >
          <OverviewSection
            title={app.dashboard_title}
            error={error}
            nodes={nodes}
            selectedNode={selectedNode}
            selectedHistory={selectedNode ? history[selectedNode.id] : undefined}
            now={now}
            onSelectNode={setSelectedId}
          />
          <NodesTable
            nodes={nodes}
            visibleNodes={visibleNodes}
            selectedNodeId={selectedNode?.id ?? ""}
            now={now}
            searchText={searchText}
            filteredCount={filteredCount}
            onSelectNode={setSelectedId}
          />
          <DashboardItems items={items} groups={groups} query={searchText} loading={loading} />
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
