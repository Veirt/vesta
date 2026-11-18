import { describe, expect, it } from "vite-plus/test";
import { groupDashboardItems, serviceMinSize, widgetMinSize } from "@/shared/lib/layout";
import type { WidgetDescriptor } from "@/shared/schemas/core";

const sampleWidgets: WidgetDescriptor[] = [
  {
    name: "Clock",
    title: "Clock",
    description: "Clock",
    default_width: 4,
    default_height: 2,
    min_width: 3,
    min_height: 2,
    fields: [],
  },
];

describe("layout helpers", () => {
  it("relaxes service minimum width as group span widens", () => {
    expect(serviceMinSize(null)).toEqual({ width: 3, height: 1 });
    expect(serviceMinSize(3)).toEqual({ width: 3, height: 1 });
    expect(serviceMinSize(6)).toEqual({ width: 3, height: 1 });
    expect(serviceMinSize(8)).toEqual({ width: 2, height: 1 });
    expect(serviceMinSize(12)).toEqual({ width: 1, height: 1 });
  });

  it("uses descriptor mins and preserves legacy hidden SystemStats minimum", () => {
    expect(widgetMinSize(sampleWidgets, "Clock")).toEqual({ width: 3, height: 2 });
    expect(widgetMinSize([], "SystemStats")).toEqual({ width: 4, height: 4 });
    expect(widgetMinSize([], "UnknownPlugin")).toEqual({ width: 3, height: 2 });
  });

  it("orders groups using explicit metadata and expands sole Ungrouped section", () => {
    const sections = groupDashboardItems(
      [
        { id: "a", group: "media", x: 0, y: 0 },
        { id: "b", group: null, x: 0, y: 0 },
      ],
      [
        { id: "media", title: "Media", width: 8 },
        { id: "", title: "Ungrouped", width: 4 },
      ],
      true,
    );
    expect(sections.map((s) => s.id)).toEqual(["media", ""]);
    expect(sections[0].width).toBe(8);
    expect(sections[1].label).toBeNull();

    const ungroupedOnly = groupDashboardItems([{ id: "solo", group: null, x: 0, y: 0 }], []);
    expect(ungroupedOnly).toHaveLength(1);
    expect(ungroupedOnly[0].width).toBe(12);
  });
});
