import * as v from "valibot";

const LoadSchema = v.object({
  one: v.number(),
  five: v.number(),
  fifteen: v.number(),
});

const StatsSchema = v.object({
  cpu_usage_percent: v.number(),
  memory: v.object({
    used_bytes: v.number(),
    total_bytes: v.number(),
    usage_percent: v.number(),
  }),
  disk: v.object({
    mount: v.string(),
    used_bytes: v.number(),
    total_bytes: v.number(),
    usage_percent: v.number(),
  }),
  load: v.nullable(LoadSchema),
  uptime_secs: v.number(),
  collected_age_secs: v.number(),
});

const NodeSchema = v.object({
  id: v.string(),
  name: v.string(),
  hostname: v.nullable(v.string()),
  device_name: v.nullable(v.string()),
  icon: v.nullable(v.string()),
  source: v.string(),
  interval_secs: v.number(),
  scrape_failed: v.boolean(),
  stats: v.nullable(StatsSchema),
});

const GroupSchema = v.object({ id: v.string(), name: v.string() });
const AppSettingsSchema = v.object({
  dashboard_title: v.string(),
  theme: v.picklist(["system", "light", "dark"]),
});
const DashboardDataSchema = v.object({
  nodes: v.array(NodeSchema),
  groups: v.array(GroupSchema),
  app: AppSettingsSchema,
});
const DashboardResponseSchema = v.object({
  status: v.literal("success"),
  data: DashboardDataSchema,
});
const NodeEventSchema = v.object({
  id: v.string(),
  stats: v.nullable(StatsSchema),
  scrape_failed: v.boolean(),
});

type Stats = v.InferOutput<typeof StatsSchema>;
type DashboardNode = v.InferOutput<typeof NodeSchema> & { receivedAt: number | null };
type DashboardGroup = v.InferOutput<typeof GroupSchema>;
type DashboardData = v.InferOutput<typeof DashboardDataSchema>;
type DashboardAppSettings = v.InferOutput<typeof AppSettingsSchema>;
type NodeEvent = v.InferOutput<typeof NodeEventSchema>;

async function fetchDashboard(): Promise<DashboardData> {
  const response = await fetch("/api/dashboard", {
    headers: { Accept: "application/json" },
    cache: "no-store",
  });
  if (!response.ok) throw new Error(`Dashboard request failed (${response.status})`);
  const parsed = v.safeParse(DashboardResponseSchema, await response.json());
  if (!parsed.success) throw new Error("The server returned invalid dashboard data");
  return parsed.output.data;
}

function parseEvent<TSchema extends v.GenericSchema>(schema: TSchema, raw: string) {
  try {
    const parsed = v.safeParse(schema, JSON.parse(raw));
    return parsed.success ? parsed.output : null;
  } catch {
    return null;
  }
}

export { DashboardDataSchema, NodeEventSchema, fetchDashboard, parseEvent };
export type {
  DashboardAppSettings,
  DashboardData,
  DashboardGroup,
  DashboardNode,
  NodeEvent,
  Stats,
};
