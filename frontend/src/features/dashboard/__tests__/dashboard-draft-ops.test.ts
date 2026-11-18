import { describe, expect, it } from "vite-plus/test";
import {
  compactY,
  moveItemToGroupDraft,
  removeGroupFromItems,
  resizeItemDraft,
} from "@/features/dashboard/dashboard-draft-ops";
import type { EditableDashboardGroup, EditableDashboardItem } from "@/shared/lib/draft-models";
import type { WidgetDescriptor } from "@/shared/schemas/core";

function serviceItem(
  id: string,
  x: number,
  y: number,
  width = 4,
  height = 1,
  group: string | null = null,
): EditableDashboardItem {
  return {
    kind: "service",
    editor_id: `ed-${id}`,
    id,
    title: id,
    group,
    href: "",
    show_destination: true,
    icon: "",
    ping_url: "",
    x,
    y,
    width,
    height,
  };
}

describe("dashboard draft operations", () => {
  it("retains tiles as Ungrouped when removing their section", () => {
    const items = [
      serviceItem("ungrouped", 0, 0, 4, 1, null),
      serviceItem("media-one", 0, 0, 4, 1, "media"),
      serviceItem("media-two", 4, 0, 4, 1, "media"),
    ];

    const updated = removeGroupFromItems(items, "media");
    expect(updated).toHaveLength(3);
    expect(updated.every((item) => item.group === null)).toBe(true);
    expect(updated.map((item) => ({ id: item.id, x: item.x, y: item.y }))).toEqual([
      { id: "ungrouped", x: 0, y: 0 },
      { id: "media-one", x: 4, y: 0 },
      { id: "media-two", x: 8, y: 0 },
    ]);
  });

  it("compacts y coordinates independently per group without cross-group collision", () => {
    const items = [
      serviceItem("a", 0, 5, 4, 2, null),
      serviceItem("b", 0, 4, 4, 2, "media"),
      serviceItem("c", 0, 9, 4, 1, "media"),
    ];
    const compacted = compactY(items);
    expect(compacted.find((i) => i.id === "a")?.y).toBe(0);
    expect(compacted.find((i) => i.id === "b")?.y).toBe(0);
    expect(compacted.find((i) => i.id === "c")?.y).toBe(2);
  });

  it("clamps tile resize to neighbour space and minimum dimensions", () => {
    const groups: EditableDashboardGroup[] = [
      { id: "media", title: "Media", description: "", icon: "", width: 6 },
    ];
    const descriptors: WidgetDescriptor[] = [];
    const items = [
      serviceItem("left", 0, 0, 4, 1, "media"),
      serviceItem("blocker", 6, 0, 4, 1, "media"),
    ];

    const shrunk = resizeItemDraft(items, groups, descriptors, "ed-left", 1, 1);
    expect(shrunk[0].width).toBe(3);

    const expanded = resizeItemDraft(items, groups, descriptors, "ed-left", 10, 2);
    expect(expanded[0].width).toBe(6);
    expect(expanded[0].height).toBe(2);
  });

  it("moves tiles between groups and reflows source and destination", () => {
    const items = [
      serviceItem("s1", 0, 0, 4, 1, "media"),
      serviceItem("s2", 4, 0, 4, 1, "media"),
      serviceItem("d1", 0, 0, 4, 1, "ops"),
    ];
    const moved = moveItemToGroupDraft(items, "ed-s1", "ops");
    expect(moved.find((i) => i.id === "s2")).toMatchObject({ group: "media", x: 0, y: 0 });
    expect(moved.find((i) => i.id === "s1")).toMatchObject({ group: "ops", x: 4, y: 0 });
  });
});
