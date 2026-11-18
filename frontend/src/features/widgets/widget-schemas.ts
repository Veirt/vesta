import type { CSSProperties } from "react";
import * as v from "valibot";
import type { DashboardService, DashboardWidget } from "@/shared/schemas/core";

export type WidgetData = Record<string, unknown>;

export const PLUGIN_NAME_PREFIX = "Plugin:";

export function parsePluginWidgetName(name: string): { pluginId: string; widgetId: string } | null {
  if (!name.startsWith(PLUGIN_NAME_PREFIX)) return null;
  const rest = name.slice(PLUGIN_NAME_PREFIX.length);
  const separator = rest.indexOf(":");
  if (separator <= 0 || separator === rest.length - 1) return null;
  return { pluginId: rest.slice(0, separator), widgetId: rest.slice(separator + 1) };
}

export function buildPluginAssetUrl(pluginId: string, entry: string): string {
  return `/api/plugins/${encodeURIComponent(pluginId)}/assets/${entry
    .split("/")
    .map(encodeURIComponent)
    .join("/")}`;
}

export function previewData(name: string): WidgetData {
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

export function layoutStyle(item: DashboardService | DashboardWidget): CSSProperties {
  const width = Math.max(1, Math.min(12, item.width ?? 4));
  return {
    "--tile-x": Math.min(item.x ?? 0, 12 - width),
    "--tile-y": Math.min(item.y ?? 0, 10_000),
    "--tile-width": width,
    "--tile-height": Math.max(1, Math.min(12, item.height ?? (item.kind === "widget" ? 2 : 1))),
  } as CSSProperties;
}

export const ClockPayloadSchema = v.object({
  timestamp: v.optional(v.string()),
  time_zone: v.optional(v.string(), "UTC"),
});

export const QuickLinkSchema = v.object({
  title: v.string(),
  url: v.string(),
  icon: v.optional(v.nullable(v.string()), null),
});

export const QuickLinksPayloadSchema = v.object({
  links: v.optional(v.array(QuickLinkSchema), []),
});

export const SystemStatsPayloadSchema = v.object({
  name: v.optional(v.nullable(v.string()), "Server"),
  stats: v.object({
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
    load: v.nullable(v.object({ one: v.number() })),
    uptime_secs: v.number(),
  }),
});

export const WeatherPayloadSchema = v.object({
  latitude: v.number(),
  longitude: v.number(),
  units: v.optional(v.string(), "celsius"),
  current: v.object({
    temperature: v.number(),
    apparent_temperature: v.number(),
    relative_humidity: v.number(),
    description: v.string(),
    icon: v.string(),
    wind_speed: v.number(),
    wind_direction: v.string(),
  }),
});

export const SonarrCalendarPayloadSchema = v.object({
  calendar_url: v.optional(v.string(), "#"),
  entries: v.optional(
    v.array(
      v.object({
        date: v.string(),
        series_url: v.string(),
        title: v.string(),
        season_number: v.number(),
        episode_number: v.number(),
        air_time: v.string(),
        status: v.string(),
      }),
    ),
    [],
  ),
});

export const JellyfinRandomShowPayloadSchema = v.union([
  v.object({ empty: v.literal(true) }),
  v.object({
    display: v.picklist(["backdrop", "poster"]),
    title: v.string(),
    series_url: v.optional(v.string(), "#"),
    poster_url: v.optional(v.nullable(v.string()), null),
    backdrop_url: v.optional(v.nullable(v.string()), null),
  }),
]);

export const ProwlarrIndexerRowSchema = v.object({
  name: v.pipe(v.string(), v.minLength(1)),
  enable: v.optional(v.boolean(), false),
  queries: v.optional(v.number(), 0),
  grabs: v.optional(v.number(), 0),
  failed_queries: v.optional(v.number(), 0),
  average_response_time: v.optional(v.nullable(v.number()), null),
});

export const ProwlarrPayloadSchema = v.object({
  indexers: v.optional(v.array(ProwlarrIndexerRowSchema), []),
  totals: v.optional(
    v.object({
      queries: v.optional(v.number(), 0),
      grabs: v.optional(v.number(), 0),
      average_response_time: v.optional(v.nullable(v.number()), null),
    }),
    {},
  ),
});

export const AdGuardTopDomainSchema = v.object({
  domain: v.pipe(v.string(), v.minLength(1)),
  count: v.optional(v.number(), 0),
});

export const AdGuardPayloadSchema = v.object({
  num_dns_queries: v.optional(v.number(), 0),
  num_blocked_filtering: v.optional(v.number(), 0),
  blocked_percent: v.optional(v.number(), 0),
  avg_processing_time: v.optional(v.nullable(v.number()), null),
  rules_count: v.optional(v.number(), 0),
  protection_enabled: v.optional(v.boolean(), false),
  top_blocked_domains: v.optional(v.array(AdGuardTopDomainSchema), []),
});

export const PluginWidgetPayloadSchema = v.object({
  entry: v.pipe(v.string(), v.minLength(1)),
  config: v.optional(v.record(v.string(), v.unknown()), {}),
});

export type CalendarEntry = v.InferOutput<typeof SonarrCalendarPayloadSchema>["entries"][number];
export type ProwlarrIndexerRow = v.InferOutput<typeof ProwlarrIndexerRowSchema>;
export type AdGuardTopDomain = v.InferOutput<typeof AdGuardTopDomainSchema>;
