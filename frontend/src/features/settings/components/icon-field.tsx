import { useId, type ChangeEvent } from "react";
import { Input } from "@/components/ui/input";
import { inputClass } from "@/features/settings/components/styles";

function IconField({
  value,
  uploading,
  onChange,
  onUpload,
}: {
  value: string;
  uploading: boolean;
  onChange: (value: string) => void;
  onUpload: (file: File) => Promise<string | null>;
}) {
  const id = useId();

  async function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
    const input = event.currentTarget;
    const file = input.files?.[0];
    if (!file) return;
    const url = await onUpload(file);
    if (url) onChange(url);
    input.value = "";
  }

  return (
    <div className="grid min-w-0 gap-2">
      <p className="text-sm font-medium">Icon URL or upload</p>
      <div className="grid gap-2">
        <div className="flex min-w-0 items-center gap-2">
          {value && (
            <img
              key={value}
              className="size-9 shrink-0 rounded-md border border-border bg-muted object-contain p-1"
              src={value}
              alt=""
              onError={(event) => {
                event.currentTarget.hidden = true;
              }}
            />
          )}
          <Input
            id={`${id}-url`}
            value={value}
            inputMode="url"
            aria-label="Icon URL or uploaded icon path"
            placeholder="https://… or /api/icons/…"
            onChange={(event) => onChange(event.target.value)}
            className={inputClass}
          />
        </div>
        <Input
          id={`${id}-file`}
          type="file"
          accept="image/png,image/jpeg,image/webp,image/gif"
          aria-label="Upload an icon image"
          disabled={uploading}
          onChange={(event) => void handleFileChange(event)}
          className={`${inputClass} h-auto py-1.5 text-xs file:mr-3 file:rounded file:border-0 file:bg-muted file:px-2 file:py-1 file:text-xs file:font-medium file:text-foreground`}
        />
        <p className="text-xs leading-5 text-muted-foreground">
          PNG, JPEG, WebP, or GIF up to 2 MiB. The image is stored locally; its URL is saved in
          config.toml.
        </p>
        {uploading && (
          <p className="text-xs text-muted-foreground" role="status">
            Uploading image…
          </p>
        )}
      </div>
    </div>
  );
}

export { IconField };
