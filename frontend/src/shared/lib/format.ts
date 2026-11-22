export function formatBytes(bytes: number, unit: "MiB" | "GiB" | "GB" = "GB"): string {
  const divisor = unit === "MiB" ? 1_048_576 : 1_073_741_824;
  if (unit === "GB") return `${(bytes / divisor).toFixed(1)} GB`;
  return `${(bytes / divisor).toFixed(0)}${unit === "MiB" ? "M" : "G"}`;
}

export function formatUptime(seconds: number, style: "compact" | "full" = "compact"): string {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (style === "full") {
    return days > 0 ? `${days}d ${hours}h ${minutes}m` : `${hours}h ${minutes}m`;
  }
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

export function percentDisplay(percent: number): string {
  const rounded = Math.round(percent * 10) / 10;
  return `${Number.isInteger(rounded) ? rounded : rounded.toFixed(1)}%`;
}
