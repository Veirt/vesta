import fs from "fs";
import toml from "toml";
import type { PageServerLoad } from "./$types";

export const load = (() => {
    const config = fs.readFileSync("./config.toml", "utf-8");

    const parsedConfig: VestaConfig = toml.parse(config);

    return { config: parsedConfig };
}) satisfies PageServerLoad;
