import { IconActivity, IconClock, IconCpu, IconDatabase, IconServer } from "@tabler/icons-react";
import { lazy, Suspense } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { DashboardNode } from "@/features/dashboard/schema";
import {
  formatAge,
  formatBytes,
  formatUptime,
  nodeAge,
  nodeStatus,
} from "@/features/servers/node-health";
import { Status } from "@/features/servers/status";

type Sample = { time: number; value: number };
type MetricName = "cpu" | "memory" | "disk" | "load";

const MetricTrend = lazy(() =>
  import("@/components/metric-trend").then((module) => ({ default: module.MetricTrend })),
);

const metricMeta: Record<MetricName, { label: string; color: string }> = {
  cpu: { label: "CPU usage", color: "var(--chart-1)" },
  memory: { label: "Memory", color: "var(--chart-2)" },
  disk: { label: "Disk usage", color: "var(--chart-3)" },
  load: { label: "Load average", color: "var(--chart-4)" },
};

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

type MetricCardData = {
  name: MetricName;
  value: string;
  note: string;
  Icon: typeof IconCpu;
};

/** Metric card content for the featured server's latest scrape. */
function buildMetricCards(
  stats: DashboardNode["stats"],
  selectedNode: DashboardNode | undefined,
): MetricCardData[] {
  return [
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
}

/** Overview section: title, server picker, metric cards, uptime and collection health. */
function OverviewSection({
  title,
  error,
  nodes,
  selectedNode,
  selectedHistory,
  now,
  onSelectNode,
}: {
  title: string;
  error: string;
  nodes: DashboardNode[];
  selectedNode: DashboardNode | undefined;
  selectedHistory: Partial<Record<MetricName, Sample[]>> | undefined;
  now: number;
  onSelectNode: (id: string) => void;
}) {
  const stats = selectedNode?.stats ?? null;
  const age = selectedNode ? nodeAge(selectedNode, now) : null;
  const selectedStatus = selectedNode ? nodeStatus(selectedNode, now) : "collecting";
  const bootedAt = stats
    ? new Date(Date.now() - stats.uptime_secs * 1_000).toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      })
    : "Waiting for first scrape";

  const metricCards = buildMetricCards(stats, selectedNode);

  return (
    <section id="overview" aria-labelledby="page-title" className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 id="page-title" className="text-2xl font-semibold tracking-tight sm:text-[1.75rem]">
            {title}
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
              onChange={(event) => onSelectNode(event.target.value)}
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
              <p className="text-[0.68rem] uppercase tracking-wide text-muted-foreground">Source</p>
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
  );
}

export { OverviewSection };
export type { MetricName, Sample };
