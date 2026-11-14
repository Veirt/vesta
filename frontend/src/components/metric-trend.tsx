import { Area, AreaChart } from "recharts";
import { ChartContainer, type ChartConfig } from "@/components/ui/chart";

type MetricName = "cpu" | "memory" | "disk" | "load";
type Sample = { time: number; value: number };

const metricMeta: Record<MetricName, { label: string; color: string }> = {
  cpu: { label: "CPU usage", color: "var(--chart-1)" },
  memory: { label: "Memory", color: "var(--chart-2)" },
  disk: { label: "Disk usage", color: "var(--chart-3)" },
  load: { label: "Load average", color: "var(--chart-4)" },
};

function MetricTrend({ name, data }: { name: MetricName; data: Sample[] }) {
  const config: ChartConfig = {
    value: { label: metricMeta[name].label, color: metricMeta[name].color },
  };
  return (
    <ChartContainer
      config={config}
      initialDimension={{ width: 112, height: 42 }}
      className="h-10 w-28 shrink-0 [aspect-ratio:auto]"
      role="img"
      aria-label={`${metricMeta[name].label} recent scrape samples`}
    >
      <AreaChart data={data} margin={{ top: 4, right: 1, left: 1, bottom: 1 }}>
        <defs>
          <linearGradient id={`metric-fill-${name}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-value)" stopOpacity={0.3} />
            <stop offset="100%" stopColor="var(--color-value)" stopOpacity={0} />
          </linearGradient>
        </defs>
        <Area
          dataKey="value"
          stroke="var(--color-value)"
          strokeWidth={1.75}
          fill={`url(#metric-fill-${name})`}
          dot={false}
          activeDot={false}
          isAnimationActive={false}
        />
      </AreaChart>
    </ChartContainer>
  );
}

export { MetricTrend };
