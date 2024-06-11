import { getWidgetInfo } from "$lib/utils/widget";
import { json, type RequestHandler } from "@sveltejs/kit";
import axios from "axios";
import https from "https";

const instance = axios.create({
    httpsAgent: new https.Agent({ rejectUnauthorized: false }),
    validateStatus() {
        return true;
    },
});

async function checkWebsiteStatus(pingUrl: string) {
    let response = await instance.head(pingUrl);
    if (response.status === 501) {
        // Use GET if HEAD is not implemented
        response = await instance.get(pingUrl);
    }

    return response.status;
}

export const GET = (async ({ url }) => {
    const group = url.searchParams.get("group")!;
    const title = url.searchParams.get("title")!;
    const { url: pingUrl } = getWidgetInfo(group, title).ping!;

    try {
        const statusCode = await checkWebsiteStatus(pingUrl);
        return json({ statusCode });
    } catch (error) {
        return json({ error });
    }
}) satisfies RequestHandler;
