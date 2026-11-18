import { IconPhotoOff } from "@tabler/icons-react";
import { useEffect, useMemo, useState } from "react";
import * as v from "valibot";
import {
  AdGuardPayloadSchema,
  ClockPayloadSchema,
  JellyfinRandomShowPayloadSchema,
  ProwlarrPayloadSchema,
  QuickLinksPayloadSchema,
  SonarrCalendarPayloadSchema,
  SystemStatsPayloadSchema,
  WeatherPayloadSchema,
  type CalendarEntry,
  type WidgetData,
} from "@/features/widgets/widget-schemas";

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

function percentDisplay(percent: number): string {
  const rounded = Math.round(percent * 10) / 10;
  return `${Number.isInteger(rounded) ? rounded : rounded.toFixed(1)}%`;
}

export function WidgetError({ message }: { message: string }) {
  return (
    <div className="widget-error" role="alert">
      {message}
    </div>
  );
}

function buildClockFormatters(timeZone: string) {
  return {
    time: new Intl.DateTimeFormat("en-GB", {
      timeZone,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    }),
    date: new Intl.DateTimeFormat("en-GB", {
      timeZone,
      weekday: "short",
      day: "2-digit",
      month: "short",
      year: "numeric",
    }),
    offset: new Intl.DateTimeFormat("en", { timeZone, timeZoneName: "shortOffset" }),
  };
}

export function ClockTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(ClockPayloadSchema, data);
  const timestamp = parsed.success ? parsed.output.timestamp : undefined;
  const zone = parsed.success && parsed.output.time_zone ? parsed.output.time_zone : "UTC";
  const [now, setNow] = useState(() => new Date());

  const formatters = useMemo(() => {
    try {
      return buildClockFormatters(zone);
    } catch {
      return buildClockFormatters("UTC");
    }
  }, [zone]);

  const parts = formatters.time.formatToParts(now);
  const hours = parts.find(({ type }) => type === "hour")?.value ?? "00";
  const minutes = parts.find(({ type }) => type === "minute")?.value ?? "00";
  const seconds = parts.find(({ type }) => type === "second")?.value ?? "00";
  const offsetLabel =
    formatters.offset.formatToParts(now).find(({ type }) => type === "timeZoneName")?.value ?? zone;

  useEffect(() => {
    if (timestamp) {
      const serverTime = new Date(timestamp);
      if (!Number.isNaN(serverTime.valueOf())) setNow(serverTime);
    }
    const timer = window.setInterval(() => setNow(new Date()), 1_000);
    return () => window.clearInterval(timer);
  }, [timestamp]);

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

export function QuickLinksTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(QuickLinksPayloadSchema, data);
  const links = parsed.success ? parsed.output.links : [];

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
        <span className="widget-value">{formatUptime(stats.uptime_secs)}</span>
      </div>
    </div>
  );
}

export function WeatherTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(WeatherPayloadSchema, data);
  if (!parsed.success) return <WidgetError message="Invalid weather payload." />;
  const { latitude, longitude, units, current } = parsed.output;
  const unit = units === "fahrenheit" ? "°F" : "°C";

  return (
    <div className="weather-widget">
      <div className="text-center">
        <div className="widget-secondary">
          {latitude.toFixed(2)}°, {longitude.toFixed(2)}°
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
        <div className="shrink-0 text-3xl" aria-hidden="true">
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

export function SonarrCalendarTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(SonarrCalendarPayloadSchema, data);
  const calendarUrl = parsed.success ? parsed.output.calendar_url : "#";
  const entries = parsed.success ? parsed.output.entries : [];
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

export function JellyfinTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(JellyfinRandomShowPayloadSchema, data);
  if (!parsed.success) return <WidgetError message="Invalid Jellyfin payload." />;
  if ("empty" in parsed.output) {
    return (
      <div className="grid min-h-full place-items-center text-xl font-bold">
        No series in library
      </div>
    );
  }

  const show = parsed.output;
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
