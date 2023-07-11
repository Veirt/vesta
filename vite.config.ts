import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import liveReload from "vite-plugin-live-reload";

export default defineConfig({
    plugins: [sveltekit(), liveReload("config.toml")],
});
