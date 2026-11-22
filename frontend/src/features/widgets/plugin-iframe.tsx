import { useEffect, useMemo, useRef, useState } from "react";
import * as v from "valibot";
import {
  buildPluginAssetUrl,
  PluginWidgetPayloadSchema,
  parsePluginWidgetName,
  type WidgetData,
} from "@/features/widgets/widget-schemas";

function PluginIframe({
  pluginId,
  entry,
  title,
  config,
}: {
  pluginId: string;
  entry: string;
  title: string;
  config: WidgetData;
}) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const src = buildPluginAssetUrl(pluginId, entry);
  const [loadedState, setLoadedState] = useState<{ src: string; count: number } | null>(null);
  const configKey = useMemo(() => JSON.stringify(config ?? {}), [config]);

  useEffect(() => {
    if (!loadedState || loadedState.src !== src) return;
    frameRef.current?.contentWindow?.postMessage(
      { type: "vesta:config", title, config: JSON.parse(configKey) },
      "*",
    );
  }, [loadedState, src, configKey, title]);

  return (
    <iframe
      key={src}
      className="plugin-widget-frame"
      ref={frameRef}
      src={src}
      title={`${title} plugin widget`}
      sandbox="allow-scripts allow-forms"
      onLoad={() =>
        setLoadedState((current) => ({
          src,
          count: current?.src === src ? current.count + 1 : 1,
        }))
      }
    />
  );
}

export function PluginWidgetPreview({ name, title }: { name: string; title?: string }) {
  const parsed = parsePluginWidgetName(name);
  return (
    <div className="grid h-full place-content-center gap-1 text-center text-xs text-muted-foreground">
      <span className="font-medium text-foreground">{title ?? parsed?.widgetId ?? name}</span>
      {parsed && (
        <span>
          Plugin {parsed.pluginId} · {parsed.widgetId}
        </span>
      )}
    </div>
  );
}

export function PluginWidgetContent({
  name,
  data,
  title,
}: {
  name: string;
  data: WidgetData;
  title?: string;
}) {
  const parsedName = parsePluginWidgetName(name);
  const parsedPayload = v.safeParse(PluginWidgetPayloadSchema, data);

  if (!parsedName || !parsedPayload.success) {
    return (
      <div className="widget-error" role="alert">
        This plugin widget has no saved configuration yet. Add it to the dashboard to render.
      </div>
    );
  }

  return (
    <PluginIframe
      pluginId={parsedName.pluginId}
      entry={parsedPayload.output.entry}
      title={title ?? parsedName.widgetId}
      config={parsedPayload.output.config}
    />
  );
}
