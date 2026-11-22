import { useEffect, useMemo, useState } from "react";
import * as v from "valibot";
import { ClockPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";

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
