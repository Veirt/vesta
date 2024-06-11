import { getWidgetInfo } from "$lib/utils/widget";
import { json, type RequestHandler } from "@sveltejs/kit";
import isReachable from "is-reachable";

export const GET = (async ({ url }) => {
    const group = url.searchParams.get("group")!;
    const title = url.searchParams.get("title")!;
    const { url: pingUrl } = getWidgetInfo(group, title).ping!;

    const status = await isReachable(pingUrl);

    return json({ status });
}) satisfies RequestHandler;
