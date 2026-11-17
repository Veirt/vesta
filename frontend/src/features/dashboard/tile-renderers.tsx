import { IconPhotoOff } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, type CSSProperties, type ReactNode, useState } from "react";
import type { DashboardService, DashboardWidget } from "@/features/dashboard/schema";

type WidgetData = Record<string, unknown>;

function layoutStyle(item: DashboardService | DashboardWidget): CSSProperties {
  const width = Math.max(1, Math.min(12, item.width ?? 4));
  return {
    "--tile-x": Math.min(item.x ?? 0, 12 - width),
    "--tile-y": Math.min(item.y ?? 0, 10_000),
    "--tile-width": width,
    "--tile-height": Math.max(1, Math.min(12, item.height ?? (item.kind === "widget" ? 2 : 1))),
  } as CSSProperties;
}

function useWidgetData(widgetName: string, itemId: string, refreshInterval: number | null) {
  const [data, setData] = useState<WidgetData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    const params = new URLSearchParams({ id: itemId });
    // Resolves the validated widget payload; all state updates happen in the
    // guarded .then/.catch handlers below so stale runs never write state.
    const loadWidget = async (): Promise<WidgetData> => {
      const response = await fetch(
        `/api/widgets/${encodeURIComponent(widgetName)}?${params.toString()}`,
        { cache: "no-store", signal: controller.signal },
      );
      const result: unknown = await response.json();
      // Non-success responses carry a jsend "error" body whose message
      // names what went wrong; prefer it over a bare status code.
      const asMessage = (candidate: unknown): string | null =>
        typeof candidate === "object" &&
        candidate !== null &&
        typeof (candidate as { message?: unknown }).message === "string"
          ? ((candidate as { message: string }).message as string)
          : null;
      if (!response.ok) {
        throw new Error(asMessage(result) ?? `Widget request failed (${response.status})`);
      }
      if (
        !result ||
        typeof result !== "object" ||
        !("status" in result) ||
        result.status !== "success" ||
        !("data" in result) ||
        !result.data ||
        typeof result.data !== "object" ||
        !("widget" in result.data) ||
        !result.data.widget ||
        typeof result.data.widget !== "object"
      ) {
        throw new Error("The server returned invalid widget data");
      }
      return result.data.widget as WidgetData;
    };

    const refresh = () => {
      void loadWidget()
        .then((widget) => {
          if (!controller.signal.aborted) {
            setData(widget);
            setError("");
          }
        })
        .catch((cause: unknown) => {
          if (!controller.signal.aborted) {
            setError(cause instanceof Error ? cause.message : "Widget data could not be loaded");
          }
        });
    };

    void refresh();
    let interval: number | undefined;
    if (refreshInterval && refreshInterval > 0) {
      interval = window.setInterval(refresh, refreshInterval * 1_000);
    }

    return () => {
      controller.abort();
      clearInterval(interval);
    };
  }, [widgetName, itemId, refreshInterval]);

  return { data, error };
}

function PingStatus({ id }: { id: string }) {
  const [isUp, setIsUp] = useState<boolean | null>(null);

  useEffect(() => {
    let active = true;
    const controller = new AbortController();
    const params = new URLSearchParams({ id });
    const check = async () => {
      try {
        const response = await fetch(`/api/ping?${params.toString()}`, {
          cache: "no-store",
          signal: controller.signal,
        });
        const result = (await response.json()) as {
          status?: string;
          data?: { up?: unknown };
        };
        if (response.ok && result.status === "success" && typeof result.data?.up === "boolean") {
          if (active) setIsUp(result.data.up);
        }
      } catch {
        if (active) setIsUp(false);
      }
    };

    void check();
    const interval = window.setInterval(() => void check(), 30_000);
    return () => {
      active = false;
      controller.abort();
      window.clearInterval(interval);
    };
  }, [id]);

  return (
    <span
      className="service-ping"
      data-up={isUp === null ? undefined : String(isUp)}
      aria-label={
        isUp === null ? "Checking service status" : isUp ? "Service online" : "Service unavailable"
      }
      role="status"
    >
      <span className="status-dot" aria-hidden="true" />
    </span>
  );
}

