import * as v from "valibot";
import { StatRow, WidgetError } from "@/features/widgets/builtins/shared";
import { SystemStatsPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";
import { formatBytes, formatUptime } from "@/shared/lib/format";

export function SystemStatsTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(SystemStatsPayloadSchema, data);
  if (!parsed.success) return <WidgetError message="Invalid server statistics payload." />;
  const { name, stats } = parsed.output;

  return (
    <div className="system-stats-widget">
      <div className="system-stats-heading">
        <span className="widget-title">{name ?? "Server"}</span>
        <div className="widget-secondary">
          Load {stats.load ? stats.load.one.toFixed(2) : "n/a"}
        </div>
      </div>
      <StatRow
        label="CPU"
        percent={stats.cpu_usage_percent}
        value={`${stats.cpu_usage_percent.toFixed(1)}%`}
      />
      <StatRow
        label="Memory"
        percent={stats.memory.usage_percent}
        value={`${stats.memory.usage_percent.toFixed(1)}% · ${formatBytes(stats.memory.used_bytes, "MiB")} / ${formatBytes(stats.memory.total_bytes, "MiB")}`}
      />
      <StatRow
        label={`Disk (${stats.disk.mount})`}
        percent={stats.disk.usage_percent}
        value={`${stats.disk.usage_percent.toFixed(1)}% · ${formatBytes(stats.disk.used_bytes, "GiB")} / ${formatBytes(stats.disk.total_bytes, "GiB")}`}
      />
      <div className="system-stats-uptime widget-divider">
        <span className="widget-secondary">Uptime</span>
        <span className="widget-value">{formatUptime(stats.uptime_secs, "full")}</span>
      </div>
    </div>
  );
}
