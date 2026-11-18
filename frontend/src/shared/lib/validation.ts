import { groupSpanOf, minSizeFor } from "@/shared/lib/layout";
import type {
  EditableDashboardGroup,
  EditableDashboardItem,
  EditableWidget,
} from "@/shared/lib/draft-models";
import type { WidgetDescriptor } from "@/shared/schemas/core";

export type ValidationIssue = { id: string; title: string; message: string };

export function validHttpUrl(value: string) {
  try {
    const url = new URL(value);
    return (url.protocol === "http:" || url.protocol === "https:") && Boolean(url.hostname);
  } catch {
    return false;
  }
}

export function validLink(value: string) {
  return (value.startsWith("/") && !value.startsWith("//")) || validHttpUrl(value);
}

export function validImage(value: string) {
  return /^\/api\/icons\/\d+$/.test(value) || validHttpUrl(value);
}

function empty(value: unknown) {
  return value === null || value === undefined || value === "";
}

function validateQuickLinks(value: unknown): string | null {
  if (!Array.isArray(value)) return "Links must be a list.";
  for (let index = 0; index < value.length; index += 1) {
    const link = value[index];
    if (!link || typeof link !== "object" || Array.isArray(link)) {
      return `Link ${index + 1} is incomplete.`;
    }
    const record = link as Record<string, unknown>;
    const title = typeof record.title === "string" ? record.title.trim() : "";
    const url = typeof record.url === "string" ? record.url.trim() : "";
    const icon = typeof record.icon === "string" ? record.icon.trim() : "";
    if (!title || !validLink(url))
      return `Link ${index + 1} needs a title and a valid destination.`;
    if (icon && !validImage(icon)) return `Link ${index + 1} has an invalid icon URL.`;
  }
  return null;
}

function validateWidget(
  item: EditableWidget,
  descriptor: WidgetDescriptor,
  nodeIds: Set<string>,
): string[] {
  const issues: string[] = [];
  for (const field of descriptor.fields) {
    const value = item.config[field.key] ?? field.default;
    if (field.required && empty(value)) {
      issues.push(`${field.label} is required.`);
      continue;
    }
    if (empty(value)) continue;

    if (field.kind === "url" && (typeof value !== "string" || !validHttpUrl(value.trim()))) {
      issues.push(`${field.label} must be a valid http(s) URL.`);
    } else if (field.kind === "links") {
      const issue = validateQuickLinks(value);
      if (issue) issues.push(issue);
    } else if (field.kind === "number") {
      const number = typeof value === "number" ? value : Number(value);
      if (!Number.isFinite(number)) {
        issues.push(`${field.label} must be a number.`);
      } else if (
        (field.min !== undefined && number < field.min) ||
        (field.max !== undefined && number > field.max)
      ) {
        issues.push(`${field.label} is outside its supported range.`);
      }
    } else if (
      field.kind === "select" &&
      !(field.options ?? []).some((option) => option.value === String(value))
    ) {
      issues.push(`${field.label} must be one of the available options.`);
    } else if (field.kind === "node" && typeof value === "string" && value && !nodeIds.has(value)) {
      issues.push(`${field.label} must refer to a configured server.`);
    }
  }
  return issues;
}

export function validateDashboardItems(
  items: EditableDashboardItem[],
  widgets: WidgetDescriptor[],
  nodes: Array<{ id: string }>,
  groups: EditableDashboardGroup[],
): ValidationIssue[] {
  const nodeIds = new Set(nodes.map(({ id }) => id));
  const issues: ValidationIssue[] = [];

  for (const item of items) {
    const add = (message: string) =>
      issues.push({ id: item.editor_id, title: item.title, message });
    if (!item.title.trim() || item.title.trim().length > 100) {
      add("Title must contain between 1 and 100 characters.");
    }
    if (item.group && (item.group.trim() !== item.group || item.group.length > 64)) {
      add("Group names must contain 1 to 64 characters.");
    }
    const min = minSizeFor(item, widgets, groupSpanOf(groups, item.group));
    if (item.width < min.width || item.height < min.height) {
      add(`Tiles must be at least ${min.width}×${min.height} grid units.`);
    }

    if (item.kind === "service") {
      if (item.href.trim() && !validLink(item.href.trim())) {
        add("Destination must be a relative path or http(s) URL.");
      }
      if (item.icon.trim() && !validImage(item.icon.trim())) {
        add("Icon must be an http(s) URL or an uploaded icon.");
      }
      if (item.ping_url.trim() && !validHttpUrl(item.ping_url.trim())) {
        add("Availability check must be a valid http(s) URL.");
      }
      continue;
    }

    const descriptor = widgets.find(({ name }) => name === item.name);
    if (!descriptor) {
      if (item.name !== "SystemStats") add("This widget type is no longer available.");
      continue;
    }
    for (const message of validateWidget(item, descriptor, nodeIds)) add(message);
  }

  return issues;
}

export function validateDashboardGroups(groups: EditableDashboardGroup[]): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  const titles = new Set<string>();
  for (const group of groups) {
    // The id-less entry is the implicit Ungrouped section's order/width slot.
    if (group.id === "") continue;
    const title = group.title.trim();
    const normalized = title.toLocaleLowerCase();
    if (!title || title.length > 64 || titles.has(normalized)) {
      issues.push({
        id: group.id,
        title,
        message: "Group names must be unique and contain 1 to 64 characters.",
      });
    } else {
      titles.add(normalized);
    }
    if (group.description.length > 240) {
      issues.push({
        id: group.id,
        title,
        message: "Group descriptions must be 240 characters or fewer.",
      });
    }
    if (group.icon.trim() && !validImage(group.icon.trim())) {
      issues.push({
        id: group.id,
        title,
        message: "Group icons must be an http(s) URL or an uploaded icon path.",
      });
    }
  }
  return issues;
}
