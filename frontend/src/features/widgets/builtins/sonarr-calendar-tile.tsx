import * as v from "valibot";
import {
  SonarrCalendarPayloadSchema,
  type CalendarEntry,
  type WidgetData,
} from "@/features/widgets/widget-schemas";

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
