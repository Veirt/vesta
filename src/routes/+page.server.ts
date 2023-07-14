import type { PageServerLoad } from "./$types";

import { config } from "$lib/server/secrets.js";

export const load = (() => {
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
