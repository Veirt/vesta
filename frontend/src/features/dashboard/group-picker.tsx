import { IconChevronDown } from "@tabler/icons-react";
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { buttonClass } from "@/shared/components/styles";
import type { EditableDashboardGroup } from "@/shared/lib/draft-models";

export function GroupPicker({
  value,
  groups,
  onChange,
  onCreate,
}: {
  value: string | null;
  groups: EditableDashboardGroup[];
  onChange: (group: string | null) => void;
  onCreate: (title: string) => string;
}) {
  const [open, setOpen] = useState(false);
  const [newGroup, setNewGroup] = useState("");

  function choose(group: string | null) {
    onChange(group);
    setOpen(false);
    setNewGroup("");
  }

  function createGroup() {
    const title = newGroup.trim();
    if (title && title.length <= 64) choose(onCreate(title));
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          className="inline-flex h-9 w-full items-center justify-between gap-2 rounded-md border border-input bg-background px-3 text-left text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
          type="button"
          aria-label="Choose a dashboard group"
        >
          <span className="truncate">
            {groups.find((group) => group.id === value)?.title ?? value ?? "Ungrouped"}
          </span>
          <IconChevronDown size={16} aria-hidden="true" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="grid w-72 gap-3">
        <div>
          <h4 className="text-sm font-medium">Group tiles</h4>
          <p className="mt-1 text-xs text-muted-foreground">
            Groups organize services and widgets together without nesting either type.
          </p>
        </div>
        <div className="grid max-h-40 gap-1 overflow-y-auto" role="group" aria-label="Groups">
          <button
            type="button"
            className={`rounded px-2 py-1.5 text-left text-sm hover:bg-accent ${!value ? "bg-accent" : ""}`}
            aria-pressed={!value}
            onClick={() => choose(null)}
          >
            Ungrouped
          </button>
          {groups
            .filter((group) => group.id !== "")
            .map((group) => (
              <button
                key={group.id}
                type="button"
                className={`truncate rounded px-2 py-1.5 text-left text-sm hover:bg-accent ${value === group.id ? "bg-accent" : ""}`}
                aria-pressed={value === group.id}
                onClick={() => choose(group.id)}
              >
                {group.title}
              </button>
            ))}
        </div>
        <div className="grid gap-2 border-t border-border pt-3">
          <label className="grid gap-1 text-sm font-medium" htmlFor="new-dashboard-group">
            Create a group
            <Input
              id="new-dashboard-group"
              value={newGroup}
              maxLength={64}
              placeholder="e.g. Media"
              onChange={(event) => setNewGroup(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  createGroup();
                }
              }}
              className="font-normal"
            />
          </label>
          <button
            type="button"
            className={buttonClass}
            disabled={!newGroup.trim()}
            onClick={createGroup}
          >
            Use group
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
