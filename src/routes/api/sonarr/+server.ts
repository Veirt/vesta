import { config } from "$lib/server/secrets";
import { json } from "@sveltejs/kit";
import type { RequestHandler } from "./$types";
import axios from "axios";
import type { SonarrCalendarParams } from "$lib/widgets/SonarrCalendar/types";
import { addDays, formatYYYY_MM_DD } from "$lib/utils/date";

const currentDate = new Date();
const start = currentDate;
const end = addDays(new Date(), 2);

function getWidgetInfo(group: string, title: string): Service {
    return config[group]["services"].find(
        (service) => service.title === title
    )!;
}

function fetchCalendar(params: SonarrCalendarParams, url: string, key: string) {
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
                `Failed fetching Sonarr's calendar. Details: ${error}`
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

    const params: SonarrCalendarParams = {
        unmonitored: false,
        includeSeries: true,
        start: formatYYYY_MM_DD(start),
        end: formatYYYY_MM_DD(end),
    };

    const calendar = await fetchCalendar(params, apiUrl, key);

    return json(calendar);
}) satisfies RequestHandler;