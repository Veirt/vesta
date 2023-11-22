import { addDays, formatYYYY_MM_DD } from "$lib/utils/date";
import { getWidgetInfo } from "$lib/utils/widget";
import type {
    Calendar,
    DownloadQueue,
    SonarrCalendarParams,
} from "$lib/widgets/SonarrCalendar/types";
import { json } from "@sveltejs/kit";
import axios from "axios";
import type { RequestHandler } from "./$types";

function fetchCalendar(
    params: SonarrCalendarParams,
    url: string,
    key: string,
): Promise<Calendar> {
    return new Promise(async (resolve, reject) => {
        try {
            // https://sonarr.tv/docs/api/#/Calendar/get_api_v3_calendar
            const calendarRes = await axios({
                method: "GET",
                baseURL: url,
                url: "/api/v3/calendar",
                params,
                headers: {
                    "X-Api-Key": key,
                },
            });

            return resolve(calendarRes.data);
        } catch (error) {
            console.error(
                `Failed to fetch Sonarr's calendar. Details: ${error}`,
            );

            return reject({ error });
        }
    });
}

function fetchDownloadQueue(url: string, key: string): Promise<DownloadQueue> {
    return new Promise(async (resolve, reject) => {
        try {
            const downloadQueueRes = await axios({
                method: "GET",
                baseURL: url,
                url: "/api/v3/queue",
                headers: {
                    "X-Api-Key": key,
                },
            });

            return resolve(downloadQueueRes.data);
        } catch (error) {
            console.error(
                `Failed to fetch Sonarr's download queue. Details: ${error}`,
            );

            return reject({ error });
        }
    });
}

export const GET = (async ({ url }) => {
    const group = url.searchParams.get("group")!;
    const title = url.searchParams.get("title")!;

    const { url: apiUrl, key } = getWidgetInfo(group, title).widget!;

    if (!apiUrl || !key) {
        return json({
            error: "url or key is undefined",
        });
    }

    // change later
    const currentDate = addDays(new Date(), -1);
    const start = currentDate;
    const end = addDays(new Date(), 2);

    const params: SonarrCalendarParams = {
        unmonitored: false,
        includeSeries: true,
        start: formatYYYY_MM_DD(start),
        end: formatYYYY_MM_DD(end),
    };

    const calendar = await fetchCalendar(params, apiUrl, key);
    const downloadQueue = await fetchDownloadQueue(apiUrl, key);

    calendar.map((cal) => {
        cal.downloading = false;
        for (const record of downloadQueue.records) {
            if (cal.seriesId === record.seriesId) {
                cal.downloading = true;
            }
        }
    });

    return json(calendar);
}) satisfies RequestHandler;
