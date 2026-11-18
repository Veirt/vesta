import * as v from "valibot";
import { describe, expect, it } from "vite-plus/test";
import {
  buildPluginAssetUrl,
  parsePluginWidgetName,
  PluginWidgetPayloadSchema,
  previewData,
} from "@/features/widgets/widget-schemas";

describe("widget schemas and plugin helpers", () => {
  it("parses valid Plugin:<plugin>:<widget> identifiers and rejects malformed ones", () => {
    expect(parsePluginWidgetName("Plugin:hello-world:counter")).toEqual({
      pluginId: "hello-world",
      widgetId: "counter",
    });
    expect(parsePluginWidgetName("Clock")).toBeNull();
    expect(parsePluginWidgetName("Plugin:missing-widget:")).toBeNull();
    expect(parsePluginWidgetName("Plugin::counter")).toBeNull();
  });

  it("encodes plugin asset URL segments safely", () => {
    expect(buildPluginAssetUrl("hello-world", "dist/my widget.html")).toBe(
      "/api/plugins/hello-world/assets/dist/my%20widget.html",
    );
  });

  it("decodes plugin widget payloads and provides preview fixtures for builtins", () => {
    const parsed = v.parse(PluginWidgetPayloadSchema, {
      entry: "index.html",
      config: { label: "Counter", start_value: 3 },
    });
    expect(parsed.entry).toBe("index.html");
    expect(parsed.config).toEqual({ label: "Counter", start_value: 3 });

    for (const builtin of [
      "Clock",
      "QuickLinks",
      "SystemStats",
      "Weather",
      "SonarrCalendar",
      "JellyfinRandomShow",
      "Prowlarr",
      "AdGuard",
    ]) {
      expect(Object.keys(previewData(builtin)).length).toBeGreaterThan(0);
    }
  });
});
