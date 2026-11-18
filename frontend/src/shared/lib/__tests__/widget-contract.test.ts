import * as v from "valibot";
import { describe, expect, it } from "vite-plus/test";
import contract from "../../../../../tests/fixtures/widget-contract.json";
import type { EditableDashboardItem } from "@/shared/lib/draft-models";
import { serviceMinSize, widgetMinSize } from "@/shared/lib/layout";
import { validateDashboardItems } from "@/shared/lib/validation";
import { WidgetDescriptorSchema } from "@/shared/schemas/core";

const descriptors = contract.descriptors.map((descriptor) =>
  v.parse(WidgetDescriptorSchema, descriptor),
);

describe("Rust to TypeScript widget contract", () => {
  it("accepts the independent Rust descriptor fixture with the real Valibot schema", () => {
    expect(descriptors).toEqual(contract.descriptors);
  });

  it("keeps Rust and TypeScript service and widget minimum sizes aligned", () => {
    for (const testCase of contract.layout.service_minimums) {
      expect(serviceMinSize(testCase.group_span)).toEqual({
        width: testCase.width,
        height: testCase.height,
      });
    }

    for (const testCase of contract.layout.widget_minimums) {
      expect(widgetMinSize(descriptors, testCase.name)).toEqual({
        width: testCase.width,
        height: testCase.height,
      });
    }
  });

  it("runs fixture configs through the real dashboard field validator", () => {
    for (const [index, testCase] of contract.validation_cases.entries()) {
      const descriptor = descriptors.find(({ name }) => name === testCase.name);
      expect(descriptor).toBeDefined();
      const item: EditableDashboardItem = {
        kind: "widget",
        editor_id: `contract-${index}`,
        id: `contract-${index}`,
        title: descriptor!.title,
        group: null,
        name: testCase.name,
        config: testCase.config,
        x: 0,
        y: 0,
        width: descriptor!.default_width,
        height: descriptor!.default_height,
      };

      const messages = validateDashboardItems([item], descriptors, [], []).map(
        ({ message }) => message,
      );
      expect(messages).toEqual(testCase.messages);
    }
  });
});
