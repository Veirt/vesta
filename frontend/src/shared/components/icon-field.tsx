import { useId, type ChangeEvent } from "react";
import { IconPhoto, IconUpload, IconX } from "@tabler/icons-react";
import { Input } from "@/components/ui/input";
import { buttonClass, inputClass } from "@/shared/components/styles";

function IconField({
  value,
  uploading,
  onChange,
  onUpload,
  className,
}: {
  value: string;
  uploading: boolean;
  onChange: (value: string) => void;
  onUpload: (file: File) => Promise<string | null>;
  className?: string;
}) {
  const id = useId();
  const uploadedIcon = value.startsWith("/api/icons/");

  async function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
    const input = event.currentTarget;
    const file = input.files?.[0];
    if (!file) return;
    const url = await onUpload(file);
    if (url) onChange(url);
    input.value = "";
  }

  return (
    <div className={`grid min-w-0 gap-2 ${className ?? ""}`}>
      <p className="text-sm font-medium">Icon</p>
      <div className="grid min-w-0 gap-3 rounded-lg border border-border/70 bg-muted/20 p-3 sm:grid-cols-[minmax(11rem,0.7fr)_minmax(0,1.3fr)] sm:items-center">
        <div className="flex min-w-0 items-center gap-3">
          <span className="relative grid size-12 shrink-0 place-items-center overflow-hidden rounded-lg border border-border bg-background text-muted-foreground">
            <IconPhoto size={20} stroke={1.7} aria-hidden="true" />
            {value && (
              <img
                key={value}
                className="absolute inset-0 size-full bg-background object-contain p-1.5"
                src={value}
                alt=""
                onError={(event) => {
                  event.currentTarget.hidden = true;
                }}
              />
            )}
          </span>
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">
              {uploadedIcon ? "Uploaded icon" : value ? "Remote icon" : "No custom icon"}
            </p>
            <p className="text-xs leading-5 text-muted-foreground">
              {uploadedIcon
                ? "Stored locally in this Vesta instance"
                : value
                  ? "Preview shown on the dashboard tile"
                  : "The tile will use its letter fallback"}
            </p>
          </div>
        </div>
        <div className="grid min-w-0 gap-2">
          <Input
            id={`${id}-url`}
            value={uploadedIcon ? "" : value}
            inputMode="url"
            aria-label="Icon URL"
            placeholder="https://example.com/icon.png"
            onChange={(event) => onChange(event.target.value)}
            className={inputClass}
          />
          <div className="flex flex-wrap items-center gap-2">
            <Input
              id={`${id}-file`}
              type="file"
              accept="image/png,image/jpeg,image/webp,image/gif"
              disabled={uploading}
              onChange={(event) => void handleFileChange(event)}
              className="screen-reader-only peer"
            />
            <label
              htmlFor={`${id}-file`}
              className={`${buttonClass} min-h-8 cursor-pointer px-2.5 text-xs peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-ring peer-disabled:pointer-events-none peer-disabled:opacity-50`}
            >
              <IconUpload size={15} aria-hidden="true" />
              {uploading ? "Uploading…" : "Upload image"}
            </label>
            {uploadedIcon && (
              <button
                className={`${buttonClass} min-h-8 px-2.5 text-xs`}
                type="button"
                aria-label="Remove uploaded icon"
                onClick={() => onChange("")}
              >
                <IconX size={15} aria-hidden="true" /> Remove
              </button>
            )}
            <span className="text-xs text-muted-foreground">PNG, JPEG, WebP, GIF · max 2 MiB</span>
          </div>
          {uploading && (
            <p className="text-xs text-muted-foreground" role="status">
              Uploading image…
            </p>
          )}
        </div>
      </div>
      <p className="text-xs leading-5 text-muted-foreground">
        Use a remote URL or upload an image. Uploaded files stay on this server; config.toml stores
        their local URL.
      </p>
    </div>
  );
}

export { IconField };
