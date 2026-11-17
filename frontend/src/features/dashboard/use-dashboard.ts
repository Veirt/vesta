import { useEffect, useState } from "react";
import {
  DashboardDataSchema,
  fetchDashboard,
  NodeEventSchema,
  parseEvent,
  type DashboardData,
  type DashboardAppSettings,
  type DashboardGroup,
  type DashboardItem,
  type DashboardNode,
  type Stats,
} from "@/features/dashboard/schema";

type Sample = { time: number; value: number };
type MetricName = "cpu" | "memory" | "disk" | "load";
type History = Record<string, Partial<Record<MetricName, Sample[]>>>;
type Connection = "connecting" | "live" | "reconnecting";

function sampleValues(stats: Stats): Partial<Record<MetricName, number>> {
  return {
    cpu: stats.cpu_usage_percent,
    memory: stats.memory.usage_percent,
    disk: stats.disk.usage_percent,
    load: stats.load?.one,
  };
}

function addSamples(history: History, id: string, stats: Stats, time: number): History {
  const values = sampleValues(stats);
  const next = { ...history, [id]: { ...history[id] } };
  for (const metric of ["cpu", "memory", "disk", "load"] as const) {
    const value = values[metric];
    if (value === undefined || !Number.isFinite(value)) continue;
    const series = [...(next[id][metric] ?? []), { time, value }];
    next[id][metric] = series.slice(-48);
  }
  return next;
}

function useDashboard() {
  const [nodes, setNodes] = useState<DashboardNode[]>([]);
  const [items, setItems] = useState<DashboardItem[]>([]);
  const [groups, setGroups] = useState<DashboardGroup[]>([]);
  const [app, setApp] = useState<DashboardAppSettings>({
    dashboard_title: "Server Monitor",
    theme: "system",
  });
  const [history, setHistory] = useState<History>({});
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [connection, setConnection] = useState<Connection>("connecting");

  useEffect(() => {
    let active = true;
    let source: EventSource | undefined;
    // boot()'s fetch may resolve after an early unmount, so connect() has to
    // hand its stream to the already-run cleanup via this flag: without it a
    // late EventSource leaks.
    let closed = false;
    const close = () => {
      active = false;
      closed = true;
      source?.close();
    };
    const applySnapshot = (data: DashboardData) => {
      if (!active) return;
      const receivedAt = Date.now();
      const mapped = data.nodes.map((node) => ({
        ...node,
        receivedAt: node.stats ? receivedAt : null,
      }));
      setNodes(mapped);
      setItems(data.items);
      setGroups(data.groups);
      setApp(data.app);
      setHistory((current) => {
        const retained: History = {};
        for (const node of mapped) {
          if (current[node.id]) retained[node.id] = current[node.id];
          else if (node.stats)
            retained[node.id] = addSamples({}, node.id, node.stats, receivedAt)[node.id];
        }
        return retained;
      });
    };

    const connect = () => {
      if (closed) return;
      source = new EventSource("/api/nodes/events");
      source.onopen = () => active && setConnection("live");
      source.onerror = () => active && setConnection("reconnecting");
      source.addEventListener("snapshot", (event) => {
        const parsed = parseEvent(DashboardDataSchema, (event as MessageEvent<string>).data);
        if (parsed) {
          applySnapshot(parsed);
        } else console.error("Ignoring invalid dashboard snapshot from event stream");
      });
      source.addEventListener("node", (event) => {
        const parsed = parseEvent(NodeEventSchema, (event as MessageEvent<string>).data);
        if (!parsed) {
          console.error("Ignoring invalid node update from event stream");
          return;
        }
        const receivedAt = Date.now();
        const { id, stats, scrape_failed: scrapeFailed } = parsed;
        setNodes((current) =>
          current.map((node) =>
            node.id === id
              ? {
                  ...node,
                  stats,
                  receivedAt: stats ? receivedAt : null,
                  scrape_failed: scrapeFailed,
                }
              : node,
          ),
        );
        if (stats) setHistory((current) => addSamples(current, id, stats, receivedAt));
      });
    };

    const boot = async () => {
      try {
        const data = await fetchDashboard();
        if (!active) return;
        applySnapshot(data);
        setError("");
        setLoading(false);
        connect();
      } catch (cause) {
        if (active) {
          setError(cause instanceof Error ? cause.message : "Dashboard data could not be loaded");
          setLoading(false);
        }
        return;
      }
    };

    void boot();
    return close;
  }, []);

  return { nodes, items, groups, app, history, error, loading, connection };
}

export { useDashboard };
