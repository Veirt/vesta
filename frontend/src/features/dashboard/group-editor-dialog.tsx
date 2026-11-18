import { IconArrowDown, IconArrowUp, IconTrash } from "@tabler/icons-react";
import type { FormEvent } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Field } from "@/shared/components/form-field";
import { IconField } from "@/shared/components/icon-field";
import {
  buttonClass,
  destructiveButtonClass,
  inputClass,
  primaryButtonClass,
} from "@/shared/components/styles";
import type { EditableDashboardGroup } from "@/shared/lib/draft-models";

const groupWidthOptions = [
  { value: 3, label: "Narrow" },
  { value: 4, label: "Small" },
  { value: 6, label: "Half" },
  { value: 8, label: "Wide" },
  { value: 12, label: "Full width" },
];

export function GroupEditorDialog({
  group,
  itemCount,
  uploading,
  onChange,
  onMove,
  onWidthChange,
  onRemove,
  onUpload,
  onClose,
}: {
  group: EditableDashboardGroup;
  itemCount: number;
  uploading: boolean;
  onChange: (update: Partial<EditableDashboardGroup>) => void;
  onMove: (direction: -1 | 1) => void;
  onWidthChange: (width: number) => void;
  onRemove: () => void;
  onUpload: (file: File) => Promise<string | null>;
  onClose: () => void;
}) {
  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onClose();
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="dashboard-editor-dialog sm:max-w-2xl">
        <form className="grid gap-5" onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Edit section</DialogTitle>
            <DialogDescription>
              Sections organize independent service and widget tiles. Changes stay in this draft
              until you save the dashboard.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field
              label="Section name"
              description="Shown as the heading above the section's tiles."
              required
            >
              <Input
                value={group.title}
                required
                maxLength={64}
                onChange={(event) => onChange({ title: event.target.value })}
                className={inputClass}
              />
            </Field>
            <Field label="Description" description="A short hint about what belongs here.">
              <textarea
                value={group.description}
                maxLength={240}
                rows={2}
                onChange={(event) => onChange({ description: event.target.value })}
                className="w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
            </Field>
            <IconField
              className="sm:col-span-2"
              value={group.icon}
              uploading={uploading}
              onChange={(icon) => onChange({ icon })}
              onUpload={onUpload}
            />
            <Field
              label="Width"
              description="How wide the section is in the dashboard grid. You can also drag its edge handle on the canvas."
            >
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={group.width}
                onChange={(event) => onWidthChange(Number(event.target.value))}
              >
                {!groupWidthOptions.some((option) => option.value === group.width) && (
                  <option value={group.width}>Custom ({group.width} of 12)</option>
                )}
                {groupWidthOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </Field>
          </div>
          <p className="text-xs text-muted-foreground">
            {itemCount === 1 ? "1 tile is" : `${itemCount} tiles are`} assigned to this section.
          </p>
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
            <div className="flex items-center gap-2">
              <button
                className={buttonClass}
                type="button"
                aria-label={`Move ${group.title} earlier`}
                onClick={() => onMove(-1)}
              >
                <IconArrowUp size={16} aria-hidden="true" /> Move earlier
              </button>
              <button
                className={buttonClass}
                type="button"
                aria-label={`Move ${group.title} later`}
                onClick={() => onMove(1)}
              >
                <IconArrowDown size={16} aria-hidden="true" /> Move later
              </button>
            </div>
            <div className="flex items-center gap-2">
              <button className={destructiveButtonClass} type="button" onClick={onRemove}>
                <IconTrash size={16} aria-hidden="true" /> Remove section
              </button>
              <button className={primaryButtonClass} type="submit">
                Done
              </button>
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            Removing a section keeps its tiles and moves them to Ungrouped.
          </p>
        </form>
      </DialogContent>
    </Dialog>
  );
}
