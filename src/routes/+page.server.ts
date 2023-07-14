import type { PageServerLoad } from "./$types";
const { getConfig } = await import("$lib/server/secrets");

export const load = (async () => {
    const config = getConfig();
    const newConfig: VestaConfig = JSON.parse(JSON.stringify(config)); // hacky deep copy

    // remove secret stuffs from config
    for (const group of Object.keys(newConfig)) {
        newConfig[group]["services"].forEach((service) => {
            if (service?.widget?.name) {
                service.widget = { name: service.widget.name };
            }

            return service;
        });
    }

    return { config: newConfig };
}) satisfies PageServerLoad;
