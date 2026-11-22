import * as v from "valibot";
import { ProwlarrPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";

export function ProwlarrTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(ProwlarrPayloadSchema, data);
  const indexers = parsed.success ? parsed.output.indexers : [];
  const totals = parsed.success
    ? parsed.output.totals
    : { queries: 0, grabs: 0, average_response_time: null };
  const totalResponseTime = totals.average_response_time ?? null;

  return indexers.length === 0 ? (
    <div className="grid min-h-full place-items-center text-xl font-bold">No indexers</div>
  ) : (
    <div className="flex h-full flex-col">
      <div className="flex flex-col justify-around">
        {indexers.map((indexer) => (
          <div key={indexer.name} className="flex items-center justify-between gap-2 py-0.5">
            <span className="flex min-w-0 items-center gap-1.5">
              <span
                className={`inline-block h-2 w-2 shrink-0 rounded-full ${
                  indexer.failed_queries > 0
                    ? "bg-red-500"
                    : indexer.enable
                      ? "bg-emerald-500"
                      : "bg-zinc-500"
                }`}
              />
              <span
                className={`truncate ${indexer.failed_queries > 0 ? "" : indexer.enable ? "" : "text-zinc-500"}`}
              >
                {indexer.name}
              </span>
            </span>
            <span
              className={`widget-secondary shrink-0 whitespace-nowrap tabular-nums ${
                indexer.failed_queries > 0 ? "text-red-500" : ""
              }`}
            >
              {indexer.failed_queries > 0
                ? `${indexer.queries}q · ${indexer.failed_queries} failed`
                : `${indexer.queries}q · ${indexer.grabs}g`}
            </span>
          </div>
        ))}
      </div>
      <div className="widget-divider mt-auto flex items-center justify-between gap-2 pt-1">
        <span className="truncate">
          {totals.queries}q · {totals.grabs}g
        </span>
        <span className="widget-secondary shrink-0 whitespace-nowrap">
          {totalResponseTime !== null && totalResponseTime !== undefined
            ? `${Math.round(totalResponseTime)} ms`
            : "—"}
        </span>
      </div>
    </div>
  );
}
