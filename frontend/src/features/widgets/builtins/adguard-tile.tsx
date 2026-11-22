import * as v from "valibot";
import { AdGuardPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";
import { percentDisplay } from "@/shared/lib/format";

export function AdGuardTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(AdGuardPayloadSchema, data);
  const payload = parsed.success
    ? parsed.output
    : {
        num_dns_queries: 0,
        num_blocked_filtering: 0,
        blocked_percent: 0,
        avg_processing_time: null,
        rules_count: 0,
        protection_enabled: false,
        top_blocked_domains: [],
      };
  const topDomains = payload.top_blocked_domains.slice(0, 3);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-baseline gap-2">
        <span className="text-2xl font-bold tabular-nums">
          {payload.num_dns_queries.toLocaleString()}
        </span>
        <span className="widget-secondary">DNS queries</span>
      </div>
      <div className="tabular-nums">
        {payload.num_blocked_filtering.toLocaleString()} blocked ·{" "}
        {percentDisplay(payload.blocked_percent)}
      </div>
      {topDomains.length > 0 ? (
        <div className="flex flex-col justify-around">
          {topDomains.map((row) => (
            <div key={row.domain} className="flex items-center justify-between gap-2 py-0.5">
              <span className="truncate">{row.domain}</span>
              <span className="widget-secondary shrink-0 whitespace-nowrap tabular-nums">
                {row.count.toLocaleString()}
              </span>
            </div>
          ))}
        </div>
      ) : null}
      <div className="widget-divider mt-auto flex items-center justify-between gap-2 pt-1">
        <span
          className={`truncate ${
            payload.protection_enabled ? "text-emerald-600 dark:text-emerald-400" : "text-red-500"
          }`}
        >
          {payload.protection_enabled ? "Protected" : "Disabled"}
        </span>
        <span className="widget-secondary shrink-0 whitespace-nowrap tabular-nums">
          {payload.avg_processing_time !== null && payload.avg_processing_time !== undefined
            ? `${Math.round(payload.avg_processing_time * 1000)} ms`
            : "—"}{" "}
          · {payload.rules_count} rules
        </span>
      </div>
    </div>
  );
}
