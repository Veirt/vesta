import fs from "fs";
import toml from "toml";

let config: VestaConfig = {};

export function getConfig() {
    const configFile = fs.readFileSync("./config/vesta.toml", "utf-8");
    config = toml.parse(configFile);

    return config;
}

export { config };