function ServiceCard({
  service,
  editing = false,
}: {
  service: DashboardService;
  editing?: boolean;
}) {
  const content = (
    <>
      {service.icon ? (
        <img className="service-icon" src={service.icon} alt="" loading="lazy" decoding="async" />
      ) : (
        <span className="service-icon-fallback" aria-hidden="true">
          {service.title.charAt(0).toLocaleUpperCase() || "S"}
        </span>
      )}
      <span className="service-copy">
        <span className="service-title">{service.title}</span>
        {service.href && service.show_destination && (
          <span className="service-subtitle">{service.href}</span>
        )}
      </span>
      {service.has_ping ? (
        <PingStatus id={service.id} />
      ) : (
        <span className="service-open-mark" aria-hidden="true">
          ↗
        </span>
      )}
    </>
  );

  const className = `service-card${editing ? " h-full" : " service-layout-item"}`;
  const style = editing ? undefined : layoutStyle(service);
  return !editing && service.href ? (
    <a className={className} style={style} href={service.href} target="_blank" rel="noreferrer">
      {content}
    </a>
  ) : (
    <div className={className} style={style}>
      {content}
    </div>
  );
}

function Loading() {
  return <div className="widget-loading">Loading…</div>;
}

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="widget-error" role="alert">
      {message}
    </div>
  );
}

function ClockWidget({ data }: { data: WidgetData }) {
  const [now, setNow] = useState(() => new Date());
  const zone = typeof data.time_zone === "string" && data.time_zone ? data.time_zone : "UTC";
  const formatters = useMemo(() => {
    try {
      return {
        time: new Intl.DateTimeFormat("en-GB", {
          timeZone: zone,
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
          hour12: false,
        }),
        date: new Intl.DateTimeFormat("en-GB", {
          timeZone: zone,
          weekday: "short",
          day: "2-digit",
          month: "short",
          year: "numeric",
        }),
        offset: new Intl.DateTimeFormat("en", { timeZone: zone, timeZoneName: "shortOffset" }),
      };
    } catch {
      return {
        time: new Intl.DateTimeFormat("en-GB", {
          timeZone: "UTC",
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
          hour12: false,
        }),
        date: new Intl.DateTimeFormat("en-GB", {
          timeZone: "UTC",
          weekday: "short",
          day: "2-digit",
          month: "short",
          year: "numeric",
        }),
        offset: new Intl.DateTimeFormat("en", { timeZone: "UTC", timeZoneName: "shortOffset" }),
      };
    }
  }, [zone]);
  const parts = formatters.time.formatToParts(now);
  const hours = parts.find(({ type }) => type === "hour")?.value ?? "00";
  const minutes = parts.find(({ type }) => type === "minute")?.value ?? "00";
  const seconds = parts.find(({ type }) => type === "second")?.value ?? "00";
  const offsetLabel =
    formatters.offset.formatToParts(now).find(({ type }) => type === "timeZoneName")?.value ?? zone;

  useEffect(() => {
    if (typeof data.timestamp === "string") {
      const serverTime = new Date(data.timestamp);
      if (!Number.isNaN(serverTime.valueOf())) setNow(serverTime);
    }
    const timer = window.setInterval(() => setNow(new Date()), 1_000);
    return () => window.clearInterval(timer);
  }, [data.timestamp]);

  return (
    <div className="clock-widget">
      <div className="clock-topline">
        <span className="clock-place">{zone}</span>
        <span className="clock-offset">{offsetLabel}</span>
      </div>
      <time
        className="clock-time widget-value"
        dateTime={now.toISOString()}
        aria-label={`${hours}:${minutes}:${seconds} ${zone}`}
      >
        <span>
          {hours}:{minutes}
        </span>
        <span className="clock-seconds">:{seconds}</span>
      </time>
      <div className="clock-footer">
        <time className="clock-date widget-secondary" dateTime={now.toISOString()}>
          {formatters.date.format(now)}
        </time>
        <span className="clock-utc widget-secondary">
          UTC{" "}
          {now.toLocaleTimeString("en-GB", {
            timeZone: "UTC",
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
          })}
        </span>
      </div>
    </div>
  );
}

