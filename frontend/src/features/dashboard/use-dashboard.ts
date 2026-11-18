import { useEffect, useState } from "react";
import { fetchDashboard } from "@/shared/api/client";
import { parseEvent } from "@/shared/schemas/api";
import {
  DashboardDataSchema,
  NodeEventSchema,
  type DashboardData,
  type DashboardAppSettings,
  type DashboardGroup,
  type DashboardItem,
  type DashboardNode,
  type Stats,
} from "@/shared/schemas/core";

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
  const previous = history[id] ?? {};
  const sampled: Partial<Record<MetricName, Sample[]>> = {};
  for (const metric of ["cpu", "memory", "disk", "load"] as const) {
    const value = values[metric];
    if (value === undefined || !Number.isFinite(value)) continue;
    sampled[metric] = extendSeries(previous[metric] ?? [], value, time);
  }
  return { ...history, [id]: { ...previous, ...sampled } };
}

// Pure: builds and returns a new series instead of mutating the old one, so
// it is safe to call from a React state updater.
function extendSeries(series: Sample[], value: number, time: number): Sample[] {
  return [...series, { time, value }].slice(-48);
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
    const controller = new AbortController();
    let closed = false;
    let streamSnapshotApplied = false;
    const applySnapshot = (data: DashboardData) => {
      if (closed) return;
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

    const onOpen = () => {
      if (!closed) setConnection("live");
    };
    const onError = () => {
      if (!closed) setConnection("reconnecting");
    };
    const onSnapshot = (event: MessageEvent<string>) => {
      const parsed = parseEvent(DashboardDataSchema, event.data);
      if (parsed) {
        streamSnapshotApplied = true;
        applySnapshot(parsed);
        setError("");
        setLoading(false);
      } else console.error("Ignoring invalid dashboard snapshot from event stream");
    };
    const onNode = (event: MessageEvent<string>) => {
      const parsed = parseEvent(NodeEventSchema, event.data);
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
    };

    const source = new EventSource("/api/nodes/events");
    source.onopen = onOpen;
    source.onerror = onError;
    source.addEventListener("snapshot", onSnapshot);
    source.addEventListener("node", onNode);

    const boot = async () => {
      try {
        const data = await fetchDashboard(controller.signal);
        if (!data || closed || controller.signal.aborted || streamSnapshotApplied) return;
        applySnapshot(data);
        setError("");
        setLoading(false);
      } catch (cause) {
        if (!closed && !controller.signal.aborted && !streamSnapshotApplied) {
          setError(cause instanceof Error ? cause.message : "Dashboard data could not be loaded");
          setLoading(false);
        }
        return;
      }
    };

    void boot();
    return () => {
      closed = true;
      controller.abort();
      source.removeEventListener("snapshot", onSnapshot);
      source.removeEventListener("node", onNode);
      source.close();
    };
  }, []);

  return { nodes, items, groups, app, history, error, loading, connection };
}

export { useDashboard };
