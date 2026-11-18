import { describe, expect, it } from "vite-plus/test";
import { decode } from "@/shared/api/client";
import { InstalledPluginResponseSchema, RemovedPluginResponseSchema } from "@/shared/schemas/api";

describe("API decoder", () => {
  it("decodes plugin removal responses matching DELETE /api/plugins/:id", async () => {
    const response = new Response(
      JSON.stringify({ status: "success", data: { removed: "hello-world" } }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
    const decoded = await decode(response, RemovedPluginResponseSchema);
    expect(decoded.data.removed).toBe("hello-world");
  });

  it("surfaces server ApiError messages on non-OK HTTP responses", async () => {
    const response = new Response(
      JSON.stringify({
        status: "error",
        error: "plugin_error",
        message: "plugin 'hello-world' is still used by dashboard tiles",
      }),
      { status: 400, headers: { "Content-Type": "application/json" } },
    );
    await expect(decode(response, InstalledPluginResponseSchema)).rejects.toThrow(
      "plugin 'hello-world' is still used by dashboard tiles",
    );
  });

  it("rejects non-JSON bodies and schema mismatches cleanly", async () => {
    const badJson = new Response("<html>502</html>", { status: 502 });
    await expect(decode(badJson, RemovedPluginResponseSchema)).rejects.toThrow(
      "unreadable response (502)",
    );

    const wrongShape = new Response(JSON.stringify({ status: "success", data: { id: 123 } }), {
      status: 200,
    });
    await expect(decode(wrongShape, RemovedPluginResponseSchema)).rejects.toThrow(
      "unexpected format",
    );
  });
});
