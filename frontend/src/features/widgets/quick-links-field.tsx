import { IconPlus, IconTrash } from "@tabler/icons-react";
import { useRef, useState } from "react";
import { Input } from "@/components/ui/input";
import { Field } from "@/shared/components/form-field";
import { IconField } from "@/shared/components/icon-field";
import { buttonClass, destructiveButtonClass, inputClass } from "@/shared/components/styles";

type QuickLink = { title: string; url: string; icon: string };

function parseLinks(value: unknown): QuickLink[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item): QuickLink[] => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const record = item as Record<string, unknown>;
    return [
      {
        title: typeof record.title === "string" ? record.title : "",
        url: typeof record.url === "string" ? record.url : "",
        icon: typeof record.icon === "string" ? record.icon : "",
      },
    ];
  });
}

export function QuickLinksField({
  value,
  uploading,
  onChange,
  onUpload,
}: {
  value: unknown;
  uploading: boolean;
  onChange: (links: QuickLink[]) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  const links = parseLinks(value);
  const nextIdRef = useRef(links.length + 1);
  const [rowIds, setRowIds] = useState<Array<string>>(() =>
    links.map((_, index) => `link-row-${index + 1}`),
  );

  function addLink() {
    const id = `link-row-${nextIdRef.current}`;
    nextIdRef.current += 1;
    setRowIds((current) => [...current, id]);
    onChange([...links, { title: "", url: "", icon: "" }]);
  }

  function removeLink(index: number) {
    setRowIds((current) => current.filter((_, itemIndex) => itemIndex !== index));
    onChange(links.filter((_, itemIndex) => itemIndex !== index));
  }

  const rows = links.map((link, index) => ({
    link,
    index,
    rowKey: rowIds[index] ?? link.url,
  }));

  return (
    <fieldset className="grid gap-3 rounded-md border border-border/70 p-3 sm:p-4">
      <legend className="px-1 text-sm font-medium">Links</legend>
      {links.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No links yet. Add one with a title and destination.
        </p>
      )}
      {rows.map(({ link, index, rowKey }) => (
        <article
          className="grid gap-3 rounded-md border border-border/70 bg-background/50 p-3 sm:grid-cols-2"
          key={rowKey}
        >
          <Field label={`Link ${index + 1} title`} required>
            <Input
              value={link.title}
              required
              onChange={(event) =>
                onChange(
                  links.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, title: event.target.value } : item,
                  ),
                )
              }
              className={inputClass}
            />
          </Field>
          <Field label={`Link ${index + 1} destination`} required>
            <Input
              value={link.url}
              required
              inputMode="url"
              placeholder="https://…"
              onChange={(event) =>
                onChange(
                  links.map((item, itemIndex) =>
                    itemIndex === index ? { ...item, url: event.target.value } : item,
                  ),
                )
              }
              className={inputClass}
            />
          </Field>
          <IconField
            value={link.icon}
            uploading={uploading}
            onChange={(icon) =>
              onChange(
                links.map((item, itemIndex) => (itemIndex === index ? { ...item, icon } : item)),
              )
            }
            onUpload={onUpload}
          />
          <div className="flex items-end">
            <button
              className={destructiveButtonClass}
              type="button"
              onClick={() => removeLink(index)}
              aria-label={`Remove link ${link.title || index + 1}`}
            >
              <IconTrash size={16} aria-hidden="true" /> Remove link
            </button>
          </div>
        </article>
      ))}
      <button className={`${buttonClass} justify-self-start`} type="button" onClick={addLink}>
        <IconPlus size={16} aria-hidden="true" /> Add link
      </button>
    </fieldset>
  );
}
