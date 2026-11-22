import { IconPhotoOff } from "@tabler/icons-react";
import * as v from "valibot";
import { WidgetError } from "@/features/widgets/builtins/shared";
import {
  JellyfinRandomShowPayloadSchema,
  type WidgetData,
} from "@/features/widgets/widget-schemas";

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
      {show.title && <span className="jellyfin-title">{show.title}</span>}
    </a>
  );
}
