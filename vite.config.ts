import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, splitVendorChunkPlugin } from "vite";

export default defineConfig({
    plugins: [sveltekit(), splitVendorChunkPlugin()],
});