function QuickLinksWidget({ data }: { data: WidgetData }) {
  const links = Array.isArray(data.links)
    ? (data.links as Array<{
        title: string;
        url: string;
        icon?: string | null;
      }>)
    : [];

  return (
    <div className="space-y-2">
      <div className="space-y-1">
        {links.map((link) => (
          <a
            key={`${link.title}-${link.url}`}
            href={link.url}
            target="_blank"
            rel="noreferrer"
            className="quick-link"
          >
            {link.icon ? (
              <img
                className="service-icon"
                src={link.icon}
                alt=""
                loading="lazy"
                decoding="async"
              />
            ) : (
              <span className="quick-link-icon">
                {link.title.charAt(0).toLocaleUpperCase() || "?"}
              </span>
            )}
            <span className="quick-link-title">{link.title}</span>
            <span className="service-open-mark" aria-hidden="true">
              ↗
            </span>
          </a>
        ))}
      </div>
    </div>
  );
}

function formatBytes(bytes: number, unit: "MiB" | "GiB") {
  const divisor = unit === "MiB" ? 1_048_576 : 1_073_741_824;
  return `${(bytes / divisor).toFixed(0)}${unit === "MiB" ? "M" : "G"}`;
}

function formatUptime(seconds: number) {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  return days > 0 ? `${days}d ${hours}h ${minutes}m` : `${hours}h ${minutes}m`;
}

function StatRow({ label, percent, value }: { label: string; percent: number; value: string }) {
  return (
    <div className="widget-stat">
      <div className="widget-stat-heading">
        <span>{label}</span>
        <span className="widget-value">{value}</span>
      </div>
      <div className="widget-meter">
        <div
          className="widget-meter-fill"
          style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
        />
      </div>
    </div>
  );
}

function SystemStatsWidget({ data }: { data: WidgetData }) {
  const stats = data.stats as {
    cpu_usage_percent: number;
    memory: { used_bytes: number; total_bytes: number; usage_percent: number };
    disk: { mount: string; used_bytes: number; total_bytes: number; usage_percent: number };
    load: { one: number } | null;
    uptime_secs: number;
  };

  return (
    <div className="system-stats-widget">
      <div className="system-stats-heading">
        <span className="widget-title">{String(data.name ?? "Server")}</span>
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
        <span className="widget-value">{formatUptime(stats.uptime_secs)}</span>
      </div>
    </div>
  );
}

function WeatherWidget({ data }: { data: WidgetData }) {
  const current = data.current as {
    temperature: number;
    apparent_temperature: number;
    relative_humidity: number;
    description: string;
    icon: string;
    wind_speed: number;
    wind_direction: string;
  };
  const unit = data.units === "fahrenheit" ? "°F" : "°C";

  return (
    <div className="weather-widget">
      <div className="text-center">
        <div className="widget-secondary">
          {Number(data.latitude).toFixed(2)}°, {Number(data.longitude).toFixed(2)}°
        </div>
      </div>
      <div className="flex items-center justify-between">
        <div className="flex-1">
          <div className="weather-temperature widget-value">
            {current.temperature.toFixed(0)}
            {unit}
          </div>
          <div className="widget-secondary">
            Feels {current.apparent_temperature.toFixed(0)}
            {unit}
          </div>
          <div className="weather-description">{current.description}</div>
        </div>
        <div className="flex-shrink-0 text-3xl" aria-hidden="true">
          {current.icon}
        </div>
      </div>
      <div className="weather-details widget-divider">
        <div className="text-center">
          <div className="widget-secondary">Humidity</div>
          <div className="weather-detail-value widget-value">{current.relative_humidity}%</div>
        </div>
        <div className="text-center">
          <div className="widget-secondary">Wind</div>
          <div className="weather-detail-value widget-value">
            {current.wind_speed.toFixed(0)} {current.wind_direction}
          </div>
        </div>
      </div>
    </div>
  );
}

