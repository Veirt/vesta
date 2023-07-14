import fs from "fs";
import toml from "toml";
import chokidar from "chokidar";

const configFile = fs.readFileSync("./config.toml", "utf-8");
let config: VestaConfig = toml.parse(configFile);

chokidar.watch("./config.toml").on("change", (_event, _path) => {
    config = toml.parse(fs.readFileSync("./config.toml", "utf-8"));
});

export { config };
