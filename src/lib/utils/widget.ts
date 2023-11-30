import { config } from "$lib/server/secrets";

export function getWidgetInfo(group: string, title: string): Service {
    return config[group]["services"].find((service) => service.title === title)!;
}
