import z from "zod";

export interface SonarrCalendarParams {
    unmonitored: boolean;
    includeSeries: boolean;
    start: string;
    end: string;
}

export const calendarEntrySchema = z.object({
    seriesId: z.number(),
    seasonNumber: z.number(), // necessary
    episodeNumber: z.number(), // necessary
    title: z.optional(z.string()),
    airDateUtc: z.string(), // necessary
    hasFile: z.boolean(), // necessary
    monitored: z.boolean(),
    series: z.object({
        title: z.string(),
        titleSlug: z.string(),
        runtime: z.number(),
    }),
    downloading: z.boolean(), // custom
});

export const calendarSchema = z.array(calendarEntrySchema);

export type CalendarEntry = z.infer<typeof calendarEntrySchema>;
export type Calendar = z.infer<typeof calendarSchema>;

export const downloadQueueSchema = z.object({
    records: z.array(
        z.object({
            seriesId: z.number(),
            status: z.string(),
            title: z.string(),
        }),
    ),
});

export type DownloadQueue = z.infer<typeof downloadQueueSchema>;
