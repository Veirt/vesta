import { getWidgetInfo } from "$lib/utils/widget";
import { json, type RequestHandler } from "@sveltejs/kit";
import axios, { isAxiosError } from "axios";
import https from "https";

const httpsAgent = new https.Agent({
    rejectUnauthorized: false,
});
const instance = axios.create({
    httpsAgent,
    validateStatus() {
        return true;
    },
});

export const GET = (async ({ url }) => {
    const group = url.searchParams.get("group")!;
    const title = url.searchParams.get("title")!;

    const { url: pingUrl } = getWidgetInfo(group, title).ping!;

    if (!pingUrl) {
        return json({
            error: "ping.url is undefined",
        });
    }

    let statusCode;
    try {
        let res;
        res = await instance.head(pingUrl);

        // use GET instead of HEAD when HEAD is not implemented
        if (res.status === 501) {
            res = await instance.get(pingUrl);
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