type CalendarEntry = {
  date: string;
  series_url: string;
  title: string;
  season_number: number;
  episode_number: number;
  air_time: string;
  status: string;
};

function SonarrCalendarWidget({ data }: { data: WidgetData }) {
  const entries = Array.isArray(data.entries) ? (data.entries as CalendarEntry[]) : [];
  const calendarUrl = typeof data.calendar_url === "string" ? data.calendar_url : "#";
  const grouped = new Map<string, CalendarEntry[]>();
  for (const entry of entries) grouped.set(entry.date, [...(grouped.get(entry.date) ?? []), entry]);

  return entries.length === 0 ? (
    <div className="grid min-h-full place-items-center text-xl font-bold">No entry</div>
  ) : (
    <div>
      {[...grouped.entries()].map(([date, dayEntries]) => (
        <div key={date}>
          <div className="calendar-date">
            <a className="widget-value" href={calendarUrl} target="_blank" rel="noreferrer">
              {date}
            </a>
          </div>
          {dayEntries.map((entry) => (
            <div
              key={`${entry.series_url}-${entry.season_number}-${entry.episode_number}`}
              className={`px-2 my-2 ${entry.status}`}
            >
              <a
                className="line-clamp-1 hover:brightness-125"
                href={entry.series_url}
                target="_blank"
                rel="noreferrer"
              >
                {entry.title}
              </a>
              <span className="block widget-secondary">
                {entry.season_number}x{String(entry.episode_number).padStart(2, "0")}
              </span>
              <span className="widget-secondary">{entry.air_time}</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

type RandomShow = {
  display: "backdrop" | "poster";
  title: string;
  series_url: string;
  poster_url: string | null;
  backdrop_url: string | null;
};

type ProwlarrIndexerRow = {
  name: string;
  enable: boolean;
  queries: number;
  grabs: number;
  failed_queries: number;
  average_response_time: number | null;
};

function parseProwlarrIndexer(value: unknown): ProwlarrIndexerRow | null {
  if (typeof value !== "object" || value === null) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.name !== "string" || record.name.length === 0) return null;
  return {
    name: record.name,
    enable: record.enable === true,
    queries:
      typeof record.queries === "number" && Number.isFinite(record.queries) ? record.queries : 0,
    grabs: typeof record.grabs === "number" && Number.isFinite(record.grabs) ? record.grabs : 0,
    failed_queries:
      typeof record.failed_queries === "number" && Number.isFinite(record.failed_queries)
        ? record.failed_queries
        : 0,
    average_response_time:
      typeof record.average_response_time === "number" &&
      Number.isFinite(record.average_response_time)
        ? record.average_response_time
        : null,
  };
}

function ProwlarrWidget({ data }: { data: WidgetData }) {
  const indexers = Array.isArray(data.indexers)
    ? (data.indexers as unknown[]).map(parseProwlarrIndexer).filter((row) => row !== null)
    : [];
  const totals = (typeof data.totals === "object" && data.totals !== null ? data.totals : {}) as {
    queries?: unknown;
    grabs?: unknown;
    average_response_time?: unknown;
  };
  const totalQueries = typeof totals.queries === "number" ? totals.queries : 0;
  const totalGrabs = typeof totals.grabs === "number" ? totals.grabs : 0;
  const totalResponseTime =
    typeof totals.average_response_time === "number" ? totals.average_response_time : null;

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
          {totalQueries}q · {totalGrabs}g
        </span>
        <span className="widget-secondary shrink-0 whitespace-nowrap">
          {totalResponseTime !== null ? `${Math.round(totalResponseTime)} ms` : "—"}
        </span>
      </div>
    </div>
  );
}

type AdGuardTopDomain = {
  domain: string;
  count: number;
};

function parseAdGuardTopDomain(value: unknown): AdGuardTopDomain | null {
  if (typeof value !== "object" || value === null) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.domain !== "string" || record.domain.length === 0) return null;
  return {
    domain: record.domain,
    count: typeof record.count === "number" && Number.isFinite(record.count) ? record.count : 0,
  };
}

function percentDisplay(percent: number): string {
  const rounded = Math.round(percent * 10) / 10;
  return `${Number.isInteger(rounded) ? rounded : rounded.toFixed(1)}%`;
}

function AdGuardWidget({ data }: { data: WidgetData }) {
  const numDnsQueries =
    typeof data.num_dns_queries === "number" && Number.isFinite(data.num_dns_queries)
      ? data.num_dns_queries
      : 0;
  const numBlocked =
    typeof data.num_blocked_filtering === "number" && Number.isFinite(data.num_blocked_filtering)
      ? data.num_blocked_filtering
      : 0;
  const blockedPercent =
    typeof data.blocked_percent === "number" && Number.isFinite(data.blocked_percent)
      ? data.blocked_percent
      : 0;
  const avgProcessingTime =
    typeof data.avg_processing_time === "number" && Number.isFinite(data.avg_processing_time)
      ? data.avg_processing_time
      : null;
  const rulesCount = typeof data.rules_count === "number" ? data.rules_count : 0;
  const protectionEnabled = data.protection_enabled === true;
  const topDomains = Array.isArray(data.top_blocked_domains)
    ? (data.top_blocked_domains as unknown[])
        .map(parseAdGuardTopDomain)
        .filter((row) => row !== null)
        .slice(0, 3)
    : [];

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-baseline gap-2">
        <span className="text-2xl font-bold tabular-nums">{numDnsQueries.toLocaleString()}</span>
        <span className="widget-secondary">DNS queries</span>
      </div>
      <div className="tabular-nums">
        {numBlocked.toLocaleString()} blocked · {percentDisplay(blockedPercent)}
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
            protectionEnabled ? "text-emerald-600 dark:text-emerald-400" : "text-red-500"
          }`}
        >
          {protectionEnabled ? "Protected" : "Disabled"}
        </span>
        <span className="widget-secondary shrink-0 whitespace-nowrap tabular-nums">
          {avgProcessingTime !== null ? `${Math.round(avgProcessingTime * 1000)} ms` : "—"} ·{" "}
          {rulesCount} rules
        </span>
      </div>
    </div>
  );
}

function JellyfinRandomShowWidget({ data }: { data: WidgetData }) {
  if (data.empty === true) {
    return (
      <div className="grid min-h-full place-items-center text-xl font-bold">
        No series in library
      </div>
    );
  }

  const show = data as RandomShow;
  const posterMode = show.display === "poster";
  const art = posterMode
    ? (show.poster_url ?? show.backdrop_url)
    : (show.backdrop_url ?? show.poster_url);

  return (
    <a
      className={`jellyfin-widget ${posterMode ? "jellyfin-poster-mode" : "jellyfin-backdrop-mode"}`}
      href={show.series_url || "#"}
      target="_blank"
      rel="noreferrer"
      aria-label={show.title}
    >
      {art ? (
        <img className="jellyfin-art" src={art} alt="" loading="lazy" decoding="async" />
      ) : (
        <IconPhotoOff className="jellyfin-art-fallback" stroke={1.5} aria-hidden="true" />
      )}
      {posterMode && show.title && <span className="jellyfin-poster-title">{show.title}</span>}
    </a>
  );
}

const PLUGIN_NAME_PREFIX = "Plugin:";

function parsePluginWidgetName(name: string): { pluginId: string; widgetId: string } | null {
  const rest = name.slice(PLUGIN_NAME_PREFIX.length);
  const separator = rest.indexOf(":");
  if (separator <= 0 || separator === rest.length - 1) return null;
  return { pluginId: rest.slice(0, separator), widgetId: rest.slice(separator + 1) };
}

/// Embeds a plugin's static asset bundle in a sandboxed iframe (opaque
/// origin) and publishes the tile's configuration through the postMessage
/// contract: `{ type: "vesta:config", title, config }`, sent when the frame
/// loads and re-sent whenever the saved configuration changes.
function PluginIframe({
  pluginId,
  entry,
  title,
  config,
}: {
  pluginId: string;
  entry: string;
  title: string;
  config: WidgetData;
}) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const [frameLoaded, setFrameLoaded] = useState(false);
  const configKey = useMemo(() => JSON.stringify(config ?? {}), [config]);

  useEffect(() => {
    if (!frameLoaded) return;
    frameRef.current?.contentWindow?.postMessage(
      { type: "vesta:config", title, config: JSON.parse(configKey) },
      "*",
    );
  }, [frameLoaded, configKey, title]);

  const src = `/api/plugins/${encodeURIComponent(pluginId)}/assets/${entry
    .split("/")
    .map(encodeURIComponent)
    .join("/")}`;

  return (
    <iframe
      className="plugin-widget-frame"
      ref={frameRef}
      src={src}
      title={`${title} plugin widget`}
      sandbox="allow-scripts allow-forms"
      onLoad={() => setFrameLoaded(true)}
    />
  );
}

function PluginWidgetContent({
  name,
  data,
  title,
}: {
  name: string;
  data: WidgetData;
  title?: string;
}) {
  const parsed = parsePluginWidgetName(name);
  const entry = typeof data.entry === "string" && data.entry.length > 0 ? data.entry : null;
  const config = (data.config ?? {}) as WidgetData;

  if (!parsed || !entry) {
    return (
      <ErrorMessage message="This plugin widget has no saved configuration yet. Add it to the dashboard to render." />
    );
  }
  return (
    <PluginIframe
      pluginId={parsed.pluginId}
      entry={entry}
      title={title ?? parsed.widgetId}
      config={config}
    />
  );
}

function WidgetContent({ name, data, title }: { name: string; data: WidgetData; title?: string }) {
  if (name.startsWith(PLUGIN_NAME_PREFIX)) {
    return <PluginWidgetContent name={name} data={data} title={title} />;
  }
  switch (name) {
    case "Clock":
      return <ClockWidget data={data} />;
    case "QuickLinks":
      return <QuickLinksWidget data={data} />;
    case "SystemStats":
      return <SystemStatsWidget data={data} />;
    case "Weather":
      return <WeatherWidget data={data} />;
    case "SonarrCalendar":
      return <SonarrCalendarWidget data={data} />;
    case "JellyfinRandomShow":
      return <JellyfinRandomShowWidget data={data} />;
    case "Prowlarr":
      return <ProwlarrWidget data={data} />;
    case "AdGuard":
      return <AdGuardWidget data={data} />;
    default:
      return <ErrorMessage message={`Unsupported widget: ${name}`} />;
  }
}

function WidgetFrame({
  widget,
  editing,
  children,
}: {
  widget: DashboardWidget;
  editing: boolean;
  children: ReactNode;
}) {
  return (
    <article
      className={`widget-frame${editing ? " h-full" : " service-layout-item"}`}
      style={editing ? undefined : layoutStyle(widget)}
      aria-label={`${widget.title} widget`}
    >
      <h3 className="widget-card-title">{widget.title}</h3>
      <div className="widget-card-content">{children}</div>
    </article>
  );
}

function LiveWidgetCard({ widget }: { widget: DashboardWidget }) {
  const { data, error } = useWidgetData(widget.name, widget.id, widget.refresh_interval_secs);
  return (
    <WidgetFrame widget={widget} editing={false}>
      {error ? (
        <ErrorMessage message={error} />
      ) : !data ? (
        <Loading />
      ) : (
        <WidgetContent name={widget.name} data={data} title={widget.title} />
      )}
    </WidgetFrame>
  );
}

function previewData(name: string): WidgetData {
  switch (name) {
    case "Clock":
      return {
        timestamp: "2026-11-14T12:34:56.000Z",
        time_zone: "UTC",
      };
    case "QuickLinks":
      return {
        links: [
          { title: "Documentation", url: "#", icon: null },
          { title: "Status page", url: "#", icon: null },
        ],
      };
    case "SystemStats":
      return {
        name: "Home server",
        stats: {
          cpu_usage_percent: 18.4,
          memory: { used_bytes: 3_221_225_472, total_bytes: 8_589_934_592, usage_percent: 37.5 },
          disk: {
            mount: "/",
            used_bytes: 322_122_547_200,
            total_bytes: 1_099_511_627_776,
            usage_percent: 29.3,
          },
          load: { one: 0.42 },
          uptime_secs: 138_240,
        },
      };
    case "Weather":
      return {
        latitude: 37.77,
        longitude: -122.42,
        units: "celsius",
        current: {
          temperature: 18,
          apparent_temperature: 17,
          relative_humidity: 62,
          description: "Partly cloudy",
          icon: "⛅",
          wind_speed: 12,
          wind_direction: "NW",
        },
      };
    case "SonarrCalendar":
      return {
        calendar_url: "#",
        entries: [
          {
            date: "2026-11-14",
            series_url: "#",
            title: "Example series",
            season_number: 2,
            episode_number: 4,
            air_time: "20:30",
            status: "upcoming",
          },
        ],
      };
    case "JellyfinRandomShow":
      return {
        display: "backdrop",
        title: "Example show",
        series_url: "#",
        poster_url: null,
        backdrop_url: null,
      };
    case "Prowlarr":
      return {
        indexers: [
          { name: "Example tracker", enable: true, queries: 128, grabs: 42 },
          { name: "Backup tracker", enable: true, queries: 96, grabs: 17 },
          { name: "Broken tracker", enable: true, queries: 31, grabs: 0, failed_queries: 7 },
          { name: "Retired tracker", enable: false, queries: 0, grabs: 0 },
        ],
        totals: { queries: 255, grabs: 59, average_response_time: 124.7 },
      };
    case "AdGuard":
      return {
        num_dns_queries: 48213,
        num_blocked_filtering: 6894,
        blocked_percent: 14.3,
        num_replaced_parental: 12,
        num_replaced_safebrowsing: 3,
        avg_processing_time: 0.021,
        top_blocked_domains: [
          { domain: "ads.example.com", count: 4120 },
          { domain: "tracker.example.net", count: 1877 },
          { domain: "telemetry.example.org", count: 897 },
        ],
        version: "0.107.44",
        protection_enabled: true,
        running: true,
        filters_count: 2,
        rules_count: 104213,
      };
    default:
      return {};
  }
}

function PreviewWidgetCard({ widget }: { widget: DashboardWidget }) {
  return (
    <WidgetFrame widget={widget} editing>
      <WidgetContent name={widget.name} data={previewData(widget.name)} title={widget.title} />
    </WidgetFrame>
  );
}

function WidgetCard({ widget, editing = false }: { widget: DashboardWidget; editing?: boolean }) {
  return editing ? <PreviewWidgetCard widget={widget} /> : <LiveWidgetCard widget={widget} />;
}

function WidgetPreview({ name }: { name: string }) {
  const data = previewData(name);
  return (
    <div className="widget-preview">
      <WidgetContent name={name} data={data} title={name} />
      <span className="widget-preview-note">Sample preview</span>
    </div>
  );
}

export { ServiceCard, WidgetCard, WidgetContent, WidgetPreview };
