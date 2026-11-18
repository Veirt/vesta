import { describe, expect, it } from "vite-plus/test";
import { type EditableDashboardItem, serializeDashboardItems } from "@/shared/lib/draft-models";
import type { WidgetDescriptor } from "@/shared/schemas/core";

const clockDescriptor: WidgetDescriptor = {
  name: "Clock",
  title: "Clock",
  description: "Clock",
  default_width: 4,
  default_height: 2,
  min_width: 3,
  min_height: 2,
  fields: [
    {
      key: "time_zone",
      label: "Time zone",
      kind: "text",
      required: false,
      description: "IANA zone",
    },
  ],
};

describe("draft model serialization", () => {
  it("strips unknown keys when descriptor exists but preserves hidden SystemStats config", () => {
    const items: EditableDashboardItem[] = [
      {
        kind: "widget",
        editor_id: "ed-clock",
        id: " clock-1 ",
        title: " Clock ",
        group: null,
        name: "Clock",
        config: { time_zone: "UTC", stale_key: "drop-me" },
        x: 0,
        y: 0,
        width: 4,
        height: 2,
      },
      {
        kind: "widget",
        editor_id: "ed-stats",
        id: "stats-1",
        title: "System stats",
        group: null,
        name: "SystemStats",
        config: { node: "local", refresh_interval: 5 },
        x: 4,
        y: 0,
        width: 4,
        height: 4,
      },
      {
        kind: "service",
        editor_id: "ed-svc",
        id: " svc-1 ",
        title: " Docs ",
        group: " ",
        href: " https://example.test ",
        show_destination: true,
        icon: " ",
        ping_url: " ",
        x: 0,
        y: 4,
        width: 4,
        height: 1,
      },
    ];

    const saved = serializeDashboardItems(items, [clockDescriptor]);
    expect(saved[0]).toMatchObject({
      kind: "widget",
      id: "clock-1",
      title: "Clock",
      config: { time_zone: "UTC" },
    });
    expect(saved[1]).toMatchObject({
      kind: "widget",
      name: "SystemStats",
      config: { node: "local", refresh_interval: 5 },
    });
    expect(saved[2]).toMatchObject({
      kind: "service",
      id: "svc-1",
      group: null,
      href: "https://example.test",
      icon: null,
      ping: null,
    });
  });
});
