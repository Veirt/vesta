import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, splitVendorChunkPlugin } from "vite";
import { compression } from "vite-plugin-compression2";

export default defineConfig({
    plugins: [
        sveltekit(),
        splitVendorChunkPlugin(),
        compression({ algorithm: "brotliCompress" }),
    ],
});
