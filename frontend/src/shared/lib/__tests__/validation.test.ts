import { describe, expect, it } from "vite-plus/test";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/shared/lib/draft-models";
import { validateDashboardGroups, validateDashboardItems } from "@/shared/lib/validation";
import type { WidgetDescriptor } from "@/shared/schemas/core";

const weatherDescriptor: WidgetDescriptor = {
  name: "Weather",
  title: "Weather",
  description: "Forecast",
  default_width: 4,
  default_height: 3,
  min_width: 3,
  min_height: 3,
  fields: [
    {
      key: "latitude",
      label: "Latitude",
      kind: "number",
      required: true,
      description: "Lat",
      min: -90,
      max: 90,
    },
    {
      key: "refresh_interval",
      label: "Refresh",
      kind: "number",
      required: true,
      description: "Refresh",
      default: 600,
      min: 60,
      max: 3600,
    },
  ],
};

describe("validation helpers", () => {
  it("enforces descriptor field min and max bounds", () => {
    const widget: EditableDashboardItem = {
      kind: "widget",
      editor_id: "ed-1",
      id: "weather-1",
      title: "Weather",
      group: null,
      name: "Weather",
      config: { latitude: 120, refresh_interval: 30 },
      x: 0,
      y: 0,
      width: 4,
      height: 3,
    };

    const issues = validateDashboardItems([widget], [weatherDescriptor], [], []);
    expect(issues).toHaveLength(2);
    expect(issues.every((issue) => issue.message.includes("outside its supported range"))).toBe(
      true,
    );

    const validIssues = validateDashboardItems(
      [{ ...widget, config: { latitude: 37.7, refresh_interval: 300 } }],
      [weatherDescriptor],
      [],
      [],
    );
    expect(validIssues).toHaveLength(0);
  });

  it("allows legacy SystemStats tiles without a catalog descriptor", () => {
    const legacy: EditableDashboardItem = {
      kind: "widget",
      editor_id: "ed-legacy",
      id: "stats-1",
      title: "Server stats",
      group: null,
      name: "SystemStats",
      config: { refresh_interval: 5 },
      x: 0,
      y: 0,
      width: 4,
      height: 4,
    };
    expect(validateDashboardItems([legacy], [weatherDescriptor], [], [])).toHaveLength(0);
  });

  it("validates service URLs, icons, and group uniqueness", () => {
    const badService: EditableDashboardItem = {
      kind: "service",
      editor_id: "ed-svc",
      id: "svc",
      title: "App",
      group: null,
      href: "javascript:alert(1)",
      show_destination: true,
      icon: "/etc/passwd",
      ping_url: "ftp://bad",
      x: 0,
      y: 0,
      width: 3,
      height: 1,
    };
    const svcIssues = validateDashboardItems([badService], [], [], []);
    expect(svcIssues).toHaveLength(3);

    const groups: EditableDashboardGroup[] = [
      { id: "a", title: "Media", description: "", icon: "", width: 6 },
      { id: "b", title: "media", description: "", icon: "", width: 6 },
    ];
    expect(validateDashboardGroups(groups)).toHaveLength(1);
  });
});
