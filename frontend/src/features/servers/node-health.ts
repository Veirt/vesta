import type { DashboardNode } from "@/features/dashboard/schema";

export function formatBytes(bytes: number) {
  return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
}

export function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

export function formatAge(seconds: number | null) {
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

export function nodeAge(node: DashboardNode, now: number) {
  if (!node.stats || node.receivedAt === null) return null;
  return node.stats.collected_age_secs + (now - node.receivedAt) / 1_000;
}

export function nodeStatus(node: DashboardNode, now: number) {
  const age = nodeAge(node, now);
  if (age === null) return node.scrape_failed ? ("unavailable" as const) : ("collecting" as const);
  return age <= Math.max(node.interval_secs * 3, 30) ? ("online" as const) : ("delayed" as const);
}

export function statusLabel(status: ReturnType<typeof nodeStatus>) {
  if (status === "online") return "Reporting";
  if (status === "delayed") return "Delayed";
  if (status === "unavailable") return "Unavailable";
  return "Collecting";
}
