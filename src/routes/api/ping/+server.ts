import { getWidgetInfo } from "$lib/utils/widget";
import { json, type RequestHandler } from "@sveltejs/kit";
import axios, { isAxiosError } from "axios";
import https from "https";

export const GET = (async ({ url }) => {
    const group = url.searchParams.get("group")!;
    const title = url.searchParams.get("title")!;

    const { url: pingUrl } = getWidgetInfo(group, title).ping!;

    if (!pingUrl) {
        return json({
            error: "ping.url is undefined",
        });
    }

    const httpsAgent = new https.Agent({
        rejectUnauthorized: false,
    });

    let statusCode;
    try {
        let res;
        res = await axios.head(pingUrl, {
            httpsAgent,
            validateStatus() {
                return true;
            },
        });

        // use GET instead of HEAD when HEAD is not implemented
        if (res.status === 501) {
            res = await axios.get(pingUrl, {
                httpsAgent,
                validateStatus() {
                    return true;
                },
            });
        }
        statusCode = res.status;
    } catch (error) {
        if (isAxiosError(error)) {
            return json({ error: error.message });
        } else {
            return json({ error });
        }
    }

    return json({ statusCode });
}) satisfies RequestHandler;
