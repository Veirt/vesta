import fs from "fs";
import toml from "toml";
import type { PageServerLoad } from "./$types";

interface Service {
    title: string;
    href: string;
    imgSrc: string;
    width: 1 | 2 | 3 | 4;
}

interface VestaConfig {
    [group: string]: {
        name: string;
        columns: 1 | 2 | 3 | 4;
        services: Service[];
    };
}

export const load = (() => {
    const config = fs.readFileSync("./config.toml", "utf-8");

    const parsedConfig: VestaConfig = toml.parse(config);

    return { config: parsedConfig };
}) satisfies PageServerLoad;
